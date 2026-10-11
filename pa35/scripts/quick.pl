#!/usr/bin/env perl
# quick.pl [--levels O0,O3] [--counter auto|callgrind|perf] [--no-kernels] [--no-behavior]
#
# The fast check for one change: rebuild the seed (`make build`, incremental),
# build self from the same source with it, count both on the frozen workload
# (instructions only: no cache simulation; perf counters when the machine has
# them), run the kernels at reduced iteration counts and the behavior tests,
# and print the result next to the previous quick run's. Unchanged binaries
# are not measured again (pa35/obj/cache). Use `make -C pa35 test` for the
# full gates.
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use File::Copy qw(copy);
use File::Path qw(make_path);
use Getopt::Long;
use PA35;

my %o = (levels => 'O0,O3', counter => 'auto');
GetOptions(\%o, 'levels=s', 'counter=s', 'no-kernels', 'no-behavior')
	or die "usage: quick.pl [--levels O0,O3] [--counter auto|callgrind|perf] [--no-kernels] [--no-behavior]\n";
my $Q = "$PA35::PA35/obj/quick";
make_path($Q);
my $S = $FindBin::Bin;
my $t0 = time;

print "make build ...\n";
PA35::run_or_die('make', '-C', $PA35::ROOT, 'build');
copy("$PA35::ROOT/dev/cppgm++", "$Q/seed") or die "copy: $!\n";
chmod 0755, "$Q/seed";
print "self build ...\n";
my ($self) = PA35::build_with(compiler => "$Q/seed", out => "$Q/self-build");

my %now;
print "measure ($o{counter}) ...\n";
my ($st, $log) = PA35::run_capture($^X, "$S/measure.pl", '--seed', "$Q/seed", '--self', $self,
	'--out', "$Q/measure", '--levels', $o{levels}, '--counter', $o{counter});
die "measure failed:\n$log" if $st;
for my $line (split /\n/, PA35::slurp("$Q/measure/summary.tsv")) {
	my ($level, $metric, $a, $b, $ratio) = split /\t/, $line;
	next if $level eq 'level' || $metric eq 'identical' || $metric eq 'calls';
	$now{"$level $metric ratio"} = $ratio;
	$now{"$level $metric seed"} = $a;
	$now{"$level $metric self"} = $b;
}
unless ($o{'no-kernels'}) {
	print "kernels ...\n";
	(undef, $log) = PA35::run_capture($^X, "$S/kernels.pl", '--compiler', "$Q/seed", '--out', "$Q/kernels", '--scale', 16);
	for my $line (split /\n/, $log) {
		$now{"kernel $1"} = $2 if $line =~ /^(\S+)\s+([\d.]+)x GCC/;
		$now{"kernel $1"} = 'ERROR' if $line =~ /^(\S+)\s+ERROR/;
	}
}
unless ($o{'no-behavior'}) {
	print "behavior ...\n";
	(undef, $log) = PA35::run_capture($^X, "$S/behavior.pl");
	my ($pass, $total) = $log =~ /(\d+)\/(\d+) tests pass/;
	$now{'behavior failures'} = defined $total ? $total - $pass : 'ERROR';
	print grep { /^(DIFF|MIR FAIL|REF FAIL)/ } map { "$_\n" } split /\n/, $log;
}

my %last;
if (-f "$Q/last.tsv") {
	for my $line (split /\n/, PA35::slurp("$Q/last.tsv")) {
		my ($k, $v) = split /\t/, $line;
		$last{$k} = $v;
	}
}
printf "\n%-24s %16s %16s\n", 'quick', 'now', 'previous';
for my $k (sort keys %now) {
	my ($n, $p) = ($now{$k}, $last{$k} // '-');
	my $fmt = sub { $_[0] =~ /^\d+$/ && $_[0] > 1e6 ? PA35::fmt_m($_[0]) : $_[0] =~ /^\d*\.\d+$/ ? sprintf('%.4f', $_[0]) : $_[0] };
	printf "%-24s %16s %16s\n", $k, $fmt->($n), $fmt->($p);
}
PA35::spit("$Q/last.tsv", join('', map { "$_\t$now{$_}\n" } sort keys %now));
printf "\n%d s; previous values kept in %s/last.tsv\n", time - $t0, $Q;
