#!/usr/bin/perl

use strict;
use warnings;

use File::Copy qw(copy);
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

sub lowir_functions
{
	my ($test, $text) = @_;
	my %result;
	while($text =~ /(function\s+\@([^\s(]+).*?\n\})\s*
	                 (?=^function\s+\@|\z)/sgmx) {
		die "$test: duplicate LowIR function symbol $2\n"
			if exists($result{$2});
		$result{$2} = $1;
	}
	return \%result;
}

sub mir_function
{
	my ($test, $text, $symbol) = @_;
	return $1 if $text =~
		/(^function \@\Q$symbol\E\b.*?)(?=^function \@|\z)/ms;
	die "$test: machine IR has no structurally selected function\n";
}

sub parameter_extent
{
	my ($body) = @_;
	return $1 if $body =~
		/^function\s+\@[^\s(]+\(\s*%[A-Za-z0-9_]+\s*:\s*ptr
		 \s*\[[^\]]*\bobject_bytes=([1-9][0-9]*)\b/x;
	return 0;
}

sub call_position
{
	my ($body) = @_;
	return index($body, ' call i64 ');
}

sub constant_field_indexes
{
	my ($body) = @_;
	my @result;
	while($body =~ /^\s+(%[A-Za-z0-9_]+)\s+=\s+index\s+i8
	               \s+\[projection=field\]\s+%[A-Za-z0-9_]+,
	               \s*([1-9][0-9]*)$/mgx) {
		push(@result, [$1, 0 + $2, $-[0]]);
	}
	return @result;
}

sub preserved_registers
{
	my ($body) = @_;
	return split(/\s+/, $1) if $body =~ /^\s+preserve\s+([^\n]+)$/m;
	return ();
}

sub stack_size
{
	my ($body) = @_;
	return 0 + $1 if $body =~ /^\s+stack_size\s+([0-9]+)$/m;
	return 0;
}

if(scalar(@ARGV) != 4)
{
	die "Usage: check_lowir_native_parameter_addresses.pl " .
		"<lowiropt> <lowir2native> <cppgm++> <test-or-directory>\n";
}

my ($optimizer, $native, $driver, $root) = @ARGV;
my @tests = collect_tests(
	$root, qr/parameter-address-folding.*\.t$/);
die "No parameter-address folding controls found under $root\n"
	if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('native-parameter-address-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my $source = read_file($test);
	my $source_functions = lowir_functions($test, $source);
	my $selected;
	my @selected_offsets;
	for my $symbol (keys(%$source_functions)) {
		my $body = $source_functions->{$symbol};
		next if !parameter_extent($body);
		my $call = call_position($body);
		next if $call < 0;
		my @indexes = constant_field_indexes($body);
		my @before = grep { $_->[2] < $call } @indexes;
		next if @before < 4;
		my %used_after;
		for my $index (@before) {
			my $value = $index->[0];
			$used_after{$index->[1]} = 1
				if substr($body, $call) =~
				   /^\s+%[A-Za-z0-9_]+ = load i64 \Q$value\E$/m;
		}
		next if scalar(keys(%used_after)) < 4;
		die "$test: multiple functions match the rematerialization relation\n"
			if defined($selected);
		$selected = $symbol;
		@selected_offsets = sort { $a <=> $b } keys(%used_after);
	}
	die "$test: no bounded parameter function defines four post-call field " .
		"addresses before the call\n" if !defined($selected);

	my (%optimized, %mir, %program);
	for my $level (qw(O2 O3)) {
		$optimized{$level} = "$directory/$level.lowir";
		my $stderr = "$directory/$level.optimizer.stderr";
		my $status = run_command_capture(
			cmd => [$optimizer, "-$level", '-o', $optimized{$level}, $test],
			stdout => "$directory/$level.optimizer.stdout",
			stderr => $stderr,
			timeout => 30,
		);
		die "$test: lowiropt -$level failed\n" . read_file($stderr)
			if $status != 0;
		$mir{$level} = "$directory/$level.mir";
		$program{$level} = "$directory/$level.program";
		$status = run_command_capture(
			cmd => [$native, "-$level", '--dump-machine-ir', $mir{$level},
				'-o', $program{$level}, $optimized{$level}],
			stdout => "$directory/$level.native.stdout",
			stderr => "$directory/$level.native.stderr",
			timeout => 30,
		);
		die "$test: $level optimized native compile failed\n" .
			read_file("$directory/$level.native.stderr") if $status != 0;
		$status = run_command_capture(
			cmd => [$program{$level}],
			stdout => "$directory/$level.run.stdout",
			stderr => "$directory/$level.run.stderr",
			timeout => 30,
		);
		die "$test: $level generated behavior failed with status $status\n"
			if $status != 0;
	}

	# The optimizer leaves the field addresses where the source computes
	# them, before the call (it has no address rematerialization pass). The
	# backend folds each base+constant field address into its consumer's
	# memory operand, at every level, so only the parameter base crosses the
	# call and each address is rebuilt from it at its load after the call;
	# O3 must not add to O2's call-preserved pressure or frame.
	my (%level_mir, %level_preserved);
	for my $level (qw(O2 O3)) {
		my $functions = lowir_functions($test, read_file($optimized{$level}));
		die "$test: $level optimization lost the selected function or object extent\n"
			if !defined($functions->{$selected}) ||
			   !parameter_extent($functions->{$selected});
		my $body = mir_function($test, read_file($mir{$level}), $selected);
		$level_mir{$level} = $body;
		my @preserved = preserved_registers($body);
		$level_preserved{$level} = scalar(@preserved);
		die "$test: native $level keeps a derived address in a call-preserved home\n"
			if @preserved > 1;
		my ($before_call, $after_call) =
			$body =~ /(.*?)^\s+call\s+\@[^\n]+\n(.*)$/ms;
		die "$test: native $level lost the selected function's call\n"
			if !defined($after_call);
		die "$test: native $level recreated derived addresses before the call\n"
			if $before_call =~ /^\s+lea\s+/m;
		my %native_offsets;
		while($after_call =~ /^\s+load\.i64\s+\w+,\s+
		                      \[\w+\+([1-9][0-9]*)\]/mgx) {
			$native_offsets{0 + $1} = 1;
		}
		for my $offset (@selected_offsets) {
			die "$test: native $level lost a post-call direct field load\n"
				if !$native_offsets{$offset};
		}
	}
	die "$test: native O3 added call-preserved address pressure\n"
		if $level_preserved{O3} > $level_preserved{O2};
	die "$test: native O3 increased the selected function's stack size\n"
		if stack_size($level_mir{O3}) > stack_size($level_mir{O2});

	# Exercise the ordinary driver path as well as the explicit staged path.
	my $driver_input = "$directory/driver.lowir";
	copy($test, $driver_input) or
		die "$test: unable to prepare driver replay: $!\n";
	my $driver_program = "$directory/driver.program";
	my $status = run_command_capture(
		cmd => [$driver, '-O3', '-o', $driver_program, $driver_input],
		stdout => "$directory/driver.stdout",
		stderr => "$directory/driver.stderr",
		timeout => 60,
	);
	die "$test: O3 driver integration compile failed\n" .
		read_file("$directory/driver.stderr") if $status != 0;
	$status = run_command_capture(
		cmd => [$driver_program],
		stdout => "$directory/driver.run.stdout",
		stderr => "$directory/driver.run.stderr",
		timeout => 30,
	);
	die "$test: O3 driver integration behavior failed with status $status\n"
		if $status != 0;
}

print "native parameter-address folding: PASS (" .
	scalar(@tests) . "/" . scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
