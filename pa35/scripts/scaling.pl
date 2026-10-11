#!/usr/bin/env perl
# scaling.pl [--compiler C] [--host g++ | --no-host] [--family NAME,...] [--keep DIR] [--out FILE]
#
# Compile-time complexity probe. Each family generates C++11 sources of
# growing size n, 2n, 4n, 8n (and an empty base for fixed costs), compiles
# them with C and with the host, and reports how the time above the base
# grows from one size to the next: about 2 for linear work, about 4 for
# quadratic. The families exercise work that should stay linear in a
# compiler: many scopes and lookups, a class with many members, many template
# instantiations, many macros, one long function (at -O3) and many small
# functions (at -O3). Each figure is the minimum user time of three runs;
# each run is bounded (30 s, 8 GB). A family stops after a size that takes
# more than 10 s, and its growth is the last doubling's; a size that exceeds
# the bound or fails makes the family's growth unbounded (`inf` or
# `failed`), whatever the earlier sizes showed. --keep DIR writes the
# generated sources there, for profiling the slowest one. Writes FILE
# (default pa35/obj/scaling.tsv); the largest growth over the families is
# the `scaling` gate's value.
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Path qw(make_path);
use File::Temp qw(tempdir);
use Getopt::Long;
use PA35;

my %o = (compiler => "$PA35::ROOT/dev/cppgm++", host => 'g++', out => "$PA35::PA35/obj/scaling.tsv", family => '');
GetOptions(\%o, 'compiler=s', 'host=s', 'family=s', 'keep=s', 'out=s', 'no-host')
	or die "usage: scaling.pl [--compiler C] [--host g++] [--family NAME,...] [--keep DIR] [--out FILE]\n";
my $C = abs_path($o{compiler}) // die "no compiler $o{compiler}\n";
my $dir = $o{keep} // tempdir('pa35-scaling-XXXXXX', TMPDIR => 1, CLEANUP => 1);
make_path($dir);
my $LIMIT = 30;      # seconds per compile
my $ENOUGH = 10;     # a family stops after a size that takes longer than this
my $MEMORY_KB = 8 * 1024 * 1024;
my $NOISE = 0.1;     # seconds above the base below which growth is not judged

# name => [level, base size n, generator(n) returning source text]
my %FAMILY = (
	scopes => ['-O0', 500, sub {
		my ($n) = @_;
		my $s = "#include <vector>\n";
		for my $i (1 .. $n) {
			$s .= "namespace n$i { struct S { int v; std::vector<int> xs; int get() const { return v + (int)xs.size(); } };\n"
				. "  inline int f(const S& s) { return s.get(); } inline int f(int x) { return x + $i; }\n"
				. "  template <class T> T twice(T t) { return t + t; } }\n";
		}
		$s .= "int run() { int s = 0;\n";
		$s .= " { n$_\::S a; a.v = $_; s += n$_\::f(a) + n$_\::f($_) + n$_\::twice(s); }\n" for 1 .. $n;
		return $s . " return s; }\n";
	}],
	members => ['-O0', 300, sub {
		my ($n) = @_;
		my $s = "struct Big {\n";
		$s .= " int m$_; int f$_() const { return m$_ + $_; } void g$_(int x) { m$_ = x; }\n" for 1 .. $n;
		$s .= "};\nint run(Big& b) { int s = 0;\n";
		$s .= " b.g$_(s); s += b.f$_() + b.m$_;\n" for 1 .. $n;
		return $s . " return s; }\n";
	}],
	templates => ['-O0', 200, sub {
		my ($n) = @_;
		my $s = "template <int I> struct T { int v; int get() const { return v + I; } T<I> next() const { T<I> t; t.v = v + 1; return t; } };\n"
			. "template <class X> X twice(X x) { return x + x; }\nint run() { int s = 0;\n";
		$s .= " { T<$_> t; t.v = s; s += t.next().get() + twice(s); }\n" for 1 .. $n;
		return $s . " return s; }\n";
	}],
	macros => ['-O0', 1000, sub {
		my ($n) = @_;
		my $s = '';
		$s .= "#define M$_(x) ((x) + $_)\n#define N$_(x) M$_(M$_(x) * 2)\n" for 1 .. $n;
		$s .= "int run() { int s = 0;\n";
		$s .= " s += N$_(s);\n" for 1 .. $n;
		return $s . " return s; }\n";
	}],
	statements => ['-O3', 200, sub {
		my ($n) = @_;
		my $s = "unsigned run(unsigned x, unsigned y) {\n";
		$s .= " x = x * 3u + ${_}u; if (x & 1u) y += x >> 2; else y ^= x + ${_}u;\n" for 1 .. $n;
		return $s . " return x + y; }\n";
	}],
	functions => ['-O3', 200, sub {
		my ($n) = @_;
		my $s = "static unsigned f0(unsigned x) { return x + 1u; }\n";
		for my $i (1 .. $n) {
			my $j = int($i / 2);
			$s .= "static unsigned f$i(unsigned x) { return x & 1u ? f" . ($i - 1) . "(x * 3u) : f$j(x + $i\u); }\n";
		}
		return $s . "unsigned run(unsigned x) { return f$n(x); }\n";
	}],
);
my @families = $o{family} ? split /,/, $o{family} : sort keys %FAMILY;

