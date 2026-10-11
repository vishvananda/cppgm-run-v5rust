#!/usr/bin/env perl
# check.pl [--group correctness|speed|codegen|all] [--seed A --self B [--tools C]]
#          [--lowir2native L] [--out DIR] [--seeds N] [--no-cache]
#
# Runs the PA35 gates in pa35/gates.txt and prints one line per gate with the
# measured value, the limit and PASS/FAIL, then the summary line the course
# runners print. The gates form three groups, run in this order:
#   correctness  the seed compiles the workload at -O0 and -O3 (bounded at
#                600 s each); behavior.pl; generated.pl
#   speed        speed.pl (compile time and memory against the host);
#                scaling.pl (growth of compile time on synthetic inputs)
#   codegen      measure.pl --sim on a seed/self pair (identical objects,
#                own-binary instructions, cycle proxy, text size); kernels.pl
# --group all (the default) stops after the first group with a failing gate
# and reports the later gates as not run.
#
# The seed is dev/cppgm++ after `make build`. The codegen group freezes the
# current tree first (freeze.pl --label check: self = the same source built by
# the seed, linked the same way) unless --seed and --self are given. The
# behavior suite's MIR dump uses the lowir2native beside the compiler it
# runs, or L. Results and logs go to DIR (default pa35/obj/check);
# DIR/report-<group>.txt holds each table and DIR/values-<group>.tsv each
# group's measurements, which progress.pl reads.
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Basename qw(dirname);
use File::Copy qw(copy);
use File::Path qw(make_path);
use Getopt::Long;
use PA35;

my %o = (out => "$PA35::PA35/obj/check", seeds => 20, group => 'all');
GetOptions(\%o, 'group=s', 'seed=s', 'self=s', 'tools=s', 'lowir2native=s', 'out=s', 'seeds=i', 'no-cache')
	&& $o{group} =~ /^(correctness|speed|codegen|all)$/
	or die "usage: check.pl [--group correctness|speed|codegen|all] [--seed A --self B [--tools C]] [--lowir2native L] [--out DIR] [--seeds N] [--no-cache]\n";
make_path($o{out});
my $out = abs_path($o{out});
my $S = $FindBin::Bin;
# Above this compile-time ratio to the host, the simulated runs are skipped.
my $SIM_LIMIT = 1.0;

