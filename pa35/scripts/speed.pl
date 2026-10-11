#!/usr/bin/env perl
# speed.pl [--compiler C] [--host g++] [--rounds 3] [--out FILE]
#
# Compiler speed on the frozen workload relative to the host compiler on the
# same input, in three modes: preprocessing (C -E against host -E), C -O0
# against host -O0, and C -O3 against host -O2. Each figure is the minimum
# user time (and peak RSS) over the rounds. The first run of each C mode is
# bounded (wall time max(30 s, 5 x host), 8 GB of address space); a mode
# that exceeds its bound or fails is reported as such and not run again.
# Writes FILE (default pa35/obj/speed.tsv) and prints the ratios.
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Temp qw(tempdir);
use Getopt::Long;
use PA35;

my %o = (compiler => "$PA35::ROOT/dev/cppgm++", host => 'g++', rounds => 3, out => "$PA35::PA35/obj/speed.tsv");
GetOptions(\%o, 'compiler=s', 'host=s', 'rounds=i', 'out=s') or die "usage: speed.pl [--compiler C] [--host g++] [--rounds N]\n";
my $C = abs_path($o{compiler}) // die "no compiler $o{compiler}\n";
PA35::verify_workload();
my $tmp = tempdir('pa35-speed-XXXXXX', TMPDIR => 1, CLEANUP => 1);
my $MEMORY_KB = 8 * 1024 * 1024;

# mode => [C level, host level]
my %MODES = (E => ['-E', '-E'], O0 => ['-O0', '-O0'], O3 => ['-O3', '-O2']);
my @order = qw(E O0 O3);
my (%user, %rss, %state);
my $run = sub {
	my ($who, $mode, $opt) = @_;
	my ($cxx, $level) = $who eq 'host' ? ($o{host}, $MODES{$mode}[1]) : ($C, $MODES{$mode}[0]);
	PA35::progress("speed: $who $level on the workload" . ($opt->{timeout} ? " (limit " . sprintf("%.0f", $opt->{timeout}) . " s)" : ''));
	my $t = PA35::timed_run({ %$opt, allow_fail => 1 }, $cxx, '-std=gnu++11', PA35::workload_args($level, "$tmp/$who-$mode.out"));
	my $key = "$who-$mode";
	if ($t->{status} ne '0') {
		$state{$key} = $t->{status} eq 'timeout' ? 'timeout' : 'failed';
		return;
	}
	$user{$key} = $t->{user} if !defined $user{$key} || $t->{user} < $user{$key};
	$rss{$key} = $t->{rss_kb} if !defined $rss{$key} || $t->{rss_kb} < $rss{$key};
};
for my $mode (@order) {
	$run->('host', $mode, {});
	die "host compiler failed on the workload in mode $mode\n" if $state{"host-$mode"};
	my $limit = 5 * $user{"host-$mode"};
	$limit = 30 if $limit < 30;
	$run->('cppgm', $mode, { timeout => $limit, memory_kb => $MEMORY_KB });
	$state{"limit-$mode"} = $limit;
}
for my $round (2 .. $o{rounds}) {
	for my $mode (@order) {
		next if $state{"cppgm-$mode"};
		$run->('host', $mode, {});
		$run->('cppgm', $mode, {});
	}
}
my @rows = ("metric\tcppgm\thost\tratio");
for my $mode (@order) {
	my ($c, $h) = ("cppgm-$mode", "host-$mode");
	my $host_label = $MODES{$mode}[1];
	if (my $why = $state{$c}) {
		my $shown = $why eq 'timeout' ? sprintf('over %d s', $state{"limit-$mode"}) : 'failed';
		printf "%-4s cppgm++ %s (host %s %.3fs)\n", $mode, $shown, $host_label, $user{$h};
		push @rows, "time-$mode\t$why\t$user{$h}\t$why";
		push @rows, "rss-$mode\t$why\t$rss{$h}\t$why";
		next;
	}
	printf "%-4s cppgm++ %.3fs (%d MB), host %s %.3fs (%d MB): time %.3fx, rss %.3fx\n",
		$mode, $user{$c}, $rss{$c} / 1024, $host_label, $user{$h}, $rss{$h} / 1024,
		$user{$c} / $user{$h}, $rss{$c} / $rss{$h};
	push @rows, sprintf("time-%s\t%.6f\t%.6f\t%.6f", $mode, $user{$c}, $user{$h}, $user{$c} / $user{$h});
	push @rows, sprintf("rss-%s\t%d\t%d\t%.6f", $mode, $rss{$c}, $rss{$h}, $rss{$c} / $rss{$h});
}
PA35::spit($o{out}, join("\n", @rows) . "\n");
