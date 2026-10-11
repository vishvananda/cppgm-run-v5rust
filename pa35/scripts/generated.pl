#!/usr/bin/env perl
# generated.pl [--compiler C] [--host g++] [--seeds N] [--levels O0,O1,O2,O3] [--jobs J]
#
# ABI programs, fixed and generated from seeds 1..N (default 20). Each is
# built by the host compiler (reference) and by C at every level (C
# compiles the code under test, the host compiles the rest and links), and
# each build must print exactly what the reference prints:
#   narrow          tests/byte_values/narrow_callee.cpp, called through
#                   pointers that pass junk above each parameter's width
#   results         tests/byte_values/results_use.cpp, reading one-byte call
#                   results with junk above their byte (results_junk.s)
#   booleans        tests/byte_values/booleans.pl SEED, the whole program
#   return_slots    tests/generators/return_slots.pl SEED, callee.cpp
#   shrink_wrap_eh  tests/generators/shrink_wrap_eh.pl SEED, the whole program
# Prints "DIFF <family> <seed> <level>" per failing build (seed "-" for the
# fixed programs) and a summary; exits nonzero on any failure.
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Temp qw(tempdir);
use Getopt::Long;
use PA35;

my %a = (compiler => "$PA35::ROOT/dev/cppgm++", host => 'g++', seeds => 20, levels => 'O0,O1,O2,O3', jobs => PA35::default_jobs());
GetOptions(\%a, 'compiler=s', 'host=s', 'seeds=i', 'levels=s', 'jobs=i')
	or die "usage: generated.pl [--compiler C] [--host g++] [--seeds N] [--levels O0,O1,O2,O3] [--jobs J]\n";
my $compiler = abs_path($a{compiler}) // die "no compiler $a{compiler}\n";
die "no compiler $a{compiler}\n" unless -x $compiler;
my @levels = split /,/, $a{levels};
/^O\w+$/ or die "bad level $_\n" for @levels;
my $BV = "$PA35::PA35/tests/byte_values";
my $GEN = "$PA35::PA35/tests/generators";
my $tmp = tempdir('pa35-generated-XXXXXX', TMPDIR => 1, CLEANUP => 1);
my $host = "'$a{host}'";
my $perl = "'$^X'";

# Seconds before a compile or a run counts as a failure.
my $COMPILE_LIMIT = 300;
my $RUN_LIMIT = 10;

# Shell text that builds level $o (C compiles $src to $o.o, the host links
# it with @objs), runs it and compares its output with ref.out.
sub level {
	my ($tag, $o, $src, @objs) = @_;
	my $d = "DIFF $tag $o";
	return "if timeout $COMPILE_LIMIT '$compiler' -$o -c -o $o.o '$src' > $o.log 2>&1; then "
		. "if $host -o $o.bin $o.o @objs > $o.log 2>&1; then "
		. "timeout $RUN_LIMIT ./$o.bin > $o.out 2> $o.log; st=\$?; "
		. "if [ \$st != 0 ]; then echo \"$d (exit status \$st)\"; head -n 3 $o.log; fail=1; "
		. "elif ! cmp -s ref.out $o.out; then echo '$d'; fail=1; fi; "
		. "else echo \"$d (link failed, status \$?)\"; head -n 5 $o.log; fail=1; fi; "
		. "else echo \"$d (compile failed, status \$?)\"; head -n 5 $o.log; fail=1; fi; ";
}

# One shell job per program: $setup writes the program and builds ./ref,
# then each level is built from $src and linked with @objs.
my @jobs;
sub program {
	my ($family, $seed, $setup, $src, @objs) = @_;
	my $tag = "$family $seed";
	my $dir = "$tmp/$family-$seed";
	my $cmd = "mkdir -p '$dir' && cd '$dir' && $setup && timeout $RUN_LIMIT ./ref > ref.out || { echo 'REF FAIL $tag'; exit 1; }; fail=0; ";
	$cmd .= level($tag, $_, $src, @objs) for @levels;
	push @jobs, $cmd . 'exit $fail';
}

program('narrow', '-', "$host -O1 -c -o main.o '$BV/narrow_main.cpp' && $host -O1 -c -o callee.o '$BV/narrow_callee.cpp' && $host -o ref main.o callee.o",
	"$BV/narrow_callee.cpp", 'main.o');
program('results', '-', "$host -O1 -o ref '$BV/results_use.cpp' '$BV/results_ref.cpp' && $host -c -o junk.o '$BV/results_junk.s'",
	"$BV/results_use.cpp", 'junk.o');
for my $s (1 .. $a{seeds}) {
	program('booleans', $s, "$perl '$BV/booleans.pl' $s > p.cpp && $host -O1 -o ref p.cpp", 'p.cpp');
	program('return_slots', $s, "$perl '$GEN/return_slots.pl' $s . && $host -O1 -c -o main.o main.cpp && $host -O1 -c -o callee.o callee.cpp && $host -o ref main.o callee.o",
		'callee.cpp', 'main.o');
	program('shrink_wrap_eh', $s, "$perl '$GEN/shrink_wrap_eh.pl' $s > p.cpp && $host -O1 -o ref p.cpp", 'p.cpp');
}

$PA35::SHOW_FAILED_COMMANDS = 0;
my $failures = PA35::run_parallel($a{jobs}, @jobs);
my $total = scalar @jobs;
printf "generated: %d/%d programs pass at %s\n", $total - $failures, $total, join(' ', @levels);
exit($failures ? 1 : 0);