my @GATES;
for my $line (split /\n/, PA35::slurp("$PA35::PA35/gates.txt")) {
	next if $line =~ /^\s*(#|$)/;
	my ($group, $name, $metric, $op, $limit) = split ' ', $line;
	push @GATES, { group => $group, name => $name, metric => $metric, op => $op, limit => $limit };
}
my @GROUPS = $o{group} eq 'all' ? qw(correctness speed codegen) : ($o{group});

# The compiler under test: an explicit seed, or dev/cppgm++ built now.
my $seed;
if ($o{seed}) {
	$seed = abs_path($o{seed});
} else {
	print "make build ...\n";
	my ($st, $log) = PA35::run_capture('make', '-C', $PA35::ROOT, 'build');
	PA35::spit("$out/build.log", $log);
	die "make build failed (see $out/build.log)\n" if $st;
	make_path("$PA35::PA35/obj/bin");
	$seed = "$PA35::PA35/obj/bin/seed-check";
	copy("$PA35::ROOT/dev/cppgm++", $seed) or die "copy: $!\n";
	chmod 0755, $seed;
}
# behavior.pl and generated.pl run dev/cppgm++ unless --tools names another
# compiler; the behavior suite's MIR dump runs the lowir2native beside it
# unless --lowir2native names one (a seed built elsewhere has none beside it).
my $tools = abs_path($o{tools} // ($o{seed} ? $seed : "$PA35::ROOT/dev/cppgm++"));
my $lowir2native = $o{lowir2native} ? abs_path($o{lowir2native}) : dirname($tools) . '/lowir2native';

my (%value, %note);
sub step {
	my ($name, @cmd) = @_;
	print "$name ...\n";
	my ($st, $log) = PA35::run_capture(@cmd);
	PA35::spit("$out/$name.log", $log);
	return ($st, $log);
}
sub suite_failures {
	my ($suite, $st, $log) = @_;
	my ($pass, $total) = $log =~ /(\d+)\/(\d+) \w+ pass/;
	$value{"$suite-failures"} = defined $total ? $total - $pass : 'error';
	my @fails = grep { /^(DIFF|MIR FAIL|REF FAIL)/ } split /\n/, $log;
	my $more = @fails > 6 ? sprintf('; %d more in %s/%s.log', @fails - 6, $out, $suite) : '';
	splice(@fails, 6) if @fails > 6;
	$note{"$suite-failures"} = join('; ', @fails) . $more;
	$note{"$suite-failures"} ||= "see $out/$suite.log" if !defined $total;
}

my %RUN = (
	correctness => sub {
		my $ok = 1;
		for my $level (qw(-O0 -O3)) {
			print "workload $level ...\n";
			my $t = PA35::timed_run({ timeout => 600, memory_kb => 8 * 1024 * 1024, allow_fail => 1 },
				$seed, PA35::workload_args($level, "$out/workload$level.o"));
			next if $t->{status} eq '0';
			$ok = 0;
			my @lines = grep { /\S/ && !/^\d+\.\d+ / } split /\n/, $t->{output};
			splice(@lines, 3) if @lines > 3;
			$note{'workload-compiles'} = $t->{status} eq 'timeout' ? "$level took over 600 s"
				: "$level failed: " . join(' | ', @lines);
			last;
		}
		$value{'workload-compiles'} = $ok;
		for my $suite (qw(behavior generated)) {
			my @extra = $suite eq 'generated' ? ('--seeds', $o{seeds}) : ('--lowir2native', $lowir2native);
			suite_failures($suite, step($suite, $^X, "$S/$suite.pl", '--compiler', $tools, @extra));
		}
	},
	speed => sub {
		unlink "$out/speed.tsv", "$out/scaling.tsv";
		step('speed', $^X, "$S/speed.pl", '--compiler', $seed, '--out', "$out/speed.tsv");
		if (-f "$out/speed.tsv") {
			for my $line (split /\n/, PA35::slurp("$out/speed.tsv")) {
				my ($metric, $c, $h, $ratio) = split /\t/, $line;
				$value{$metric} = $ratio if $metric ne 'metric';
			}
		}
		step('scaling', $^X, "$S/scaling.pl", '--compiler', $seed, '--no-host', '--out', "$out/scaling.tsv");
		if (-f "$out/scaling.tsv") {
			my @bad;
			for my $line (split /\n/, PA35::slurp("$out/scaling.tsv")) {
				my ($family, $who, $growth) = split /\t/, $line;
				next if $family eq 'family';
				if ($family eq 'worst') { $value{scaling} = $growth; next; }
				push @bad, "$family $growth" if $growth !~ /^(flat|[0-2](\.\d+)?)$/;
			}
			$note{scaling} = join(', ', @bad) if @bad;
		}
	},
	codegen => sub {
		my $self;
		if ($o{self}) {
			$self = abs_path($o{self});
		} else {
			print "self build (freeze.pl --label check) ...\n";
			my ($st, $log) = PA35::run_capture($^X, "$S/freeze.pl", '--label', 'check');
			PA35::spit("$out/freeze.log", $log);
			die "freeze failed (see $out/freeze.log):\n$log" if $st;
			($seed, $self) = ("$PA35::PA35/obj/bin/seed-check", "$PA35::PA35/obj/bin/self-check");
		}
		# The simulated runs cost about 100 times a native compile: measure
		# only a compiler that is already faster than the host.
		unlink "$out/measure/summary.tsv", "$out/measure/summary.txt";
		my $t0 = PA35::timed_run({ timeout => 120, allow_fail => 1 }, $seed, PA35::workload_args('-O3', "$out/probe.o"));
		my $h0 = PA35::timed_run({ allow_fail => 1 }, 'g++', '-std=gnu++11', PA35::workload_args('-O2', "$out/probe-host.o"));
		my $too_slow = $t0->{status} eq 'timeout' ? 'the seed takes over 120 s at -O3'
			: $t0->{status} ne '0' ? 'the seed does not compile the workload at -O3'
			: $t0->{user} > $SIM_LIMIT * $h0->{user} ? sprintf('the seed takes %.2f of the host time at -O3', $t0->{user} / $h0->{user})
			: undef;
		if ($too_slow) {
			print "measure: skipped: $too_slow\n";
			$value{identical} = 0;
			$note{identical} = "not measured: $too_slow; fix compiler speed first";
		} else {
			step('measure', $^X, "$S/measure.pl", '--seed', $seed, '--self', $self, '--out', "$out/measure", '--sim',
				($o{'no-cache'} ? '--no-cache' : ()));
			if (-f "$out/measure/summary.tsv") {
				my $identical = 1;
				for my $line (split /\n/, PA35::slurp("$out/measure/summary.tsv")) {
					my ($level, $metric, $a, $b, $ratio) = split /\t/, $line;
					next if $level eq 'level';
					if ($metric eq 'identical') { $identical &&= $a; next; }
					$value{"$level-$metric"} = $ratio;
				}
				$value{identical} = $identical ? 1 : 0;
				($value{text}) = PA35::slurp("$out/measure/summary.txt") =~ /text bytes: .*\(([\d.]+)x\)/;
			} else {
				$value{identical} = 0;
				$note{identical} = "measure failed: see $out/measure.log";
			}
		}
		my (undef, $log) = step('kernels', $^X, "$S/kernels.pl", '--compiler', $seed, '--out', "$out/kernels", '--scale', 16);
		for my $line (split /\n/, $log) {
			$value{"kernel-$1"} = $2 if $line =~ /^(\S+)\s+([\d.]+)x GCC/;
			if ($line =~ /^(\S+)\s+ERROR (.*)$/) { $value{"kernel-$1"} = 'error'; $note{"kernel-$1"} = $2; }
		}
	},
);

my ($passed, $total, $report, $stopped) = (0, 0, '', undef);
for my $group (@GROUPS) {
	my @gates = grep { $_->{group} eq $group } @GATES;
	if ($stopped) {
		$report .= sprintf("SKIP %-26s not run: %s gates failed\n", $_->{name}, $stopped) for @gates;
		$total += @gates;
		unlink "$out/values-$group.tsv";
		next;
	}
	$RUN{$group}->();
	# This group's measurements, for progress.pl.
	PA35::spit("$out/values-$group.tsv", join('', map { "$_->{metric}\t" . ($value{ $_->{metric} } // 'missing') . "\n" } @gates));
	my $group_ok = 1;
	for my $g (@gates) {
		my $v = $value{ $g->{metric} };
		my $ok = defined $v && $v =~ /^[\d.]+$/
			&& ($g->{op} eq '<=' ? $v <= $g->{limit} : $g->{op} eq '==' ? $v == $g->{limit} : $v >= $g->{limit});
		$total++;
		$passed++ if $ok;
		$group_ok &&= $ok;
		my $shown = !defined $v ? 'not measured' : $v =~ /^\d+\.\d+$/ ? sprintf('%.4f', $v) : $v;
		$report .= sprintf("%-4s %-26s %-14s %s %s%s\n", $ok ? 'PASS' : 'FAIL', $g->{name}, $shown, $g->{op}, $g->{limit},
			$note{ $g->{metric} } ? "  ($note{ $g->{metric} })" : '');
	}
	$stopped = $group if !$group_ok && $o{group} eq 'all';
}
$report .= "\nseed $seed\n";
$report .= $passed == $total
	? "===== ALL TESTS PASSED SUCCESSFULLY! ($passed/$total) =====\n"
	: "===== TEST SUMMARY: $passed / $total TESTS PASSED =====\n";
PA35::spit("$out/report-$o{group}.txt", $report);
print "\n", $report;
exit($passed == $total ? 0 : 1);
