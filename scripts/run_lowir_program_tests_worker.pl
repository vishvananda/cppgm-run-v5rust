#!/usr/bin/perl

use strict;
use warnings;

use Cwd qw(getcwd);
use File::Basename qw(basename);
use FindBin;
use lib $FindBin::Bin;

use CppgmBatchWorker qw(
	clear_progress_state
	close_worker
	collect_tests
	detect_jobs
	ensure_test_app_available
	get_timeout_from_env
	note_progress_state
	open_worker
	print_test_run_summary
	run_command_capture
	submit_cli_request
	write_file
	write_numeric_status
);

# Construct student LowIR, combine it with the fixture's calling program,
# then compile and execute it with the supplied native backend. The fixture
# does not supply the function implementation the student is asked to build.
sub process_one_test
{
	my ($app, $backend, $suffix, $test, $worker_out, $worker_in) = @_;
	note_progress_state('build', $test);
	my $test_base = $test;
	$test_base =~ s/\.t$//;

	unlink(glob("$test_base.$suffix.lowir"));
	unlink(glob("$test_base.$suffix.program"));
	unlink(glob("$test_base.$suffix.program.exit_status"));
	unlink(glob("$test_base.$suffix.program.stdout"));
	unlink(glob("$test_base.$suffix.program.stderr"));
	unlink(glob("$test_base.$suffix.impl.stdout"));
	unlink(glob("$test_base.$suffix.impl.stderr"));
	unlink(glob("$test_base.$suffix.impl.exit_status"));

	my $impl_stdout = "$test_base.$suffix.impl.stdout";
	my $impl_stderr = "$test_base.$suffix.impl.stderr";
	write_file($impl_stdout, '');
	write_file($impl_stderr, '');

	my $lowir_source = "$test_base.$suffix.lowir";
	open(my $exercise_file, '<', "$test_base.exercise")
		or die "Missing exercise name for $test: $!\n";
	my $exercise = do { local $/; <$exercise_file> };
	close($exercise_file);
	$exercise =~ s/\s+$//;
	die "Invalid exercise name for $test\n" if $exercise !~ /\A[a-z]+\z/;
	my $build_timeout = get_timeout_from_env("CPPGM_BUILD_TEST_TIMEOUT_SEC", 30);
	my $impl_status = defined($worker_in) ? submit_cli_request($worker_in,
	                                     $worker_out,
	                                     $impl_stdout,
	                                     $impl_stderr,
	                                     { CPPGM_BATCH_TIMEOUT_SEC => $build_timeout },
	                                     '--exercise', $exercise, '-o', $lowir_source) :
		run_command_capture(
			cmd => [$app, CppgmBatchWorker::app_args_for($app),
			        '--exercise', $exercise, '-o', $lowir_source],
			stdout => $impl_stdout,
			stderr => $impl_stderr,
			timeout => $build_timeout,
		);

	if ($impl_status == 0)
	{
		note_progress_state('native', $test);
		$impl_status = run_command_capture(
			cmd => [$backend, '-O0', '-o', "$test_base.$suffix.program", $lowir_source, $test],
			stdout => $impl_stdout,
			stderr => $impl_stderr,
			timeout => $build_timeout,
		);
	}
	write_numeric_status("$test_base.$suffix.impl.exit_status", $impl_status);

	if ($impl_status == 0)
	{
		note_progress_state('run', $test);
		my $program_status = run_command_capture(
			cmd => ["$test_base.$suffix.program"],
			stdout => "$test_base.$suffix.program.stdout",
			stderr => "$test_base.$suffix.program.stderr",
			stdin => (-f "$test_base.stdin" ? "$test_base.stdin" : undef),
			timeout => get_timeout_from_env("CPPGM_PROGRAM_TEST_TIMEOUT_SEC", 10),
		);
		write_numeric_status("$test_base.$suffix.program.exit_status", $program_status);
	}
	else
	{
		unlink(glob("$test_base.$suffix.program"));
		unlink(glob("$test_base.$suffix.program.exit_status"));
		unlink(glob("$test_base.$suffix.program.stdout"));
		unlink(glob("$test_base.$suffix.program.stderr"));
	}
}

sub run_program_tests
{
	my ($app, $backend, $suffix, $tests, $verbose) = @_;
	my ($worker_pid, $worker_out, $worker_in);
	if ($ENV{CPPGM_BATCH_TESTS} &&
		(!defined($ENV{CPPGM_TEST_RUNNER}) || $ENV{CPPGM_TEST_RUNNER} ne '0'))
	{
		($worker_pid, $worker_out, $worker_in) = open_worker($app);
	}
	for my $test (@{$tests})
	{
		print "Running $test...\n" if $verbose;
		process_one_test($app, $backend, $suffix, $test, $worker_out, $worker_in);
	}
	close_worker($worker_pid, $worker_out, $worker_in) if defined($worker_pid);
}

if (scalar(@ARGV) != 4)
{
	die "Usage: run_lowir_program_tests_worker.pl <app> <native-backend> <suffix> <testlocation>";
}

my ($app, $backend, $suffix, $tests_root) = @ARGV;
ensure_test_app_available($app, $suffix, $tests_root);
ensure_test_app_available($backend, $suffix, $tests_root);
my @tests = collect_tests($tests_root, qr/\.t$/);
my $verbose = $ENV{VERBOSE} || $ENV{CPGM_TEST_VERBOSE};
my $keep_going = $ENV{KEEP_GOING};
my $assignment = basename(getcwd());
if (!$verbose && !$keep_going)
{
	print_test_run_summary($assignment, $tests_root, \@tests);
}
my $ntests = scalar(@tests);
my $jobs = detect_jobs();
$jobs = $ntests if $jobs > $ntests;
if ($jobs <= 1)
{
	clear_progress_state();
	run_program_tests($app, $backend, $suffix, \@tests, $verbose);
	clear_progress_state();
	exit 0;
}

clear_progress_state();
my @shards;
for (my $i = 0; $i < $jobs; ++$i)
{
	$shards[$i] = [];
}
for (my $i = 0; $i < @tests; ++$i)
{
	push @{$shards[$i % $jobs]}, $tests[$i];
}

my @pids;
for my $shard (@shards)
{
	next if scalar(@{$shard}) == 0;
	my $pid = fork();
	die "fork failed: $!" if !defined($pid);
	if ($pid == 0)
	{
		run_program_tests($app, $backend, $suffix, $shard, $verbose);
		exit 0;
	}
	push @pids, $pid;
}

my $failed = 0;
for my $pid (@pids)
{
	waitpid($pid, 0);
	$failed = 1 if $? != 0;
}

clear_progress_state();
exit($failed ? 1 : 0);
