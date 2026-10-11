#!/usr/bin/perl
# Self-checking controls for the LowIR-graded assignments (PA10-PA23).
#
# A `tests/controls/*.cpp` that states its expected outcome is compiled by the
# student's frontend (`cppgm++ --emit-lowir -O0`), the LowIR is built by the
# supplied reference backend (`lowir2native-ref -O0`), and the program is run
# with no input: it must exit with the stated status, and print the stated
# output when one is given.  A control that states a rejection must make the
# frontend fail.  The LowIR text itself is not compared, so any LowIR whose
# behaviour is right passes.  A program or frontend killed by a signal fails:
# SIGABRT is not `exit 134`, nor a crashing frontend a rejection.
#
# The expected outcome comes from, in order:
#   NAME.expect  one line, `exit N` or `reject`; NAME.stdout, when present,
#                holds the exact expected standard output;
#   the leading `//` comment block of NAME.cpp: `expect exit N` (also
#                `expect link ok and exit N`), or `expect reject`,
#                `expect compile reject`, `must be rejected`, `must reject`.
# Files that state no outcome belong to the assignment's focused checkers
# (scripts/check_pa1[012]_*.pl) and are skipped when a directory is checked.
#
# Usage: check_controls.pl <cppgm++> <native-reference> <test-or-directory>
#        check_controls.pl --owns <test> [<test> ...]
#            exits 0 when every named file states an expected outcome.
#        check_controls.pl --list <test-or-directory> [...]
#            prints the controls a run over those paths grades, one a line
#            (scripts/check_fixture_lanes.py).

use strict;
use warnings;

use Config;
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