# Minimum user time of three runs, or a failure status.
sub timed {
	my ($cxx, $level, $src) = @_;
	my $best;
	for (1 .. 3) {
		my $t = PA35::timed_run({ timeout => $LIMIT, memory_kb => $MEMORY_KB, allow_fail => 1 },
			$cxx, '-std=c++11', $level, '-c', $src, '-o', "$src.o");
		return $t->{status} eq 'timeout' ? 'timeout' : 'failed' if $t->{status} ne '0';
		$best = $t->{user} if !defined $best || $t->{user} < $best;
		last if $t->{user} > 5;    # long runs are stable enough once
	}
	return $best;
}

my @rows = ("family\tcompiler\tgrowth\ttimes");
my $worst = 0;
for my $f (@families) {
	my ($level, $n0, $gen) = @{ $FAMILY{$f} // die "unknown family $f\n" };
	my @sizes = (0, map { $n0 * $_ } 1, 2, 4, 8);
	my %growth;
	for my $who (['cppgm', $C], $o{'no-host'} ? () : (['host', $o{host}])) {
		my ($label, $cxx) = @$who;
		PA35::progress("scaling: $f $label $level, sizes @sizes");
		my (@times, $stop);
		for my $n (@sizes) {
			my $src = "$dir/$f-$n.cpp";
			PA35::spit($src, $gen->($n)) unless -f $src;
			my $t = timed($cxx, $level, $src);
			push @times, $t;
			if ($t !~ /^[\d.]+$/) { $stop = $t; last; }
			last if $t > $ENOUGH;
		}
		my $g = PA35::scaling_growth(\@times, $NOISE);
		$growth{$label} = $g;
		my $shown = join(' ', map { /^[\d.]+$/ ? sprintf('%.2f', $_) : $_ } @times);
		printf "%-11s %-6s %-5s growth %-8s times (n = %s): %s\n", $f, $label, $level, $g,
			join(',', @sizes[0 .. $#times]), $shown;
		push @rows, "$f\t$label\t$g\t$shown";
		if ($label eq 'cppgm') {
			my $v = $g =~ /^[\d.]+$/ ? $g : $g eq 'flat' ? 0 : 1e9;
			$worst = $v if $v > $worst;
		}
	}
}
push @rows, sprintf("worst\tcppgm\t%s\t", $worst >= 1e9 ? 'inf' : sprintf('%.2f', $worst));
PA35::spit($o{out}, join("\n", @rows) . "\n");
printf "largest growth per doubling: %s\n", $worst >= 1e9 ? 'unbounded (timeout or failure)' : sprintf('%.2f', $worst);
