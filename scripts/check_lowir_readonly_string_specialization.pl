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
	my ($lowir, $name) = @_;
	return scalar(() = $lowir =~ /call i64 \@\Q$name\E\(/g);
}

# Names that main binds to the address of each global.
sub address_names
{
	my ($main) = @_;
	my %names;
	while ($main =~ /^\s+%([A-Za-z0-9_]+) = addr \@([A-Za-z0-9_]+)$/mg)
	{
		push(@{$names{$2}}, $1);
	}
	return \%names;
}

# Whether main still reads the first byte of a global at run time: a direct
# byte load through its address or a classify call that receives it.
sub reads_global_byte
{
	my ($main, $names, $global) = @_;
	for my $name (@{$names->{$global} // []})
	{
		return 1 if $main =~ /^\s+%\S+ = load (?:i8|u8) %\Q$name\E$/m ||
			$main =~ /\bcall i64 \@classify\(%\Q$name\E,/;
	}
	return 0;
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
	die "Usage: check_lowir_readonly_string_specialization.pl " .
		"<lowiropt> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/readonly-string-specialization\.t$/);
die "No readonly-string specialization controls found under $root\n"
	if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('lowir-readonly-string-specialization-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my (undef, $o0) = optimize($app, $test, $directory, 'O0');
	my ($o1_path, $o1) = optimize($app, $test, $directory, 'O1');
	my (undef, $o2) = optimize($app, $test, $directory, 'O2');
	my ($o3_path, $o3) = optimize($app, $test, $directory, 'O3');
	die "$test: O0 changed readonly-string call groups\n"
		if call_count($o0, 'classify') != 7;

	for my $level_output (['O1', $o1], ['O2', $o2], ['O3', $o3])
	{
		my ($level, $lowir) = @$level_output;
		my $main = function_body($test, $lowir, 'main');
		my $names = address_names($main);
		# Internal, readonly, NUL-terminated strings: classify's byte load
		# and switch are decided at each such call.
		for my $global ('letter', 'digit', 'high_byte')
		{
			die "$test: $level did not fold the readonly string byte of " .
				"\@$global\n" if reads_global_byte($main, $names, $global);
		}
		# A GNU weak string may be replaced at link time and a writable one
		# may change, so their bytes are read at run time.
		for my $global ('replaceable', 'writable')
		{
			die "$test: $level folded the byte of \@$global as immutable " .
				"data\n" if !reads_global_byte($main, $names, $global);
		}
		my $dynamic = function_body($test, $lowir, 'dynamic_calls');
		die "$test: $level specialized a dynamically supplied string\n"
			if $dynamic !~ /^\s+%\S+ = load i8 %text$/m &&
			   $dynamic !~ /\bcall i64 \@classify\(%text,/;

		my $volatile = function_body($test, $lowir, 'volatile_probe');
		die "$test: $level specialized volatile byte access as immutable data\n"
			if call_count($lowir, 'volatile_probe') != 2 ||
			   $volatile !~ /^\s+%first = load volatile i8 %text$/m;
		my $indexed = function_body($test, $lowir, 'indexed_probe');
		die "$test: $level specialized a variable byte offset as a fixed byte\n"
			if call_count($lowir, 'indexed_probe') != 2 ||
			   $indexed !~ /^\s+%element = index i8 \[projection=array_element\] %text, %offset$/m ||
			   $indexed !~ /^\s+%byte = load u8 %element$/m;
	}
	compile_and_run($driver, $test, $directory, $o1_path);
	compile_and_run($driver, $test, $directory, $o3_path);
}

print "Readonly-string specialization: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