sub leading_comment
{
	my ($source) = @_;
	my @lines;
	for my $line (split(/\n/, $source))
	{
		if ($line =~ m{^\s*//(.*)$})
		{
			push @lines, $1;
			next;
		}
		last if @lines || $line !~ /^\s*$/;
	}
	return join(' ', @lines);
}

# Returns (kind, status, expected stdout or undef); kind '' when the file
# states no expected outcome.
sub expectation
{
	my ($test) = @_;
	(my $base = $test) =~ s/\.cpp$//;
	my $stdout = -f "$base.stdout" ? read_file("$base.stdout") : undef;
	if (-f "$base.expect")
	{
		my $text = read_file("$base.expect");
		return ('exit', $1, $stdout) if $text =~ /^\s*exit\s+(\d+)\s*$/m;
		return ('reject', undef, undef) if $text =~ /^\s*reject\s*$/m;
		die "$test: $base.expect states neither `exit N` nor `reject`\n";
	}
	my $comment = leading_comment(read_file($test));
	my ($exit_at, $reject_at) = (-1, -1);
	my $status;
	if ($comment =~ /\bexpect(?:ed)?\s+(?:link\s+ok\s+and\s+)?exit\s+(\d+)/i)
	{
		($exit_at, $status) = ($-[0], $1);
	}
	if ($comment =~ /\bexpect(?:ed)?\s+(?:compile\s+)?reject|\bmust\s+(?:be\s+)?reject/i)
	{
		$reject_at = $-[0];
	}
	return ('exit', $status, $stdout)
		if $exit_at >= 0 && ($reject_at < 0 || $exit_at < $reject_at);
	return ('reject', undef, undef) if $reject_at >= 0;
	return ('', undef, undef);
}

sub tail_of
{
	my ($path) = @_;
	return '' if !-f $path;
	my $text = read_file($path);
	$text =~ s/\s+$//;
	return length($text) > 400 ? '...' . substr($text, -400) : $text;
}

my @signal_names = split(' ', $Config{sig_name});

# How a command ended, from run_command_capture's outcome: `timed out`,
# `killed by signal N (SIGNAME)` or `exited with status N`.
sub ending
{
	my ($outcome) = @_;
	return 'timed out' if $outcome->{timed_out};
	if ($outcome->{signal})
	{
		my $name = $signal_names[$outcome->{signal}];
		return "was killed by signal $outcome->{signal}" .
			(defined($name) ? " (SIG$name)" : '');
	}
	return "exited with status $outcome->{exit}";
}

sub grade
{
	my ($app, $native_reference, $test) = @_;
	my ($kind, $status, $stdout) = expectation($test);
	my $directory = tempdir('cppgm-control-XXXXXX', TMPDIR => 1, CLEANUP => 1);
	my $lowir = "$directory/control.lowir";
	my %compile;
	run_command_capture(
		cmd => [$app, '--emit-lowir', '-O0', '-o', $lowir, $test],
		stdout => "$directory/frontend.stdout",
		stderr => "$directory/frontend.stderr",
		timeout => 60,
		outcome => \%compile,
	);
	if ($kind eq 'reject')
	{
		return 'frontend ' . ending(\%compile) . ' instead of rejecting'
			if $compile{timed_out} || $compile{signal};
		return $compile{exit} != 0 ? undef : 'frontend accepted an ill-formed program';
	}
	return 'frontend ' . ending(\%compile) . ': ' . tail_of("$directory/frontend.stderr")
		if $compile{timed_out} || $compile{signal} || $compile{exit} != 0;
	my $program = "$directory/program";
	my %build;
	run_command_capture(
		cmd => [$native_reference, '-O0', '-o', $program, $lowir],
		stdout => "$directory/native.stdout",
		stderr => "$directory/native.stderr",
		timeout => 60,
		outcome => \%build,
	);
	return 'reference backend ' . ending(\%build) . ' on the generated LowIR: ' .
		tail_of("$directory/native.stderr")
		if $build{timed_out} || $build{signal} || $build{exit} != 0;
	my %run;
	run_command_capture(
		cmd => [$program],
		stdin => '/dev/null',
		stdout => "$directory/program.stdout",
		stderr => "$directory/program.stderr",
		timeout => 30,
		outcome => \%run,
	);
	return 'program ' . ending(\%run) . ", expected exit status $status"
		if $run{timed_out} || $run{signal} || $run{exit} != $status;
	if (defined($stdout))
	{
		my $got = read_file("$directory/program.stdout");
		return 'program output differs from the expected output' if $got ne $stdout;
	}
	return undef;
}

if (@ARGV && $ARGV[0] eq '--owns')
{
	shift @ARGV;
	die "Usage: check_controls.pl --owns <test> [<test> ...]\n" if !@ARGV;
	my @tests = map { collect_tests($_, qr/\.cpp$/) } @ARGV;
	exit 1 if !@tests;
	for my $test (@tests)
	{
		my ($kind) = expectation($test);
		exit 1 if $kind eq '';
	}
	exit 0;
}

if (@ARGV && $ARGV[0] eq '--list')
{
	shift @ARGV;
	die "Usage: check_controls.pl --list <test-or-directory> [...]\n" if !@ARGV;
	for my $test (map { collect_tests($_, qr/\.cpp$/) } @ARGV)
	{
		print "$test\n" if (expectation($test))[0] ne '';
	}
	exit 0;
}

die "Usage: check_controls.pl <cppgm++> <native-reference> <test-or-directory>\n"
	if scalar(@ARGV) != 3;
my ($app, $native_reference, $root) = @ARGV;
my @candidates = collect_tests($root, qr/\.cpp$/);
my @tests = grep { (expectation($_))[0] ne '' } @candidates;
if (-f $root && !@tests)
{
	die "$root: states no expected outcome (`expect exit N` or `expect reject` " .
		"in its leading comment, or a .expect sidecar)\n";
}
my @failures;
for my $test (@tests)
{
	my $failure = grade($app, $native_reference, $test);
	push @failures, "$test: FAIL: $failure" if defined($failure);
}
if (@failures)
{
	print STDERR "$_\n" for @failures;
	die "self-checking controls: FAIL (" . (scalar(@tests) - scalar(@failures)) .
		"/" . scalar(@tests) . " passed)\n";
}
print "self-checking controls: PASS (" . scalar(@tests) . "/" . scalar(@tests) . ")\n"
	if @tests && !$ENV{CPPGM_REPORT_QUIET};
