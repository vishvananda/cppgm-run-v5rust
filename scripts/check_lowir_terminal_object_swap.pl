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
	return ($output, read_file($output));
}

sub function_body
{
	my ($test, $output, $name) = @_;
	return $1 if $output =~
		/(function \@\Q$name\E\b.*?)(?=\nfunction \@|\z)/s;
	die "$test: optimized output has no $name definition\n";
}

if (scalar(@ARGV) != 3)
{
	die "Usage: check_lowir_terminal_object_swap.pl " .
		"<lowiropt> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/o3-terminal-staged-object-swap.*\.t$/);
die "No O3 terminal staged-object-swap controls found under $root\n"
	if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('cppgm-terminal-object-swap-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my %paths;
	my %outputs;
	for my $level ('O0', 'O1', 'O2', 'O3')
	{
		($paths{$level}, $outputs{$level}) =
			run_optimizer($app, $test, $directory, $level);
	}

	# -O0 keeps the staged swap: the stage slot and its object copies.
	for my $name ('aggregate_swap', 'fieldwise_swap')
	{
		my $body = function_body($test, $outputs{O0}, $name);
		die "$test: O0 unexpectedly removed the staged swap in $name\n"
			if $body !~ /^\s+slot \$\w+ : obj</m ||
			   $body !~ /^\s+copyobj /m;
	}

	# From -O1 the stage slot is split into its fields: the swap captures
	# the first object's fields as scalars, before anything overwrites that
	# object, and stores each into the second (the copy between the two
	# objects themselves may stay an object copy).
	for my $level ('O1', 'O2', 'O3')
	{
		for my $name ('aggregate_swap', 'fieldwise_swap')
		{
			my $body = function_body($test, $outputs{$level}, $name);
			my $loads = () = $body =~ /^\s+%\w+ = load i64 /mg;
			my $stores = () = $body =~ /^\s+store i64 /mg;
			die "$test: $level did not lower the complete staged swap in $name\n"
				if $body =~ /^\s+slot \$/m || $loads < 4 || $loads != $stores;
			my $overwrite = $body =~
				/^\s+(?:copyobj \S+ %\w+, |zeroinit \S+ |store \S+ [^,\n]+, )%first$/m ?
				$-[0] : length($body);
			my $last_load = 0;
			$last_load = $-[0] while $body =~ /^\s+%\w+ = load i64 /mg;
			die "$test: $level captured $name after overwriting its first object\n"
				if $last_load > $overwrite;
		}

		# A stage whose address escapes stays in memory.
		my $escaped = function_body($test, $outputs{$level},
			'retain_escaped_stage');
		die "$test: $level lowered an unproved staged swap in retain_escaped_stage\n"
			if $escaped !~ /^\s+slot \$\w+ : obj</m ||
			   $escaped !~ /^\s+copyobj /m ||
			   $escaped !~ /^\s+call void \@observe\(%\w+\)$/m;

		# A volatile read of the captured object stays one volatile read.
		my $volatile = function_body($test, $outputs{$level},
			'retain_volatile_capture');
		my $volatile_loads = () = $volatile =~ /^\s+%\w+ = load volatile i64 /mg;
		die "$test: $level changed the volatile capture in retain_volatile_capture\n"
			if $volatile_loads != 1;

		# An incomplete stage captures only its 16 bytes: the zeroed rest
		# must not be read from the first object.
		my $incomplete = function_body($test, $outputs{$level},
			'retain_incomplete_stage');
		die "$test: $level widened the incomplete capture in retain_incomplete_stage\n"
			if $incomplete =~ /^\s+%\w+ = index \S+ (?:\[[^\]]*\] )?%first, (?:1[6-9]|2\d|3[01])$/m ||
			   $incomplete =~ /^\s+copyobj 32x8 %first, /m;
	}

	for my $level ('O0', 'O1', 'O3')
	{
		my $executable = "$directory/behavior-$level";
		my $compile_stderr = "$directory/compile-$level.stderr";
		my $compile_status = run_command_capture(
			cmd => [$driver, "-$level", '-o', $executable, $paths{$level}],
			stdout => "$directory/compile-$level.stdout",
			stderr => $compile_stderr,
			timeout => 60,
		);
		die "$test: $level optimized LowIR did not compile\n" .
			read_file($compile_stderr) if $compile_status != 0;
		my $run_status = run_command_capture(
			cmd => [$executable],
			stdout => "$directory/run-$level.stdout",
			stderr => "$directory/run-$level.stderr",
			timeout => 30,
		);
		die "$test: $level optimized behavior failed with status " .
			"$run_status\n" if $run_status != 0;
	}
}

print "terminal staged object swaps: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
