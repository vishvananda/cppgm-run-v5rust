#!/usr/bin/env perl
# booleans.pl SEED > out.cpp
#
# 40 functions computing Booleans from compares (some through a phi) and
# combining them (and, or, xor, not, equality), then reading them as a
# branch, a call argument, a byte store, a compare, arithmetic, a
# loop-carried flip, a choice or a return; main calls each over a grid and
# prints a hash. The whole program is built by the compiler under test at
# each level and its output compared with a host build (generated.pl).
# The same SEED always writes the same program.
use strict;
use warnings;

my $seed = shift // die "usage: booleans.pl SEED > out.cpp\n";
die "SEED must be a non-negative integer\n" unless $seed =~ /^\d+$/;

# xorshift32 seeded from SEED.
my $state = ((($seed % 4294967296) * 2654435761) ^ 0x9e3779b9) & 0xffffffff || 1;
sub next32 {
	$state ^= ($state << 13) & 0xffffffff;
	$state ^= $state >> 17;
	$state ^= ($state << 5) & 0xffffffff;
	return $state;
}
sub randint { my ($lo, $hi) = @_; return $lo + next32() % ($hi - $lo + 1); }
sub choice { return $_[next32() % @_]; }
next32() for 1 .. 8;

sub cond {
	my $v = choice('a', 'b', 'c', 'p[a & 3]', 'p[b & 3]');
	my $w = choice('a', 'b', 'c', randint(-5, 9), 'p[c & 3]');
	my $op = choice('<', '<=', '==', '!=', '>', '>=');
	return "($v $op $w)";
}

# The ways a function reads its Booleans x, y and z.
my @USES = (
	'  return z ? a : b;',
	'  return take(z, a) + take(y, b);',
	'  sink[si++ & 63] = z; sink[si++ & 63] = y; return sink[(si - 1) & 63] + 2 * sink[(si - 2) & 63];',
	'  return (z == y) ? 100 : (x == z) ? 200 : 300;',
	'  return a + z + 2 * y + 4 * x;',
	'  return take2(c, z, y) + z;',
	'  long s = 0; for (long k = 0; k < (a & 3); ++k) { if (z) s += k; z = !z; } return s + z;',
	'  bool w = z; if (b > 2) w = y; return take(w, c) + (w ? 1 : 0);',
	'  return (long)(unsigned char)z * 1000 + (long)(signed char)y;',
	'  if (z) return 1; if (y) return 2; return x ? 3 : 4;',
);

my $n = 40;
my @out = (
	'static bool sink[64]; static int si = 0;',
	'__attribute__((noinline)) long take(bool b, long x) { return b ? x * 3 : x + 5; }',
	'__attribute__((noinline)) long take2(long x, bool a, bool b) { return (a ? 7 : 0) + (b ? 11 : 0) + x; }',
);
for my $i (0 .. $n - 1) {
	push @out, "__attribute__((noinline)) long f$i(long a, long b, long c, long* p) {";
	push @out, '  bool x = ' . cond() . ';';
	push @out, '  bool y;';
	my $test = cond();
	my $then = cond();
	my $else = choice('true', 'false', cond());
	push @out, "  if ($test) y = $then; else y = $else;";
	push @out, '  bool z = ' . choice('x && y', 'x || y', 'x != y', 'x == y', 'x & y', 'x | y', 'x ^ y', '!x', cond()) . ';';
	push @out, $USES[randint(0, $#USES)];
	push @out, '}';
}
push @out, '#include <cstdio>';
push @out, 'int main() { long p[4] = { 3, -2, 7, 0 }; unsigned long sum = 0;';
push @out, '  for (long a = -3; a < 6; ++a) for (long b = -2; b < 5; ++b) for (long c = -1; c < 3; ++c) {';
push @out, "    sum = sum * 31 + f$_(a, b, c, p);" for 0 .. $n - 1;
push @out, '  }';
push @out, '  printf("%lu\n", sum); return 0; }';
print join("\n", @out), "\n";
