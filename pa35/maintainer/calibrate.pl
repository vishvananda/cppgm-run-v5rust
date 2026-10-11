#!/usr/bin/env perl
# calibrate.pl [--seeds N] [--from DIR] [--out FILE] [--course-out FILE]
#
# The PA35 release check, for course maintainers (`make -C pa35 ref-test`).
#
# 1. The course build: every gate (check.pl, all three groups, N generated
#    seeds, default 100) on dev/cppgm++ as this repository's Makefile builds
#    it, into DIR/course.
# 2. The student build: the same source built as the student export's
#    dev/Makefile builds it (%STUDENT_MAKE: plain -O3, the C library's
#    malloc), seed by the host and self by that seed, checked by all three
#    groups into DIR/student. These are the values a student's gates
#    compare with. The correctness group runs on each build: a compiler
#    built with other flags or another allocator is another binary, and a
#    gate that was not measured in a build fails.
#
# In both builds every gate must pass, and every measured gate must clear its
# limit by the release margin of its kind (below): the value times
# (1 + margin) must still be within the limit.
#
# DIR is pa35/obj/calibrate; --from DIR reads finished runs there instead of
# measuring. It writes two files: FILE (default pa35/calibration.tsv, which
# the export ships) holds the student build's value and margin for each gate
# and the measurements behind them; the course build's go to the course file
# (default pa35/maintainer/calibration-course.tsv, which the export drops).
# Each records the machine and toolchain generically (the kernel's release,
# the compiler's version, the C library's version) and the commit measured.
# Exits nonzero when a gate fails or a margin falls short in either build;
# the files are written either way.
#
# Release margins, by kind of measurement (TESTING_AND_REFERENCES.md, "PA35
# release check", has the measurements behind them):
#   count   own-binary and kernel instructions: deterministic; relinking the
#           seed and self in other object orders, and other paths and
#           environment sizes, moved the ratios by under 0.01%
#   proxy   cycle proxy: the simulated caches and branch predictors follow
#           code layout; 81 relinked seed/self pairs spread over 1.2% at -O0
#           and 0.5% at -O3
#   text    .text bytes: relinking moved the ratio by 0.05% (padding)
#   memory  peak RSS ratio: repeated runs spread over 13%
#   time    user-time ratios and scaling growth: repeated runs spread over
#           15% (-O0 time) and up to 24% (scaling, across seed builds)
use strict;
use warnings;
use FindBin;
use lib "$FindBin::Bin/../scripts";
use Cwd qw(abs_path);
use File::Path qw(make_path remove_tree);
use Getopt::Long;
use POSIX qw(strftime);
use PA35;

our %MARGIN = (count => 0.02, proxy => 0.015, text => 0.01, memory => 0.25, time => 0.25);
my %KIND = (
	'O0-own' => 'count', 'O3-own' => 'count',
	'O0-proxy' => 'proxy', 'O3-proxy' => 'proxy',
	'text' => 'text',
	'rss-O3' => 'memory',
	'time-O0' => 'time', 'time-O3' => 'time', 'scaling' => 'time',
);
$KIND{$_} = 'count' for map { "kernel-$_" } qw(accessor-distinct accessor-alias lookup exceptions);
# The student export's dev/Makefile flags (scripts/export_student_repo.sh):
# no -fno-strict-aliasing.
our @STUDENT_MAKE = ('CC_FLAGS=-std=gnu++11 -Wall -O3');

my %o = (seeds => 100, out => "$PA35::PA35/calibration.tsv", 'course-out' => "$PA35::PA35/maintainer/calibration-course.tsv");
GetOptions(\%o, 'seeds=i', 'from=s', 'out=s', 'course-out=s')
	or die "usage: calibrate.pl [--seeds N] [--from DIR] [--out FILE] [--course-out FILE]\n";

my $dir = $o{from} // "$PA35::PA35/obj/calibrate";
my $check = "$PA35::PA35/scripts/check.pl";
if (!$o{from}) {
	remove_tree("$dir/course", "$dir/student");
	make_path("$dir/course", "$dir/student");
	# check.pl prints its own progress and report; the gates are judged
	# below from the measurements it leaves.
	print "course build: check.pl --seeds $o{seeds} ...\n";
	system($^X, $check, '--seeds', $o{seeds}, '--out', "$dir/course");
	print "student build (@STUDENT_MAKE): seed and self ...\n";
	my $host = (grep { -x } map { "$_/g++" } split /:/, $ENV{PATH})[0] // die "no g++ on PATH\n";
	remove_tree("$dir/student-seed", "$dir/student-self");
	my ($seed) = PA35::build_with(compiler => $host, out => "$dir/student-seed", make_vars => \@STUDENT_MAKE);
	my ($self) = PA35::build_with(compiler => $seed, out => "$dir/student-self", make_vars => \@STUDENT_MAKE);
	for my $group (qw(correctness speed codegen)) {
		print "student build: check.pl --group $group ...\n";
		system($^X, $check, '--group', $group, '--seed', $seed, '--self', $self, '--seeds', $o{seeds},
			'--lowir2native', "$PA35::ROOT/dev/lowir2native", '--out', "$dir/student");
	}
}
$dir = abs_path($dir) // die "no calibration directory $dir\n";

