#!/usr/bin/env perl
# abba.pl A B [--level -O3] [--blocks 12] [--runs 1] [--allow-different]
#
# Paired timing of two compilers on the frozen workload. Each block runs
# A B B A (each side --runs compiles per position); the block statistic is
# mean(B)/mean(A) - 1 for user, user+sys and wall time, measured with
# microsecond rusage. Prints the median and range over blocks. The objects
# must be identical unless --allow-different. Run A against a copy of A to
# calibrate the noise (A/A) before trusting a small difference, and compare
# binaries linked the same way (freeze.pl and build_with.pl do that).
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Compare qw(compare);
use File::Temp qw(tempdir);
use Getopt::Long;
use PA35;

my %o = (level => '-O3', blocks => 12, runs => 1);
GetOptions(\%o, 'level=s', 'blocks=i', 'runs=i', 'allow-different') && @ARGV == 2
	or die "usage: abba.pl A B [--level -O3] [--blocks N] [--runs N] [--allow-different]\n";
my ($A, $B) = map { abs_path($_) // die "no compiler $_\n" } @ARGV;
PA35::verify_workload();
my $tmp = tempdir('pa35-abba-XXXXXX', TMPDIR => 1, CLEANUP => 1);
my $run = sub {
	my ($compiler, $out) = @_;
	return PA35::timed_run($compiler, PA35::workload_args($o{level}, $out));
};
$run->($A, "$tmp/a.o");
$run->($B, "$tmp/b.o");
die "objects differ (pass --allow-different for compilers whose output differs)\n"
	if !$o{'allow-different'} && compare("$tmp/a.o", "$tmp/b.o") != 0;
my (%ratio, %mean);
for my $block (1 .. $o{blocks}) {
	my %sum = (a => [0, 0, 0], b => [0, 0, 0]);
	for my $side (qw(a b b a)) {
		for (1 .. $o{runs}) {
			my $t = $run->($side eq 'a' ? $A : $B, "$tmp/$side.o");
			my @v = ($t->{user}, $t->{user} + $t->{sys}, $t->{wall});
			$sum{$side}[$_] += $v[$_] for 0 .. 2;
		}
	}
	my @names = qw(user cpu wall);
	push @{ $ratio{ $names[$_] } }, $sum{b}[$_] / $sum{a}[$_] - 1 for 0 .. 2;
	push @{ $mean{$_} }, $sum{$_}[0] / (2 * $o{runs}) for qw(a b);
}
printf "A user median %.4fs  B user median %.4fs  (%d blocks x ABBA x %d, %s)\n",
	PA35::median(@{ $mean{a} }), PA35::median(@{ $mean{b} }), $o{blocks}, $o{runs}, $o{level};
for my $name (qw(user cpu wall)) {
	my @r = sort { $a <=> $b } @{ $ratio{$name} };
	printf "paired %-5s B/A-1: median %+.2f%%  range [%+.2f%%, %+.2f%%]\n",
		$name, 100 * PA35::median(@r), 100 * $r[0], 100 * $r[-1];
}
