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

# The definition of the C++ entity with the Itanium name `mangled`: either
# the function's own name or its `object=` metadata.
sub definition_of_entity
{
	my ($lowir, $mangled) = @_;
	for my $function (split(/(?=^function \@)/m, $lowir))
	{
		next if $function !~ /^function \@([A-Za-z0-9_]+)\(([^\n]*)/;
		my ($symbol, $rest) = ($1, $2);
		return $function
			if $symbol eq $mangled || $rest =~ /\[[^\]]*\bobject=\Q$mangled\E(?:,|\])/;
	}
	return undef;
}

my $selection = qr/unreachable-terminator\.(?:cpp|t)$/;
if (@ARGV && $ARGV[0] eq '--list')
{
	shift @ARGV;
	print "$_\n" for map { collect_tests($_, $selection) } @ARGV;
	exit 0;
}

if (scalar(@ARGV) != 2)
{
	die "Usage: check_pa11_unreachable_terminator.pl " .
		"<cppgm++> <test-or-directory>\n";
}

my ($app, $root) = @ARGV;
my @tests = collect_tests($root, $selection);
die "No PA11 unreachable-terminator properties found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('pa11-unreachable-XXXXXX', TMPDIR => 1,
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
	die "$test: __builtin_unreachable did not lower to the `unreachable` terminator\n"
		if $lowir !~ /^\s+unreachable\s*$/m;
	die "$test: __builtin_unreachable was lowered as a declaration or a call instead of the `unreachable` terminator\n"
		if $lowir =~ /\brole=unreachable\b|^declare function \@[^\n]*unreachable|^\s+call\s+void\s+\@[^\n]*unreachable/m;
	my $guarded = definition_of_entity($lowir, '_Z13guarded_valueb');
	die "$test: guarded source function is missing\n" if !defined($guarded);
	die "$test: unreachable did not terminate its source block\n"
		if $guarded !~ /^\s+block \^[A-Za-z0-9_]+:\n\s+unreachable\s*(?:\n\s*)?(?=block \^|\})/m;
}

print "PA11 unreachable lowering properties: PASS (" . scalar(@tests) .
	"/" . scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
