#!/usr/bin/env perl
# parity.pl [--reference C] [--levels O0,O3] [--counter auto|callgrind|perf] [--sim] [--no-cache] [--out DIR]
#           [--from DIR]
#
# How this compiler compares with the reference compiler on this machine,
# as information beside the gates (the gates are floors; this is the
# distance to the course solution):
#   speed    compile time and peak memory of each on the frozen workload
#            (speed.pl on each): the figures themselves, with each
#            compiler's ratio to the host compiler beside them
#   codegen  generated code at constant work: this repository's own source
#            built by the reference compiler and by dev/cppgm++, both
#            counted on the workload (ab.pl), with the text size of each;
#            --sim counts under callgrind's cache and branch simulation and
#            adds its cycle proxy
# The reference is C, else reference-binaries/cppgm++ (after
# `make reference-binaries`), else dev/cppgm++-ref. Prints one table with
# the reference's figure, yours and the ratio yours / reference, writes
# DIR/parity.tsv (default pa35/obj/parity), and exits 1 when a figure is
# missing: a compile that failed or ran over its limit is shown as such.
# --from DIR renders the report again from the measurements a run left in
# DIR without measuring.
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Path qw(make_path);
use Getopt::Long;
use PA35;

my %o = (levels => 'O0,O3', counter => 'auto', out => "$PA35::PA35/obj/parity");
GetOptions(\%o, 'reference=s', 'levels=s', 'counter=s', 'sim', 'no-cache', 'out=s', 'from=s')
	or die "usage: parity.pl [--reference C] [--levels O0,O3] [--counter auto|callgrind|perf] [--sim] [--no-cache] [--out DIR] [--from DIR]\n";
$o{out} = $o{from} if defined $o{from};
my ($reference, $ours);
if (!defined $o{from}) {
	$reference = $o{reference} // (-x "$PA35::ROOT/reference-binaries/cppgm++"
		? "$PA35::ROOT/reference-binaries/cppgm++" : "$PA35::ROOT/dev/cppgm++-ref");
	$reference = abs_path($reference) // die "no reference compiler $reference (run `make reference-binaries`)\n";
	$ours = abs_path("$PA35::ROOT/dev/cppgm++") // die "no dev/cppgm++ (run `make build`)\n";
}
make_path($o{out});
my $out = abs_path($o{out}) // die "no directory $o{out}\n";
my $S = $FindBin::Bin;

my @order;    # measurements in report order
my %row;      # measurement => [reference's figure, ours]
sub add_row {
	my ($name, $a, $b) = @_;
	push @order, $name if !exists $row{$name};
	$row{$name} = [$a, $b];
}

# speed.pl's rows: metric, the compiler's figure, the host's, their ratio;
# a compile that failed or ran over its limit has the word instead.
sub speed_of {
	my ($label, $compiler) = @_;
	if (!defined $o{from}) {
		print "speed: $label ...\n";
		PA35::run_or_die($^X, "$S/speed.pl", '--compiler', $compiler, '--out', "$out/speed-$label.tsv");
	}
	my %figure;
	for my $line (split /\n/, PA35::slurp("$out/speed-$label.tsv")) {
		my ($metric, $c, $h, $r) = split /\t/, $line;
		$figure{$metric} = [$c, $r] if $metric ne 'metric';
	}
	return \%figure;
}
my ($ref_speed, $our_speed) = (speed_of('reference', $reference), speed_of('yours', $ours));
for my $metric (qw(time-O0 time-O3 rss-O3)) {
	my $unit = $metric =~ /^time/ ? 'seconds' : 'KB';
	add_row("$metric $unit", $ref_speed->{$metric}[0], $our_speed->{$metric}[0]);
	add_row("$metric (to host)", $ref_speed->{$metric}[1], $our_speed->{$metric}[1]);
}

# Constant work: both compilers build this source, and the two builds run
# the workload.
if (!defined $o{from}) {
	print "codegen: this source built by the reference and by dev/cppgm++ ...\n";
	my ($st, $log) = PA35::run_capture($^X, "$S/ab.pl", $reference, $ours,
		'--levels', $o{levels}, '--counter', $o{counter}, '--out', "$out/ab",
		($o{sim} ? '--sim' : ()), ($o{'no-cache'} ? '--no-cache' : ()));
	die "ab.pl failed:\n$log" if $st;
}
for my $line (split /\n/, PA35::slurp("$out/ab/measure/summary.tsv")) {
	my ($level, $metric, $a, $b, $ratio) = split /\t/, $line;
	next if $level eq 'level' || $metric eq 'identical' || $metric eq 'calls';
	add_row($metric eq 'total' ? "$level total instructions" : "$level $metric", $a, $b);
}
my ($ta, $tb) = PA35::slurp("$out/ab/measure/summary.txt") =~ /text bytes: \S+ (\d+), \S+ (\d+)/;
add_row('text bytes', $ta, $tb) if $ta;

my (@lines, $missing);
printf "\n%-28s %14s %14s %8s\n", 'measurement', 'reference', 'yours', 'ratio';
my $fmt = sub {
	my ($v) = @_;
	return 'missing' if !defined $v;
	return $v if $v !~ /^[\d.]+$/;
	return $v > 1e6 ? PA35::fmt_m($v) : $v =~ /\./ ? sprintf('%.4f', $v) : $v;
};
for my $k (@order) {
	my ($a, $b) = @{ $row{$k} };
	my $numeric = defined $a && defined $b && $a =~ /^[\d.]+$/ && $b =~ /^[\d.]+$/;
	$missing++ if !$numeric;
	my $ratio = $numeric && $a > 0 ? sprintf('%.3f', $b / $a) : '-';
	printf "%-28s %14s %14s %8s\n", $k, $fmt->($a), $fmt->($b), $ratio;
	push @lines, join("\t", $k, $a // 'missing', $b // 'missing', $ratio);
}
PA35::spit("$out/parity.tsv", join("\n", "measurement\treference\tyours\tratio", @lines) . "\n");
print "\nratios above 1 are where the reference is ahead; wrote $out/parity.tsv\n";
if ($missing) {
	print "incomplete: $missing measurement(s) have no figure for one of the compilers\n";
	exit 1;
}
