#!/usr/bin/env perl
# kernels.pl [--compiler C] [--host g++] [--out DIR] [--scale N] | --list
#
# Generated-code kernels: each kernel in pa35/kernels is compiled by C (and
# by the host GCC as the yardstick) at its level, linked against the same
# GCC-built driver, checked against the driver's scalar oracle on edge
# inputs, then run under cachegrind. The work is the instruction count of
# the measured input minus that of a zero-iteration run. Prints one line
# per kernel with C's work relative to GCC's and writes DIR/kernels.tsv.
# --scale N divides every iteration count by N for a faster run; the
# zero-iteration subtraction keeps the ratios nearly unchanged.
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Path qw(make_path);
use Getopt::Long;
use PA35;

my %a = (compiler => "$PA35::ROOT/dev/cppgm++", host => 'g++', out => "$PA35::PA35/obj/kernels", scale => 1);
GetOptions(\%a, 'compiler=s', 'host=s', 'out=s', 'scale=i', 'list') && $a{scale} >= 1
	or die "usage: kernels.pl [--compiler C] [--host g++] [--out DIR] [--scale N] | --list\n";
my $K = "$PA35::PA35/kernels";

# name, driver case, source, level, accessor depth, iterations, alias flag
my @CASES = (
	['accessor-distinct', 1, 'accessor.cpp', 2, 64, 1 << 24, 0],
	['accessor-alias', 1, 'accessor.cpp', 2, 64, 1 << 24, 1],
	['lookup', 2, 'lookup.cpp', 3, 0, 1 << 25, 0],
	['exceptions', 3, 'exception.cpp', 3, 0, 1 << 23, 0],
);
# --list prints the kernel names, so a check that counts them (pa33's) knows
# the total before anything is measured.
if ($a{list}) { print "$_->[0]\n" for @CASES; exit 0; }
my $compiler = abs_path($a{compiler}) // die "no compiler $a{compiler}\n";
my @EDGES = (0, 1, 3, 4, 15, 16, 17, 18, 257, 1023, 1024, 1025);

# Generated programs that loop forever must fail, not hang the check.
my ($COMPILE_LIMIT, $RUN_LIMIT, $MEASURE_LIMIT) = (300, 20, 900);

sub irefs {
	my (@cmd) = @_;
	my ($status, $out) = PA35::run_capture('timeout', $MEASURE_LIMIT, 'valgrind', '--tool=cachegrind', '--cache-sim=no',
		'--cachegrind-out-file=/dev/null', @cmd);
	die $status == 124 ? "timed out under cachegrind: @cmd\n" : "cachegrind failed: @cmd\n$out" if $status;
	my ($n) = $out =~ /I\s+refs:\s+([\d,]+)/ or die "no I refs for @cmd\n";
	$n =~ tr/,//d;
	return $n;
}

sub run_limited {
	my ($limit, @cmd) = @_;
	my ($status, $out) = PA35::run_capture('timeout', $limit, @cmd);
	die "timed out after $limit s: @cmd\n" if $status == 124;
	die "command failed ($status): @cmd\n$out" if $status;
	return $out;
}

make_path($a{out});
my $out = abs_path($a{out});
my @rows;
my $failed = 0;
for my $c (@CASES) {
	my ($name, $case, $src, $level, $depth, $count, $alias) = @$c;
	$count = int($count / $a{scale});
	my $dir = "$out/$name";
	make_path($dir);
	my $result = eval {
		PA35::progress("kernels: $name: compiling host and candidate");
		PA35::run_or_die($a{host}, '-std=c++11', '-O2', "-DQUALITY_CASE=$case", "-I$K", '-c', "$K/driver.cpp", '-o', "$dir/driver.o");
		my %prog;
		for my $side (['host', $a{host}], ['candidate', $compiler]) {
			my ($label, $cxx) = @$side;
			run_limited($COMPILE_LIMIT, $cxx, '-std=c++11', "-O$level", '-g0', "-DQUALITY_DEPTH=$depth", "-I$K", '-c', "$K/$src", '-o', "$dir/$label.o");
			PA35::run_or_die($a{host}, "$dir/driver.o", "$dir/$label.o", '-o', "$dir/$label");
			$prog{$label} = "$dir/$label";
		}
		PA35::progress("kernels: $name: checking " . (@EDGES + 1) . " iteration counts against the host");
		for my $n (@EDGES, $count) {
			my $want = run_limited($RUN_LIMIT, $prog{host}, 'oracle', $n, $alias);
			my $got = run_limited($RUN_LIMIT, $prog{candidate}, 'actual', $n, $alias);
			die "wrong result for $n iterations: got $got, want $want" if $got ne $want;
		}
		my %work;
		for my $label (qw(host candidate)) {
			PA35::progress("kernels: $name: measuring $label under cachegrind (up to 2 x $MEASURE_LIMIT s)");
			$work{$label} = irefs($prog{$label}, 'actual', $count, $alias) - irefs($prog{$label}, 'actual', 0, $alias);
		}
		$work{candidate} / $work{host};
	};
	if (!defined $result) {
		my $err = $@ || 'unknown error';
		$err =~ s/\s+$//;
		printf "%-18s ERROR %s\n", $name, $err;
		push @rows, "$name\tERROR";
		$failed++;
		next;
	}
	printf "%-18s %.4fx GCC instructions\n", $name, $result;
	push @rows, sprintf("%s\t%.6f", $name, $result);
}
PA35::spit("$out/kernels.tsv", join("\n", "kernel\tratio", @rows) . "\n");
exit($failed ? 1 : 0);
