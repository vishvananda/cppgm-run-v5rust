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

sub optimize
{
	my ($app, $test, $directory, $level) = @_;
	my $output = "$directory/$level.lowir";
	my $status = run_command_capture(
		cmd => [$app, "-$level", '-o', $output, $test],
		stdout => "$directory/$level.stdout",
		stderr => "$directory/$level.stderr",
		timeout => 30,
	);
	die "$test: lowiropt -$level failed\n" .
		read_file("$directory/$level.stderr") if $status != 0;
	return ($output, read_file($output));
}

sub functions
{
	my ($test, $text) = @_;
	my %result;
	while($text =~ /(function\s+\@([^\s(]+).*?\n\})
	                 (?=\nfunction\s+\@|\s*\z)/sgx) {
		die "$test: duplicate optimized function symbol $2\n"
			if exists($result{$2});
		$result{$2} = $1;
	}
	return \%result;
}

sub call_targets
{
	my ($body) = @_;
	return $body =~ /^\s+call\s+void\s+\@([^\s(]+)\(/mg;
}

sub count_calls
{
	my ($body, $target) = @_;
	return scalar(() = $body =~
		/^\s+call\s+void\s+\@\Q$target\E\(/mg);
}

sub parameter_names
{
	my ($body) = @_;
	my ($parameters) = $body =~ /^function\s+\@[^\s(]+\((.*?)\)\s*->/s;
	return () if !defined($parameters);
	return $parameters =~ /(%[A-Za-z0-9_]+)\s*:/g;
}

sub blocks
{
	my ($body) = @_;
	my @result;
	while($body =~ /^\s*block\s+\^([^:]+):\n(.*?)(?=^\s*block\s+\^|^\}|\z)/msg) {
		my ($label, $text) = ($1, $2);
		my ($terminator) = $text =~ /^\s+((?:jump|branch|switch)\b.*)\z/ms;
		my @successors = defined($terminator) ? $terminator =~ /\^(\w+)/g : ();
		push(@result, { label => $label, text => $text,
			successors => \@successors });
	}
	return @result;
}

sub compile_and_run
{
	my ($driver, $test, $directory, $level, $lowir) = @_;
	my $program = "$directory/behavior-$level";
	my $status = run_command_capture(
		cmd => [$driver, '-O0', '-o', $program, $lowir],
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
	die "$test: $level behavior failed with status $status\n"
		if $status != 0;
}

if(scalar(@ARGV) != 3)
{
	die "Usage: check_lowir_fast_path_versioning.pl " .
		"<lowiropt> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/o3-fast-path-versioning\.t$/);
die "No O3 fast-path-versioning controls found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('lowir-fast-path-versioning-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my (%paths, %outputs, %records);
	for my $level (qw(O0 O1 O2 O3)) {
		($paths{$level}, $outputs{$level}) =
			optimize($app, $test, $directory, $level);
		$records{$level} = functions($test, $outputs{$level});
		compile_and_run($driver, $test, $directory, $level, $paths{$level});
	}
	my $complete_arms = count_calls($records{O0}->{versionable} //
		die("$test: O0 lost the versionable reducer\n"), 'observe');
	my $guard_arms = count_calls($records{O0}->{effect_before_bailout} //
		die("$test: O0 lost the effect-before-bailout guard reducer\n"),
		'observe');

	my @runners = qw(run_versionable run_versionable_again);
	for my $level (qw(O0 O1)) {
		for my $runner (@runners) {
			my $body = $records{$level}->{$runner} //
				die "$test: $level lost the $runner caller\n";
			my @calls = call_targets($body);
			die "$test: $level unexpectedly versioned the fast path in $runner\n"
				if scalar(@calls) != 1 || $calls[0] ne 'versionable' ||
				   $body =~ /^\s+(?:branch|switch)\b/m;
		}
	}

	# From O2 a call site takes a copy of the callee's fast path whose every
	# exit ahead of a visible effect calls the complete callee with the same
	# arguments and returns: the slow arms run only in that callee.
	for my $level (qw(O2 O3)) {
		my $complete = $records{$level}->{versionable} //
			die "$test: $level lost the complete versionable body\n";
		die "$test: $level complete body did not retain all four observable arms\n"
			if count_calls($complete, 'observe') != $complete_arms;
		for my $runner (@runners) {
			my $body = $records{$level}->{$runner} //
				die "$test: $level lost the $runner caller\n";
			die "$test: $level fast copy in $runner retained an observable " .
				"slow-path call\n" if count_calls($body, 'observe') != 0;
			my @calls = call_targets($body);
			die "$test: $level did not copy the fast path into $runner\n"
				if !@calls || scalar(grep { $_ ne 'versionable' } @calls) ||
				   $body !~ /^\s+(?:branch|switch)\b/m;
			my $parameters = join(' ', parameter_names($body));
			my @blocks = blocks($body);
			my %return_only = map { $_->{label} => 1 }
				grep { $_->{text} =~ /\A\s+return void\s*\z/ } @blocks;
			my %transfer;
			for my $block (@blocks) {
				next if $block->{text} !~ /^\s+call\s+void\s+\@versionable\(([^)]*)\)$/m;
				my @arguments = $1 =~ /(%[A-Za-z0-9_]+)/g;
				die "$test: $level slow transfer in $runner did not preserve " .
					"the parameter tuple\n"
					if join(' ', @arguments) ne $parameters;
				my $exits_now = $block->{text} =~
					/\A\s+call\s+void\s+\@versionable\([^)]*\)\n\s+return void\s*\z/;
				my $exits_after = $block->{text} =~
					/\A\s+call\s+void\s+\@versionable\([^)]*\)\n\s+jump \^(\w+)\s*\z/ &&
					$return_only{$1};
				die "$test: $level slow transfer in $runner is not a call/return block\n"
					if !$exits_now && !$exits_after;
				$transfer{$block->{label}} = 1;
			}
			my $bailouts = grep { $transfer{$_} }
				map { @{$_->{successors}} } @blocks;
			die "$test: $level $runner did not route at least three " .
				"pre-effect bailouts to the complete body\n" if $bailouts < 3;
			my %block_of = map { $_->{label} => $_ } @blocks;
			my @work = ($blocks[0]->{label});
			my (%seen, $fast);
			while (@work && !$fast) {
				my $label = pop(@work);
				next if $seen{$label}++ || $transfer{$label};
				my $block = $block_of{$label} // next;
				$fast = $block->{text} =~ /^\s+return\b/m;
				push(@work, @{$block->{successors}});
			}
			die "$test: $level $runner has no fast path to a return " .
				"around the complete body\n" if !$fast;
		}
	}

	# A path that stores before its bailouts cannot restart in the callee:
	# no level copies any of it into its callers.
	for my $level (qw(O1 O2 O3)) {
		my $guard = $records{$level}->{effect_before_bailout} //
			die "$test: $level lost the effect-before-bailout guard reducer\n";
		die "$test: $level lost the guard's store or observable arms\n"
			if count_calls($guard, 'observe') != $guard_arms ||
			   $guard !~ /^\s+store\s+i64\s+%\w+,\s+%\w+/m;
		for my $runner (qw(run_effect run_effect_again)) {
			my $body = $records{$level}->{$runner} //
				die "$test: $level lost the $runner caller\n";
			my @calls = call_targets($body);
			die "$test: $level versioned a path after an externally " .
				"visible store in $runner\n"
				if scalar(@calls) != 1 || $calls[0] ne 'effect_before_bailout' ||
				   $body =~ /^\s+(?:store|branch|switch)\b/m;
		}
	}
}

print "fast-path versioning: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
