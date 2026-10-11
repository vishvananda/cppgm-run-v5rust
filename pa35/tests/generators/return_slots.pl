#!/usr/bin/env perl
# return_slots.pl SEED DIR
#
# Writes DIR/callee.cpp: 30 functions returning small structs in rax/rdx
# (value-initialized, constant, copied from memory, field by field, copied
# then changed, escaping to a call, built in a loop, chosen at a join), and
# DIR/main.cpp, which calls each over a grid and prints a hash of every
# field. callee.cpp is built by the compiler under test at each level,
# main.cpp by the host, both linked by the host, and the output compared
# with an all-host build (generated.pl): return forwarding and the frames it
# drops under test. The same SEED always writes the same programs.
use strict;
use warnings;
use File::Path qw(make_path);

my ($seed, $dir) = @ARGV;
die "usage: return_slots.pl SEED DIR\n" unless defined $dir;
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

my @KINDS = ('unsigned char', 'bool', 'unsigned short', 'int', 'unsigned', 'long', 'void*', 'signed char', 'short');
my %SIZE = ('unsigned char' => 1, 'bool' => 1, 'signed char' => 1, 'unsigned short' => 2, 'short' => 2,
	'int' => 4, 'unsigned' => 4, 'long' => 8, 'void*' => 8);

# 1 to 6 field kinds whose struct fits in 16 bytes (returned in rax/rdx).
sub layout {
	while (1) {
		my @fields = map { choice(@KINDS) } 1 .. randint(1, 6);
		my ($off, $align) = (0, 1);
		for my $f (@fields) {
			my $s = $SIZE{$f};
			$off = int(($off + $s - 1) / $s) * $s + $s;
			$align = $s if $s > $align;
		}
		return \@fields if int(($off + $align - 1) / $align) * $align <= 16;
	}
}

# A C++ expression of type $kind computed from $expr.
sub value {
	my ($kind, $expr) = @_;
	return "(void*)(long)($expr)" if $kind eq 'void*';
	return "(($expr) & 1) != 0" if $kind eq 'bool';
	return "($kind)($expr)";
}

my $n_structs = 6;
my @structs = map { layout() } 1 .. $n_structs;
my @head;
for my $i (0 .. $#structs) {
	my $fs = $structs[$i];
	push @head, "struct S$i { " . join(' ', map { "$fs->[$_] f$_;" } 0 .. $#$fs) . ' };';
}
my @callee = (@head, '__attribute__((noinline)) void touch(void* p, long x) { ((unsigned char*)p)[0] ^= (unsigned char)x; }');
my @main = (@head, '#include <cstdio>');
my @calls;
for my $i (0 .. 29) {
	my $s = randint(0, $n_structs - 1);
	my $fs = $structs[$s];
	my $t = "S$s";
	my $shape = randint(0, 8);
	my $sig = "$t g$i($t* p, long a, long b)";
	push @main, "$sig;";
	my @body;
	if ($shape == 0) {
		push @body, "  return $t();";
	} elsif ($shape == 1) {
		push @body, "  $t v = $t();", '  if (a & 1) return v;', '  return p[b & 3];';
	} elsif ($shape == 2) {
		push @body, '  return p[(a + b) & 3];';
	} elsif ($shape == 3) {
		push @body, "  $t v;";
		for my $j (0 .. $#$fs) {
			push @body, "  v.f$j = " . value($fs->[$j], sprintf('a * %d + b + %d', $j + 1, randint(0, 99))) . ';';
		}
		push @body, '  return v;';
	} elsif ($shape == 4) {
		my $j = randint(0, $#$fs);
		push @body, "  $t v = p[a & 3];", "  v.f$j = " . value($fs->[$j], 'b + ' . randint(0, 9)) . ';', '  return v;';
	} elsif ($shape == 5) {
		push @body, "  $t v = p[a & 3];", '  touch(&v, b);', '  return v;';
	} elsif ($shape == 6) {
		my $j = randint(0, $#$fs);
		my $next = value($fs->[$j], $fs->[$j] eq 'void*' ? 'k + b' : "v.f$j + k + b");
		push @body, "  $t v = $t();", '  for (long k = 0; k < (a & 7); ++k) {', "    v.f$j = $next;", '  }', '  return v;';
	} elsif ($shape == 7) {
		push @body, '  if (a > b) return p[0];', "  if (a == b) return $t();", "  $t v = p[1];", '  return v;';
	} else {
		my $j = randint(0, $#$fs);
		push @body, "  $t v;", '  if (a & 2) v = p[2]; else v = p[3];', "  if (b & 1) v.f$j = " . value($fs->[$j], 'a') . ';', '  return v;';
	}
	push @callee, "$sig {", @body, '}';
	push @calls, [$i, $s];
}
push @main, 'int main() {', '  unsigned long sum = 0;';
for my $s (0 .. $n_structs - 1) {
	push @main, "  S$s p$s\[4];";
	for my $e (0 .. 3) {
		for my $j (0 .. $#{ $structs[$s] }) {
			push @main, "  p$s\[$e].f$j = " . value($structs[$s][$j], randint(-300, 300)) . ';';
		}
	}
}
push @main, '  for (long a = 0; a < 9; ++a) for (long b = 0; b < 5; ++b) {';
for my $c (@calls) {
	my ($i, $s) = @$c;
	push @main, "    { S$s v = g$i(p$s, a, b);";
	push @main, "      sum = sum * 31 + (long)v.f$_;" for 0 .. $#{ $structs[$s] };
	push @main, '    }';
}
push @main, '  }', '  printf("%lu\n", sum);', '  return 0;', '}';

make_path($dir);
for my $file (['callee.cpp', \@callee], ['main.cpp', \@main]) {
	open(my $fh, '>', "$dir/$file->[0]") or die "cannot write $dir/$file->[0]: $!\n";
	print $fh join("\n", @{ $file->[1] }), "\n";
	close $fh or die "cannot write $dir/$file->[0]: $!\n";
}
