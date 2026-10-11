#!/usr/bin/env perl
# profile.pl [--compiler C] [--level -O0] [--input FILE] [--seconds 30] [--exact] [--out DIR]
#
# Where a compiler spends its time on the frozen workload (or on FILE, for
# example a slow scaling.pl source kept with --keep). By default it samples
# with `perf record -e cpu-clock` at native speed, stopping after --seconds
# so a compile that blows up still yields a profile; it prints the hottest
# functions and the share of time per binary and library (allocator, libc,
# libstdc++). --exact runs callgrind instead (about 30 times slower, bounded
# by --seconds x 30): exact self and inclusive costs and the number of calls
# to the allocation functions. Reports go to DIR (default pa35/obj/profile).
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Path qw(make_path);
use Getopt::Long;
use PA35;

my %o = (compiler => "$PA35::ROOT/dev/cppgm++", level => '-O0', seconds => 30, out => "$PA35::PA35/obj/profile", top => 40);
GetOptions(\%o, 'compiler=s', 'level=s', 'input=s', 'seconds=i', 'exact', 'out=s', 'top=i')
	or die "usage: profile.pl [--compiler C] [--level -O0] [--input FILE] [--seconds 30] [--exact] [--out DIR]\n";
my $C = abs_path($o{compiler}) // die "no compiler $o{compiler}\n";
make_path($o{out});
my $out = abs_path($o{out});
my @argv;
if ($o{input}) {
	@argv = ($C, '-std=c++11', $o{level}, '-c', abs_path($o{input}), '-o', "$out/input.o");
} else {
	PA35::verify_workload();
	@argv = ($C, PA35::workload_args($o{level}, "$out/workload.o"));
}

if (!$o{exact}) {
	my @rec = ('timeout', '--signal=INT', '--kill-after=10', $o{seconds}, 'perf', 'record', '-e', 'cpu-clock',
		'-F', '2000', '-o', "$out/perf.data", '--', @argv);
	my ($st, $log) = PA35::run_capture(@rec);
	PA35::spit("$out/record.log", $log);
	die "perf record produced no data (see $out/record.log); try --exact\n" unless -s "$out/perf.data";
	print "stopped after $o{seconds} s (partial profile)\n" if $st == 124 || $log =~ /Interrupt|terminated/;
	my $dso = PA35::run_or_die('perf', 'report', '-i', "$out/perf.data", '--stdio', '--sort', 'dso', '--percent-limit', '0.5');
	my $sym = PA35::run_or_die('perf', 'report', '-i', "$out/perf.data", '--stdio', '--no-children', '--sort', 'dso,symbol', '--percent-limit', '0.2');
	PA35::spit("$out/dso.txt", $dso);
	PA35::spit("$out/symbols.txt", $sym);
	my @d = map { s/\s+$//r } grep { /^\s+\d+\.\d+%/ } split /\n/, $dso;
	my @s = map { s/\s+$//r } grep { /^\s+\d+\.\d+%/ } split /\n/, $sym;
	splice(@s, $o{top}) if @s > $o{top};
	print "time by binary and library:\n", map({ "$_\n" } @d), "\nhottest functions (self time):\n", map({ "$_\n" } @s);
	print "\nfull reports: $out/dso.txt, $out/symbols.txt\n";
	exit 0;
}

my @cg = ('timeout', '--signal=INT', '--kill-after=10', $o{seconds} * 30, 'valgrind', '--tool=callgrind',
	"--callgrind-out-file=$out/callgrind.out", @argv);
my ($st, $log) = PA35::run_capture(@cg);
PA35::spit("$out/callgrind.log", $log);
die "callgrind produced no profile (see $out/callgrind.log)\n" unless -s "$out/callgrind.out";
print "stopped early (partial profile)\n" if $st;
my $p = PA35::parse_callgrind("$out/callgrind.out", $C);
my $total = $p->{totals}{Ir} || 1;
print "instructions by binary and library:\n";
for my $ob (sort { $p->{objects}{$b} <=> $p->{objects}{$a} } keys %{ $p->{objects} }) {
	my $share = 100 * $p->{objects}{$ob} / $total;
	printf "  %6.2f%%  %s\n", $share, $ob if $share >= 0.5;
}
# Calls into the allocator, counted from every caller in the compiler binary.
my %alloc = map { $_ => 1 } ('malloc', 'free', 'calloc', 'realloc', 'operator new(unsigned long)',
	'operator new[](unsigned long)', 'operator delete(void*)', 'operator delete(void*, unsigned long)',
	'operator delete[](void*)');
my $allocs = 0;
$allocs += $p->{calls}{$_} // 0 for keys %alloc;
printf "\ncalls from the compiler into malloc/free/new/delete: %d\n", $allocs;
my $self = PA35::run_or_die('callgrind_annotate', '--threshold=99', "$out/callgrind.out");
my $incl = PA35::run_or_die('callgrind_annotate', '--inclusive=yes', '--threshold=99', "$out/callgrind.out");
PA35::spit("$out/self.txt", $self);
PA35::spit("$out/inclusive.txt", $incl);
for my $pair (['self cost', $self], ['inclusive cost', $incl]) {
	my @lines = grep { /^\s*[\d,]+ \(/ } split /\n/, $pair->[1];
	splice(@lines, $o{top}) if @lines > $o{top};
	print "\nhottest functions by $pair->[0]:\n", map { "$_\n" } @lines;
}
print "\nfull reports: $out/self.txt, $out/inclusive.txt\n";
