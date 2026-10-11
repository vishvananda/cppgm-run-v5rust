#!/usr/bin/env perl
# shrink_wrap_eh.pl SEED > out.cpp
#
# 40 functions with frameless fast paths and slow paths that call a thrower
# (shapes: early return, nested tests, a loop, a switch, a loop of calls,
# values live across calls past the callee-saved registers, a call-free
# path naming more registers than the renames have), and a driver catching
# what propagates through them with destructors in between. The whole
# program is built by the compiler under test at each level and its output
# compared with a host build (generated.pl): the unwind tables of
# shrink-wrapped functions under test. The same SEED always writes the
# same program.
use strict;
use warnings;

my $seed = shift // die "usage: shrink_wrap_eh.pl SEED > out.cpp\n";
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
next32() for 1 .. 8;

# Body lines of one function from its constants and shape (0..6; shape 3
# is the switch).
sub body {
	my ($k1, $k2, $k3, $shape) = @_;
	if ($shape == 0) {
		return (
			sprintf('  if (a %% %d == %d) return p[0] + b;', $k3 + 1, $k1 % ($k3 + 1)),
			'  long t = thrower(a, b) + p[1];',
			"  return t + a * $k2;",
		);
	}
	if ($shape == 1) {
		return (
			"  if (b < $k1) { if (a & 1) return a - b; return p[a & 1] * 2; }",
			sprintf('  if (a > %d) { long u = thrower(a, %d); return u + b + p[0]; }', $k2 * 10, $k1),
			"  long v = thrower(b, a) * $k3;",
			'  return v + p[1] + a;',
		);
	}
	if ($shape == 2) {
		return (
			'  long s = 0;',
			'  for (long j = 0; j < (a & 7); ++j) s += p[j & 1] ^ j;',
			sprintf('  if (s %% %d != 0) return s + b;', $k3 + 1),
			'  G g((int)a); s += thrower(s, b);',
			'  return s + g.v;',
		);
	}
	if ($shape == 4) {
		return (
			"  if (p[a & 1] == a + $k1) return b - a;",
			'  long s = b;',
			'  for (long j = 0; j < (a & 3) + 1; ++j) s += thrower(a + j, s & 15) + p[j & 1];',
			"  return s + a * $k3;",
		);
	}
	if ($shape == 5) {
		return (
			sprintf('  if ((a & 3) == %d) return p[a & 1] + b;', $k3 % 4),
			"  long c = p[0] + a, d = p[1] - b, e = a * $k1, g = a ^ b, h = a + $k2, q = b * $k3;",
			'  long t = thrower(a, b);',
			'  long u = thrower(c & 31, d & 31);',
			'  return t + u + c + d + e + g + h + q + a + b + p[1];',
		);
	}
	if ($shape == 6) {
		return (
			sprintf('  if ((a & 7) == %d) return b;', $k1 % 8),
			sprintf('  long c = p[0] + a, d = p[1] + b, e = a ^ b, g = a * %d, h = b * %d, q = a - b;', $k2 + 2, $k3 + 2),
			sprintf('  if ((a & 3) == %d) return c + d * 2 + e * 3 + g * 5 + h * 7 + q * 11;', $k3 % 4),
			'  long t = thrower(c & 31, d & 31);',
			'  return t + c + d + e + g + h + q + a + b;',
		);
	}
	return (
		"  switch (a & 3) { case 0: return b + $k1; case 1: return p[0] - b; default: break; }",
		"  long w = thrower(a + $k2, b) + thrower(b, a + $k1);",
		'  return w;',
	);
}

my $n = 40;
my @out = (
	'#include <cstdio>',
	'static long acc = 0;',
	'struct G { int v; G(int x) : v(x) { acc += x; } ~G() { acc -= v; } };',
	'__attribute__((noinline)) long thrower(long x, long y) { if ((x ^ y) % 7 == 3) throw (int)(x + y); return x * 3 + y; }',
);
for my $i (0 .. $n - 1) {
	my ($k1, $k2, $k3) = (randint(0, 9), randint(0, 9), randint(1, 5));
	my $shape = randint(0, 6);
	push @out, "__attribute__((noinline)) long f$i(long a, long b, long* p) {", body($k1, $k2, $k3, $shape), '}';
}
push @out, 'int main() { long p[2] = { 13, 29 }; long total = 0; int caught = 0;';
push @out, '  for (long a = 0; a < 60; ++a) for (long b = 0; b < 12; ++b) {';
push @out, "    for (int i = 0; i < $n; ++i) { try { G g(1); switch (i) {";
push @out, "      case $_: total += f$_(a + i, b, p); break;" for 0 .. $n - 1;
push @out, '    } } catch (int e) { caught += e & 255; } }';
push @out, '  }';
push @out, '  printf("%ld %d %ld\n", total, caught, acc); return 0; }';
print join("\n", @out), "\n";
