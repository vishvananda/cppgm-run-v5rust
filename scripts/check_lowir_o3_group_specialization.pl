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

sub function_body
{
	my ($test, $lowir, $name) = @_;
	return $1 if $lowir =~
		/(function \@\Q$name\E\b.*?)(?=\nfunction \@|\z)/s;
	die "$test: optimized output has no $name definition\n";
}

sub compile_and_run
{
	my ($driver, $test, $directory, $lowir) = @_;
	my $program = "$directory/behavior";
	my $status = run_command_capture(
		cmd => [$driver, '-O0', '-o', $program, $lowir],
		stdout => "$directory/compile.stdout",
		stderr => "$directory/compile.stderr",
		timeout => 60,
	);
	die "$test: optimized LowIR did not compile\n" .
		read_file("$directory/compile.stderr") if $status != 0;
	$status = run_command_capture(
		cmd => [$program],
		stdout => "$directory/run.stdout",
		stderr => "$directory/run.stderr",
		timeout => 30,
	);
	die "$test: optimized behavior failed with status $status\n"
		if $status != 0;
}

if(scalar(@ARGV) != 3)
{
	die "Usage: check_lowir_o3_group_specialization.pl " .
		"<lowiropt> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/o3-group-specialization\.t$/);
die "No O3 grouped-specialization controls found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('lowir-o3-group-specialization-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my (undef, $o0) = optimize($app, $test, $directory, 'O0');
	my ($o1_path, $o1) = optimize($app, $test, $directory, 'O1');
	my (undef, $o2) = optimize($app, $test, $directory, 'O2');
	my ($o3_path, $o3) = optimize($app, $test, $directory, 'O3');

	# `value <=u 0` holds only for zero, so the zero edge decides the
	# redundant `value != 0` comparison and its `return i64 1` arm is dead.
	my $o0_edge = function_body($test, $o0, 'edge_target');
	die "$test: O0 control lacks the redundant zero comparison\n"
		if $o0_edge !~ /cmp ne i64 %value, 0/ ||
		   $o0_edge !~ /^\s+return i64 1$/m;
	for my $level_output (['O1', $o1], ['O2', $o2], ['O3', $o3])
	{
		my ($level, $lowir) = @$level_output;
		my $edge = function_body($test, $lowir, 'edge_target');
		die "$test: $level did not consume the unsigned-zero edge fact\n"
			if $edge =~ /cmp ne i64 %value, 0/ ||
			   $edge =~ /^\s+return i64 1$/m;
	}
	compile_and_run($driver, $test, $directory, $o1_path);
	compile_and_run($driver, $test, $directory, $o3_path);
}

print "O3 grouped specialization: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