my @GATES;
for my $line (split /\n/, PA35::slurp("$PA35::PA35/gates.txt")) {
	next if $line =~ /^\s*(#|$)/;
	my ($group, $name, $metric, $op, $limit) = split ' ', $line;
	push @GATES, { group => $group, name => $name, metric => $metric, op => $op, limit => $limit };
}
sub values_in {
	my ($d) = @_;
	my %value;
	for my $group (qw(correctness speed codegen)) {
		my $file = "$d/values-$group.tsv";
		next unless -f $file;
		for my $line (split /\n/, PA35::slurp($file)) {
			my ($metric, $v) = split /\t/, $line;
			$value{$metric} = $v if defined $v;
		}
	}
	return \%value;
}
my %build = (course => values_in("$dir/course"), student => values_in("$dir/student"));

# One gate in one build: (shown value, margin, status).
sub judge {
	my ($g, $v) = @_;
	my $numeric = defined $v && $v =~ /^-?\d+(\.\d+)?(e-?\d+)?$/;
	my $pass = $numeric && ($g->{op} eq '<=' ? $v <= $g->{limit} : $g->{op} eq '==' ? $v == $g->{limit} : $v >= $g->{limit});
	my ($margin, $status) = ('-', $pass ? 'pass' : 'FAIL');
	if ($pass && $g->{op} eq '<=') {
		my $kind = $KIND{ $g->{metric} } // die "no release margin kind for $g->{metric}\n";
		my $m = $v > 0 ? $g->{limit} / $v - 1 : 9.99;
		$margin = sprintf('%.2f%%', 100 * $m);
		$status = $m + 1e-12 >= $MARGIN{$kind} ? 'pass' : 'SHORT';
	}
	my $shown = !defined $v ? 'not measured' : $v =~ /^\d+\.\d+$/ ? sprintf('%.4f', $v) : $v;
	return ($shown, $margin, $status);
}
my ($bad, %rows) = (0);
for my $g (@GATES) {
	my $need = $g->{op} eq '<=' ? sprintf('%.1f%% (%s)', 100 * $MARGIN{ $KIND{ $g->{metric} } }, $KIND{ $g->{metric} }) : '-';
	my $failed = 0;
	for my $b (qw(course student)) {
		my @j = judge($g, $build{$b}{ $g->{metric} });
		$failed = 1 if $j[2] ne 'pass';
		push @{ $rows{$b} }, [$g->{name}, $g->{op}, $g->{limit}, $need, @j];
	}
	$bad += $failed;
}

# Raw measurements behind the ratios, per build.
sub details_of {
	my ($b) = @_;
	my $d = "$dir/$b";
	my @detail;
	if (-f "$d/measure/summary.tsv") {
		for my $line (split /\n/, PA35::slurp("$d/measure/summary.tsv")) {
			my ($level, $metric, $seed, $self, $ratio) = split /\t/, $line;
			next if $level eq 'level' || $metric eq 'identical';
			push @detail, join("\t", "$level-$metric", $seed, $self, $ratio);
		}
		my ($ts, $tf) = PA35::slurp("$d/measure/summary.txt") =~ /text bytes: seed (\d+), self (\d+)/;
		push @detail, join("\t", 'text-bytes', $ts, $tf, sprintf('%.6f', $tf / $ts)) if $ts;
	}
	if (-f "$d/speed.tsv") {
		for my $line (split /\n/, PA35::slurp("$d/speed.tsv")) {
			my ($metric, $c, $h, $ratio) = split /\t/, $line;
			push @detail, join("\t", $metric, $c, $h, $ratio) if $metric ne 'metric';
		}
	}
	if (-f "$d/scaling.tsv") {
		for my $line (split /\n/, PA35::slurp("$d/scaling.tsv")) {
			my ($family, $who, $growth, $times) = split /\t/, $line;
			push @detail, join("\t", "scaling-$family", $growth, $times // '', '') if $family ne 'family';
		}
	}
	for my $suite (qw(behavior generated)) {
		my $log = "$d/$suite.log";
		my ($line) = grep { /\d+\/\d+ \w+ pass/ } split /\n/, (-f $log ? PA35::slurp($log) : '');
		push @detail, join("\t", $suite, $line // 'not run', '', '');
	}
	return @detail;
}

# The machine, the toolchain and the source, generically: the release of
# the kernel, the version of the compiler and of the C library, not the
# distribution's package strings.
sub first_line { my ($cmd) = @_; my $out = `$cmd 2>/dev/null` // ''; ($out) = split /\n/, $out; return $out // ''; }
my ($cpu) = (-r '/proc/cpuinfo' ? PA35::slurp('/proc/cpuinfo') : '') =~ /^model name\s*:\s*(.*)$/m;
my ($kernel) = first_line('uname -sr') =~ /^(\S+\s+\d+\.\d+)/;
my ($gxx) = first_line('g++ --version') =~ /(\d+\.\d+(?:\.\d+)?)/;
my ($libc) = first_line('ldd --version') =~ /(\d+\.\d+)\s*$/;
my ($valgrind) = first_line('valgrind --version') =~ /(\d+\.\d+(?:\.\d+)?)/;
my $macros = `printf '#include <cstddef>\\n' | g++ -std=gnu++11 -x c++ -E -dM - 2>/dev/null` // '';
my ($glibcxx) = $macros =~ /#define __GLIBCXX__ (\d+)/;
my ($glibcxx_release) = $macros =~ /#define _GLIBCXX_RELEASE (\d+)/;
# The source measured: HEAD, and whether the compiler's source differs from
# it (the calibration is committed after it is written, so HEAD is the
# calibration commit's parent).
my $commit = first_line("git -C '$PA35::ROOT' rev-parse HEAD");
my $dirty = `git -C '$PA35::ROOT' status --porcelain -- dev/src dev/*.cpp dev/Makefile dev/frontend_source_sets.mk 2>/dev/null` // '';
my $cachesim = 'unknown';
for my $cg (glob("$PA35::PA35/obj/cache/*-O3-sim/run.cg")) {
	open(my $fh, '<', $cg) or next;
	my @desc;
	while (my $l = <$fh>) {
		last if $l =~ /^(events|positions|fl|ob|fn)[:=]/;
		push @desc, $1 if $l =~ /^desc: ((?:I1|D1|LL) cache:.*?)\s*$/;
	}
	close $fh;
	if (@desc) { $cachesim = join('; ', @desc); last; }
}
my ($flags) = PA35::slurp("$PA35::ROOT/dev/Makefile") =~ /^CC_FLAGS \?= (.*?)\s*$/m;
$flags //= 'unknown';
my @meta = (
	['date', strftime('%Y-%m-%dT%H:%M:%SZ', gmtime)],
	['source', $commit . ($dirty =~ /\S/ ? ' with the compiler source modified' : '')],
	['cpu', $cpu // 'unknown'],
	['processors', first_line('getconf _NPROCESSORS_ONLN')],
	['kernel', $kernel // 'unknown'],
	['host compiler', defined $gxx ? "g++ $gxx" : 'unknown'],
	['libstdc++', defined $glibcxx ? "__GLIBCXX__ $glibcxx, release " . ($glibcxx_release // '?') : 'unknown'],
	['libc', defined $libc ? "glibc $libc" : 'unknown'],
	['valgrind', defined $valgrind ? "valgrind $valgrind" : 'unknown'],
	['cache simulation', $cachesim],
	['generated seeds', $o{from} ? 'as run' : $o{seeds}],
);
my %BUILD_META = (
	student => join(' ', @STUDENT_MAKE) . ' (the student export\'s dev/Makefile)',
	course => "dev/Makefile: CC_FLAGS $flags",
);
my %HEAD = (
	student => "# PA35 calibration: the course solution's value for every gate in gates.txt, built\n"
		. "# as the student export's dev/Makefile builds a seed (the configuration a student's\n"
		. "# gates compare), on the machine and toolchain below; written by the course's\n"
		. "# release check, do not edit by hand. Each gate must clear its limit by the\n"
		. "# release margin of its kind: margin = limit / value - 1. The limits do not depend on\n"
		. "# this file.\n",
	course => "# PA35 calibration of the course build (this repository's dev/Makefile), for the\n"
		. "# release check; written by `make -C pa35 ref-test`, do not edit by hand. The\n"
		. "# student build's values are in pa35/calibration.tsv.\n",
);
for my $b (qw(student course)) {
	my $text = $HEAD{$b};
	$text .= join('', map { "meta\t$_->[0]\t$_->[1]\n" } @meta, ['build', $BUILD_META{$b}]);
	$text .= "#\n# gate\top\tlimit\trelease margin\tvalue\tmargin\tstatus\n";
	$text .= join('', map { join("\t", 'gate', @$_) . "\n" } @{ $rows{$b} });
	$text .= "#\n# measurement\tseed or cppgm++\tself or host\tratio\n";
	$text .= join('', map { "detail\t$_\n" } details_of($b));
	PA35::spit($b eq 'student' ? $o{out} : $o{'course-out'}, $text);
}

my $fmt = "%-26s %-3s %-5s %-16s | %-12s %-8s %-5s | %-12s %-8s %s\n";
printf "\n$fmt", 'gate', 'op', 'limit', 'release margin', 'course', 'margin', '', 'student', 'margin', '';
for my $i (0 .. $#{ $rows{course} }) {
	my ($c, $s) = ($rows{course}[$i], $rows{student}[$i]);
	printf $fmt, @$c, @$s[4 .. 6];
}
print "\nwrote $o{out} and $o{'course-out'}\n";
print $bad ? "===== RELEASE CHECK FAILED: $bad of " . scalar(@GATES) . " gates =====\n"
	: "===== RELEASE CHECK PASSED (" . scalar(@GATES) . "/" . scalar(@GATES) . ") =====\n";
exit($bad ? 1 : 0);
