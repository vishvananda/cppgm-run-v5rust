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

sub main_body
{
	my ($test, $lowir) = @_;
	return $1 if $lowir =~ /(function \@main\b.*?)(?=\nfunction \@|\z)/s;
	die "$test: optimized output has no main definition\n";
}

sub compile_and_run
{
	my ($driver, $test, $directory, $level, $lowir) = @_;
	my $program = "$directory/behavior-$level";
	my $stderr = "$directory/compile-$level.stderr";
	my $status = run_command_capture(
		cmd => [$driver, "-$level", '-o', $program, $lowir],
		stdout => "$directory/compile-$level.stdout",
		stderr => $stderr,
		timeout => 60,
	);
	die "$test: $level optimized LowIR did not compile\n" .
		read_file($stderr) if $status != 0;
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
	die "Usage: check_lowir_inlined_addressed_store.pl " .
		"<lowiropt> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/inlined-addressed-store.*\.t$/);
die "No inlined addressed-store controls found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('lowir-inlined-addressed-store-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my (%outputs, %paths);
	for my $level ('O0', 'O1', 'O2', 'O3') {
		($outputs{$level}, $paths{$level}) =
			optimize($app, $test, $directory, $level);
	}

	# O0 keeps the slot whose address the callee stores through.  Every
	# optimizing level inlines the internal callee, after which the
	# address only reaches a whole-slot store and load: the slot, its
	# address, the call and the reload are all gone.
	my $o0 = main_body($test, $outputs{O0});
	die "$test: O0 did not retain the unoptimized addressed slot\n"
		if $o0 !~ /^\s+slot \$value : i32$/m ||
		   $o0 !~ /^\s+%\w+ = addr \$value$/m ||
		   $o0 !~ /^\s+call void \@write_inlined_value\(%\w+\)$/m;
	for my $level ('O1', 'O2', 'O3') {
		my $body = main_body($test, $outputs{$level});
		die "$test: $level did not inline the addressed store\n"
			if $body =~ /\bcall void \@write_inlined_value\(/;
		die "$test: $level retained the recoverable addressed slot\n"
			if $body =~ /^\s+slot \$value : i32$/m ||
			   $body =~ /^\s+%\w+ = addr \$value$/m;
		die "$test: $level did not forward the inlined store to its load\n"
			if $body =~ /^\s+(?:store i32 |%\w+ = load i32 )/m;
	}

	for my $level ('O1', 'O2', 'O3') {
		compile_and_run(
			$driver, $test, $directory, $level, $paths{$level});
	}
}

print "inlined addressed stores: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
