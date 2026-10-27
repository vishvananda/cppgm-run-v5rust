#!/usr/bin/perl
# check_kernels.pl [--compiler ../dev/cppgm++] [--limit 1.50] [--scale 16] [--out obj/kernels]
#
# PA33's dynamic code-quality check. Runs PA35's kernels
# (../pa35/scripts/kernels.pl) with the compiler under test: each kernel is
# compiled by it and by GCC at the kernel's level, linked against one
# GCC-built driver, checked against the driver's oracle and counted under
# cachegrind. A kernel passes when its instructions are at most --limit
# times GCC's. Prints one PASS/FAIL line per kernel (PASS lines are omitted
# under CPPGM_REPORT_QUIET), records the counts the way the course lanes do
# under KEEP_GOING (.test_counts, .test_failed) and exits nonzero when a
# kernel fails or cannot be measured.
use strict;
use warnings;
use FindBin;
use File::Basename qw(dirname);
use File::Path qw(make_path);
use Getopt::Long;

my $pa33 = dirname($FindBin::Bin);
my $root = dirname($pa33);
my %o = (compiler => "$root/dev/cppgm++", limit => 1.50, scale => 16, out => "$pa33/obj/kernels");
GetOptions(\%o, 'compiler=s', 'limit=f', 'scale=i', 'out=s') && $o{limit} > 0 && $o{scale} >= 1
	or die "usage: check_kernels.pl [--compiler C] [--limit L] [--scale N] [--out DIR]\n";
my $kernels = "$root/pa35/scripts/kernels.pl";
-f $kernels or die "missing $kernels\n";

sub capture
{
	my (@command) = @_;
	open(my $fh, '-|', @command) or die "cannot run @command: $!\n";
	local $/;
	my $output = <$fh>;
	close($fh);
	return ($? >> 8, $output // '');
}

my ($list_status, $list) = capture('perl', $kernels, '--list');
my @names = grep { length } split(/\n/, $list);
die "kernels.pl --list failed\n$list" if $list_status || !@names;

make_path($o{out});
unlink("$o{out}/kernels.tsv");
my ($status, $output) = capture('perl', $kernels, '--compiler', $o{compiler}, '--out', $o{out}, '--scale', $o{scale});

# kernels.tsv holds one row per kernel: a ratio to GCC, or ERROR; the
# script's output names the error.
my %ratio;
if (open(my $tsv, '<', "$o{out}/kernels.tsv"))
{
	while (my $row = <$tsv>)
	{
		chomp $row;
		my ($name, $value) = split(/\t/, $row);
		$ratio{$name} = $value if defined $value && $name ne 'kernel';
	}
	close($tsv);
}
my %error = map { /^(\S+)\s+ERROR (.*)$/ ? ($1 => $2) : () } split(/\n/, $output);

my $passed = 0;
for my $name (@names)
{
	my $value = $ratio{$name};
	my $ok = defined $value && $value =~ /^\d+(?:\.\d+)?$/ && $value <= $o{limit};
	++$passed if $ok;
	my $shown = !defined $value ? ($status ? 'not measured' : 'missing')
		: $value =~ /^\d/ ? sprintf('%.4fx GCC', $value)
		: $error{$name} ? "ERROR $error{$name}" : $value;
	printf "%s kernel-%-18s %s (limit %.2f)\n", $ok ? 'PASS' : 'FAIL', $name, $shown, $o{limit}
		if !$ok || !$ENV{CPPGM_REPORT_QUIET};
}
my $total = scalar(@names);
print "kernels: $passed/$total pass\n" unless $ENV{CPPGM_REPORT_QUIET} && $passed == $total;
print $output if $passed != $total && !%ratio;

if ($ENV{KEEP_GOING})
{
	my $counts = defined($ENV{CPPGM_TEST_COUNTS_FILE}) && $ENV{CPPGM_TEST_COUNTS_FILE} ne ''
		? $ENV{CPPGM_TEST_COUNTS_FILE} : "$root/.test_counts";
	if (open(my $fh, '>>', $counts))
	{
		print $fh "$passed $total\n";
		close($fh);
	}
	if ($passed != $total && open(my $failed, '>>', "$pa33/.test_failed")) { close($failed); }
}
exit($passed == $total ? 0 : 1);
