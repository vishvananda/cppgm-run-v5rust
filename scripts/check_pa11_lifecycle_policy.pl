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

sub function_body
{
	my ($test, $lowir, $name) = @_;
	return $1 if $lowir =~
		/(function \@\Q$name\E\b.*?)(?=\nfunction \@|\z)/s;
	die "$test: generated LowIR has no $name definition\n";
}

# The header and symbol of the definition of the C++ entity with the
# Itanium name `mangled`: either the function's own name or its `object=`
# metadata (the comparison pairs internal symbols by the latter).
sub definition_of_entity
{
	my ($lowir, $mangled) = @_;
	for my $line (split(/\n/, $lowir))
	{
		next if $line !~ /^function \@([A-Za-z0-9_]+)\(/;
		my $symbol = $1;
		return ($line, $symbol)
			if $symbol eq $mangled || $line =~ /\[[^\]]*\bobject=\Q$mangled\E(?:,|\])/;
	}
	return (undef, undef);
}

my $selection = qr/lifecycle-inline-policy\.(?:cpp|t)$/;
if (@ARGV && $ARGV[0] eq '--list')
{
	shift @ARGV;
	print "$_\n" for map { collect_tests($_, $selection) } @ARGV;
	exit 0;
}

if (scalar(@ARGV) != 2)
{
	die "Usage: check_pa11_lifecycle_policy.pl " .
		"<cppgm++> <test-or-directory>\n";
}

my ($app, $root) = @ARGV;
my @tests = collect_tests($root, $selection);
die "No PA11 lifecycle-policy properties found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('pa11-lifecycle-policy-XXXXXX', TMPDIR => 1,
		CLEANUP => 1);
	my $output = "$directory/output.lowir";
	my $status = run_command_capture(
		cmd => [$app, '--emit-lowir', '-O0', '-o', $output, $test],
		stdout => "$directory/compiler.stdout",
		stderr => "$directory/compiler.stderr",
		timeout => 60,
	);
	die "$test: source-to-LowIR compile failed\n" .
		read_file("$directory/compiler.stderr") if $status != 0;
	my $lowir = read_file($output);
	die "$test: generated LowIR states a symbol key the format does not define (trivial_lifecycle=)\n"
		if $lowir =~ /\btrivial_lifecycle\s*=/;
	for my $entry (qw(_ZN9MandatoryC1Ev _ZN9MandatoryC2Ev))
	{
		my ($header) = definition_of_entity($lowir, $entry);
		die "$test: the trivial Mandatory constructor was emitted ($entry)\n"
			if defined($header);
	}

	my ($header, $guarded) = definition_of_entity($lowir, '_ZN7GuardedC1Ev');
	die "$test: nontrivial Guarded constructor definition is missing\n"
		if !defined($header);
	die "$test: Guarded constructor does not state no_inline=yes (GNU noinline)\n"
		if $header !~ /\bno_inline=yes\b/;
	die "$test: Guarded constructor states force_inline=yes\n"
		if $header =~ /\bforce_inline=yes\b/;
	my $main = function_body($test, $lowir, 'main');
	die "$test: the call of the noinline Guarded constructor is missing from main\n"
		if $main !~ /^\s+call void \@\Q$guarded\E\(/m;
}

print "PA11 lifecycle policy properties: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
