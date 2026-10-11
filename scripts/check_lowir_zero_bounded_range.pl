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

sub has_signed_bounds
{
	my ($body) = @_;
	return $body =~ /^\s+%\w+ = cmp lt i32 %value, 0$/m &&
		$body =~ /^\s+%\w+ = cmp gt i32 %value, 127$/m;
}

sub has_any_signed_bound
{
	my ($body) = @_;
	return $body =~ /^\s+%\w+ = cmp (?:lt i32 %value, 0|gt i32 %value, 127)$/m;
}

# The two signed bounds became one switch on %value that sends exactly
# 0 through 127 to ^accept and every other value to ^reject.
sub has_range_switch
{
	my ($body) = @_;
	my @switches = $body =~ /^\s+switch %value, (.*)$/mg;
	return 0 if scalar(@switches) != 1;
	my ($default, @cases) = split(/, /, $switches[0]);
	return 0 if $default ne '^reject' || scalar(@cases) != 128;
	my %values;
	for my $case (@cases)
	{
		return 0 if $case !~ /^(-?\d+):\^accept$/;
		$values{$1} = 1;
	}
	return !grep { !$values{$_} } (0 .. 127);
}

sub has_switch
{
	my ($body) = @_;
	return $body =~ /^\s+switch\b/m;
}

if (scalar(@ARGV) != 3)
{
	die "Usage: check_lowir_zero_bounded_range.pl " .
		"<lowiropt> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/zero-bounded-signed-range.*\.t$/);
die "No zero-bounded signed-range tests found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('cppgm-zero-bounded-range-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my %paths;
	my %outputs;
	for my $level ('O0', 'O1', 'O2', 'O3')
	{
		($paths{$level}, $outputs{$level}) =
			run_optimizer($app, $test, $directory, $level);
	}

	for my $level ('O0', 'O1')
	{
		for my $name ('fold_zero_bounded', 'fold_with_accept_phi')
		{
			my $body = function_body($test, $outputs{$level}, $name);
			die "$test: $level did not preserve the two signed bounds in $name\n"
				if !has_signed_bounds($body) || has_switch($body);
		}
	}

	for my $level ('O2', 'O3')
	{
		for my $name ('fold_zero_bounded', 'fold_with_accept_phi')
		{
			my $body = function_body($test, $outputs{$level}, $name);
			die "$test: $level did not combine the signed bounds in $name\n"
				if !has_range_switch($body) || has_any_signed_bound($body);
		}
		for my $name ('retain_distinct_rejections', 'retain_distinct_values',
			'retain_shared_upper', 'retain_rejection_phi')
		{
			my $body = function_body($test, $outputs{$level}, $name);
			die "$test: $level combined unsafe bounds in $name\n"
				if has_switch($body) ||
				   $body =~ /^\s+%\w+ = cmp ugt i32 /m ||
				   $body !~ /^\s+%\w+ = cmp lt i32 /m ||
				   $body !~ /^\s+%\w+ = cmp gt i32 /m;
		}
	}

	for my $level ('O1', 'O2', 'O3')
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

print "zero-bounded signed ranges: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
