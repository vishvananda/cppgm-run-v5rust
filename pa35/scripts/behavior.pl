#!/usr/bin/env perl
# behavior.pl [--compiler C] [--host g++] [--levels O0,O1,O2,O3] [--jobs N]
#             [--lowir2native L] [TEST...]
#
# Each pa35/tests/behavior/*.cpp is built by the host compiler (reference)
# and by C at every level (C compiles, the host links), and each build must
# print exactly what the reference prints. At -O3 the program's LowIR must
# also lower through lowir2native (L, default the one beside C) with a
# machine-IR dump. Prints one line per failure and a summary; exits nonzero
# on any failure.
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Basename qw(basename dirname);
use File::Temp qw(tempdir);
use Getopt::Long;
use PA35;

my %a = (compiler => "$PA35::ROOT/dev/cppgm++", host => 'g++', levels => 'O0,O1,O2,O3', jobs => PA35::default_jobs());
GetOptions(\%a, 'compiler=s', 'host=s', 'levels=s', 'jobs=i', 'lowir2native=s')
	or die "usage: behavior.pl [--compiler C] [--levels ...] [--lowir2native L] [TEST...]\n";
my $compiler = abs_path($a{compiler}) // die "no compiler $a{compiler}\n";
my $lowir2native = $a{lowir2native} // dirname($compiler) . '/lowir2native';
my @tests = @ARGV ? map { abs_path($_) } @ARGV : sort glob("$PA35::PA35/tests/behavior/*.cpp");
my @levels = split /,/, $a{levels};
my $tmp = tempdir('pa35-behavior-XXXXXX', TMPDIR => 1, CLEANUP => 1);

# One shell job per test; it prints "DIFF <test> <level>" lines on failure.
my @jobs;
for my $src (@tests) {
	my $name = basename($src, '.cpp');
	my $t = "$tmp/$name";
	my $cmd = "mkdir -p '$t' && cd '$t' && '$a{host}' -O1 -o ref '$src' && timeout 10 ./ref > ref.out || { echo 'REF FAIL $name'; exit 1; }; fail=0; ";
	for my $o (@levels) {
		$cmd .= "if timeout 300 '$compiler' -$o -c -o $o.o '$src' && '$a{host}' -o $o.bin $o.o && timeout 10 ./$o.bin > $o.out && cmp -s ref.out $o.out; then :; else echo 'DIFF $name $o'; fail=1; fi; ";
	}
	if (-x $lowir2native && grep { $_ eq 'O3' } @levels) {
		$cmd .= "if timeout 300 '$compiler' -O3 --emit-lowir -o p.lowir '$src' && timeout 300 '$lowir2native' -O3 --dump-machine-ir p.mir p.lowir > /dev/null; then :; else echo 'MIR FAIL $name'; fail=1; fi; ";
	}
	$cmd .= 'exit $fail';
	push @jobs, $cmd;
}
$PA35::SHOW_FAILED_COMMANDS = 0;
my $failures = PA35::run_parallel($a{jobs}, @jobs);
my $total = scalar @tests;
printf "behavior: %d/%d tests pass at %s\n", $total - $failures, $total, join(' ', @levels);
exit($failures ? 1 : 0);
