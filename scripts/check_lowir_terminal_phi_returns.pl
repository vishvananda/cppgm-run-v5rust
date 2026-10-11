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

sub function_body
{
	my ($test, $lowir, $name) = @_;
	return $1 if $lowir =~
		/(function \@\Q$name\E\b.*?)(?=\nfunction \@|\z)/s;
	die "$test: optimized output has no $name definition\n";
}

sub instruction_count
{
	my ($body, $instruction) = @_;
	return scalar(() = $body =~ /^\s+\Q$instruction\E\b/mg);
}

sub phi_count
{
	my ($body) = @_;
	return scalar(() = $body =~ /^\s+%\w+ = phi\b/mg);
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
	die "Usage: check_lowir_terminal_phi_returns.pl " .
		"<lowiropt> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/terminal-phi-return\.t$/);
die "No terminal-phi return controls found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('lowir-terminal-phi-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my ($o0) = optimize($app, $test, $directory, 'O0');
	my ($o1, $o1_path) = optimize($app, $test, $directory, 'O1');
	my ($o2, $o2_path) = optimize($app, $test, $directory, 'O2');
	my ($o3, $o3_path) = optimize($app, $test, $directory, 'O3');

	# A merge phi whose constant input decides the branch below it on that
	# edge is threaded onto the return paths: from O1 the phi is gone and
	# each arm reaches its own return.
	my $positive = 'thread_phi_to_return_branch';
	die "$test: O0 changed the terminal phi in $positive\n"
		if phi_count(function_body($test, $o0, $positive)) != 1;
	for my $optimized (['O1', $o1], ['O2', $o2], ['O3', $o3]) {
		my ($level, $lowir) = @{$optimized};
		my $body = function_body($test, $lowir, $positive);
		die "$test: $level retained the eligible phi in $positive\n"
			if phi_count($body) != 0;
		die "$test: $level did not distribute $positive onto return paths\n"
			if instruction_count($body, 'return') < 2;
		my %return_only;
		while($body =~ /^\s*block \^([^:]+):\n\s+return\b[^\n]*\n(?:\s*\n|\}|\z)/mg) {
			$return_only{$1} = 1;
		}
		my ($entry) = $body =~ /^\s*block \^[^:]+:\n(.*?)(?=^\s*block \^|\z)/ms;
		my @targets = defined($entry) ? $entry =~ /\^(\w+)/g : ();
		die "$test: $level did not thread the decided edge of $positive to its return\n"
			if !grep { $return_only{$_} } @targets;
	}

	compile_and_run($driver, $test, $directory, $o1_path);
	compile_and_run($driver, $test, $directory, $o2_path);
	compile_and_run($driver, $test, $directory, $o3_path);
}

print "terminal-phi returns: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
