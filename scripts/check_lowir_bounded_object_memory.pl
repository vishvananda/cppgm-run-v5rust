#!/usr/bin/perl

use strict;
use warnings;

use File::Temp qw(tempdir);
use FindBin;
use lib $FindBin::Bin;

use CppgmBatchWorker qw(collect_tests run_command_capture);

sub read_file
{
	my ($path) = @_;
	open(my $fh, '<', $path) or die "Unable to read $path: $!\n";
	local $/;
	my $data = <$fh>;
	close($fh) or die "Unable to close $path: $!\n";
	return defined($data) ? $data : '';
}

sub run_optimizer
{
	my ($app, $test, $directory, $level) = @_;
	my $output = "$directory/$level.lowir";
	my $stderr = "$directory/$level.stderr";
	my $status = run_command_capture(
		cmd => [$app, "-$level", '-o', $output, $test],
		stdout => "$directory/$level.stdout",
		stderr => $stderr,
		timeout => 30,
	);
	die "$test: lowiropt -$level failed\n" . read_file($stderr)
		if $status != 0;
	return (read_file($output), $output);
}

sub functions
{
	my ($test, $text) = @_;
	my %result;
	while($text =~ /(function\s+\@([^\s(]+).*?\n\})
	                 (?=\nfunction\s+\@|\z)/sgx) {
		die "$test: duplicate optimized function symbol $2\n"
			if exists($result{$2});
		$result{$2} = $1;
	}
	die "$test: optimized LowIR contained no functions\n" if !%result;
	return \%result;
}

# Block label => [instruction lines], plus the entry label.
sub blocks
{
	my ($body) = @_;
	my (%blocks, $entry, $current);
	for my $line (split(/\n/, $body)) {
		if($line =~ /^\s*block \^([^:\s]+):\s*$/) {
			$current = $1;
			$entry = $current if !defined($entry);
			$blocks{$current} = [];
		} elsif(defined($current) && $line =~ /\S/ && $line !~ /^\}/) {
			push(@{$blocks{$current}}, $line);
		}
	}
	return (\%blocks, $entry);
}

sub successors
{
	my ($lines) = @_;
	return () if !@$lines;
	my $last = $lines->[-1];
	return ($1) if $last =~ /^\s+jump \^(\S+)\s*$/;
	return ($1, $2) if $last =~ /^\s+branch %\S+, \^([^,\s]+), \^(\S+)\s*$/;
	return ();
}

