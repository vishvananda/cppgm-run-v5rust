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

sub call_count
{
	my ($body, $callee) = @_;
	return scalar(() = $body =~ /call i64 \@\Q$callee\E\(/g);
}

sub require_pair_calls
{
	my ($test, $lowir, $function, $expected, $level) = @_;
	my $body = function_body($test, $lowir, $function);
	my $actual = call_count($body, 'preferred_body');
	die "$test: $level $function has $actual preferred-body calls, " .
		"expected $expected\n" if $actual != $expected;
	return $body;
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
	die "Usage: check_lowir_o3_loop_priority_inline.pl " .
		"<lowiropt> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/o3-loop-priority-inline\.t$/);
die "No O3 loop-priority inline controls found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('lowir-o3-loop-priority-inline-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my (undef, $o0) = optimize($app, $test, $directory, 'O0');
	for my $caller (qw(loop_first loop_second no_loop_pair two_loop_calls))
	{
		require_pair_calls($test, $o0, $caller, 2, 'O0');
	}
	my $complete = call_count(function_body($test, $o0, 'preferred_body'),
		'advance');

	# A call inside a loop runs once per iteration, so its callee's inline
	# caps double there: from O1 a hinted callee too large for straight-line
	# sites is expanded at the loop site of loop_first and loop_second,
	# while its straight-line calls (their entry calls, no_loop_pair's pair)
	# stay calls to the retained definition. (two_loop_calls, a small body
	# with a call-free path to its return, is a wrapper the inliner keeps
	# large callees out of; it is not checked.)
	for my $level (qw(O1 O2 O3))
	{
		my ($path, $lowir) = optimize($app, $test, $directory, $level);
		for my $caller (qw(loop_first loop_second))
		{
			my $body = require_pair_calls($test, $lowir, $caller, 1, $level);
			my ($entry) = $body =~
				/^\s*block \^[^:]+:\n(.*?)(?=^\s*block \^|\z)/ms;
			die "$test: $level expanded the straight-line site in $caller " .
				"instead of its loop site\n"
				if !defined($entry) ||
				   call_count($entry, 'preferred_body') != 1;
			die "$test: $level $caller does not contain the complete body\n"
				if call_count($body, 'advance') != $complete;
			my ($failure_label, $failure_body);
			while($body =~
				/^\s*block \^([^:]+):\n(.*?)(?=^\s*block \^|\z)/msg) {
				my ($label, $block) = ($1, $2);
				next if $block !~ /^\s+call void \@fail\(\)$/m;
				die "$test: $level $caller contains more than one cloned failure arm\n"
					if defined($failure_label);
				($failure_label, $failure_body) = ($label, $block);
			}
			die "$test: $level $caller lost the cloned noreturn arm\n"
				if !defined($failure_label);
			die "$test: $level inlining retained a normal successor after noreturn\n"
				if $failure_body =~ /^\s+(?:jump|branch|switch)\b/m;
			die "$test: $level removed noreturn successor remains in a merge phi\n"
				if $body =~ /^\s+%\w+ = phi [^\n]*\[\^\Q$failure_label\E:/m;
		}
		my $pair = require_pair_calls($test, $lowir, 'no_loop_pair', 2, $level);
		die "$test: $level copied the callee into straight-line no_loop_pair\n"
			if call_count($pair, 'advance') != 0;
		die "$test: $level shared callable body was not retained for " .
			"non-loop calls\n"
			if call_count(function_body($test, $lowir, 'preferred_body'),
				'advance') != $complete;
		compile_and_run($driver, $test, $directory, $path);
	}
}

print "loop-site priority inlining: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