sub has_reachable_cycle
{
	my ($body) = @_;
	my ($blocks, $entry) = blocks($body);
	return 0 if !defined($entry);
	my %state;
	my $visit;
	$visit = sub {
		my ($label) = @_;
		return 0 if !exists($blocks->{$label});
		return 1 if ($state{$label} // 0) == 1;
		return 0 if ($state{$label} // 0) == 2;
		$state{$label} = 1;
		for my $next (successors($blocks->{$label})) {
			return 1 if $visit->($next);
		}
		$state{$label} = 2;
		return 0;
	};
	return $visit->($entry);
}

sub load_count
{
	my ($body) = @_;
	my @loads = $body =~ /^\s+%[A-Za-z0-9_]+ = load i64 /mg;
	return scalar(@loads);
}

sub compile_and_run
{
	my ($driver, $test, $directory, $level, $lowir) = @_;
	my $program = "$directory/behavior-$level";
	my $status = run_command_capture(
		cmd => [$driver, "-$level", '-o', $program, $lowir],
		stdout => "$directory/compile-$level.stdout",
		stderr => "$directory/compile-$level.stderr",
		timeout => 60,
	);
	die "$test: $level optimized LowIR did not compile\n" .
		read_file("$directory/compile-$level.stderr") if $status != 0;
	$status = run_command_capture(
		cmd => [$program],
		stdout => "$directory/run-$level.stdout",
		stderr => "$directory/run-$level.stderr",
		timeout => 30,
	);
	die "$test: $level optimized behavior failed with status $status\n"
		if $status != 0;
}

if(scalar(@ARGV) != 3)
{
	die "Usage: check_lowir_bounded_object_memory.pl " .
		"<lowiropt> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/o3-bounded-object-memory.*\.t$/);
die "No O3 bounded-object-memory controls found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('cppgm-bounded-object-memory-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my (%outputs, %paths, %records);
	for my $level ('O0', 'O1', 'O2', 'O3') {
		($outputs{$level}, $paths{$level}) =
			run_optimizer($app, $test, $directory, $level);
		$records{$level} = functions($test, $outputs{$level});
	}

	# Select the cases by structure in the unoptimized output: the direct
	# write barriers (a zeroing and an atomic store between reads of one
	# field), the by-address scalar reader and its equality-edge caller,
	# and the gated loop whose literal store sits on its backedge.
	my ($writes, $loop, $gate, $address_target, $address_caller);
	for my $name (sort(keys(%{$records{O0}})))
	{
		my $body = $records{O0}{$name};
		if($body =~ /^\s+zeroinit /m && $body =~ /^\s+atomic_store /m) {
			$writes = $name;
			next;
		}
		my ($parameter) = $body =~
			/^function\s+\@[^\s(]+\(%([A-Za-z0-9_]+)\s*:\s*ptr\s*
			 \[[^\]]*pass=by_address[^\]]*\]/x;
		if(defined($parameter) &&
		   $body =~ /^\s+%[A-Za-z0-9_]+\s+=\s+load\s+i64\s+%\Q$parameter\E$/m)
		{
			$address_target = $name;
			next;
		}
		my ($equal) = $body =~
			/^\s+%[A-Za-z0-9_]+\s+=\s+cmp\s+eq\s+i64\s+%([A-Za-z0-9_]+),\s*0$/m;
		if(defined($equal) && has_reachable_cycle($body) &&
		   $body =~ /^\s+store\s+i(?:8|16|32|64)\s+[0-9]+,\s+%\S+\n
		             \s+jump\s+\^\S+$/mx &&
		   $body =~ /^\s+%[A-Za-z0-9_]+\s+=\s+binary\s+add\s+i64
		             \s+%\Q$equal\E,\s*[0-9]+$/mx)
		{
			$loop = $name;
			$gate = $equal;
		}
	}
	die "$test: control has no structurally identifiable by-address scalar reader\n"
		if !defined($address_target);
	for my $name (sort(keys(%{$records{O0}})))
	{
		my $body = $records{O0}{$name};
		my ($value) = $body =~
			/^\s+%([A-Za-z0-9_]+)\s+=\s+load\s+i64\s+/m;
		if(defined($value) &&
		   $body =~ /^\s+%[A-Za-z0-9_]+\s+=\s+cmp\s+eq\s+i64\s+%\Q$value\E,\s*0$/m &&
		   $body =~ /^\s+%[A-Za-z0-9_]+\s+=\s+call\s+i64\s+
		             \@\Q$address_target\E\(%\Q$value\E\)$/mx)
		{
			$address_caller = $name;
			last;
		}
	}
	die "$test: control has no equality-edge by-address call relationship\n"
		if !defined($address_caller);
	die "$test: control has no structurally identifiable writes case\n"
		if !defined($writes);
	die "$test: control has no structurally identifiable gated loop case\n"
		if !defined($loop);
	die "$test: O0 changed the gated loop baseline\n"
		if load_count($records{O0}{$loop}) != 4;

	for my $level ('O0', 'O1', 'O2', 'O3') {
		# A zeroing or atomic store to the field ends the earlier read's
		# validity: the read after the atomic store stays a load, and
		# the read before the writes is not reused for a later one.
		my $body = $records{$level}{$writes} //
			die "$test: $level lost the direct-write case\n";
		die "$test: $level removed a direct write barrier\n"
			if $body !~ /^\s+zeroinit /m || $body !~ /^\s+atomic_store /m;
		die "$test: $level did not reload the field after its atomic store\n"
			if $body !~ /^\s+atomic_store [^\n]*\n
			             (?:[^\n]*\n)*?\s+%[A-Za-z0-9_]+\s+=\s+load\s+i64\s/mx;
		my ($before) = $body =~ /^\s+%([A-Za-z0-9_]+)\s+=\s+load\s+i64\s/m;
		die "$test: $level reused the pre-write load across a write\n"
			if !defined($before) ||
			   scalar(() = $body =~ /%\Q$before\E\b/g) != 2;
	}

	for my $level ('O1', 'O2', 'O3') {
		my $body = $records{$level}{$loop} //
			die "$test: $level lost the gated loop case\n";
		# The branch edge on which the gate equals zero substitutes that
		# value into the arithmetic it dominates: only the comparison
		# still reads the gate parameter.
		my @gate_uses = $body =~ /^[ \t]+[^\n]*%\Q$gate\E\b[^\n]*$/mg;
		die "$test: $level did not propagate the branch-edge equality\n"
			if scalar(@gate_uses) != 1 ||
			   $gate_uses[0] !~ /=\s+cmp\s+eq\s+i64\s+%\Q$gate\E,\s*0\s*$/;
		# The literal store on the backedge forwards to the header's
		# reload, so its edge is threaded to the exit: the loop is gone
		# but the store stays.
		die "$test: $level dropped the loop's literal store\n"
			if $body !~ /^\s+store\s+i(?:8|16|32|64)\s+[0-9]+,\s+%\S+$/m;
		die "$test: $level did not thread the literal-store backedge\n"
			if has_reachable_cycle($body);
	}

	# Equality propagation may replace value uses, but a by-address call
	# needs the address of storage holding the scalar: its argument stays
	# the scalar temporary rather than a literal of the same value, and
	# the callee keeps the parameter it reads through.
	for my $level ('O1', 'O2', 'O3') {
		my ($address_argument) = $records{$level}{$address_caller} =~
			/^\s+%[A-Za-z0-9_]+\s+=\s+call\s+i64\s+
			  \@\Q$address_target\E\(([^)]*)\)$/mx;
		die "$test: $level equality propagation made a by-address call " .
			"non-addressable\n"
			if !defined($address_argument) ||
			   $address_argument !~ /^%[A-Za-z0-9_]+$/;
		my $target = $records{$level}{$address_target} //
			die "$test: $level lost the by-address scalar reader\n";
		die "$test: $level removed the by-address parameter\n"
			if $target !~ /^function\s+\@[^\s(]+\(%[A-Za-z0-9_]+\s*:\s*ptr\s*
			               \[[^\]]*pass=by_address/x;
	}

	compile_and_run($driver, $test, $directory, 'O0', $paths{O0});
	compile_and_run($driver, $test, $directory, 'O3', $paths{O3});
}

print "O3 bounded object memory: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
