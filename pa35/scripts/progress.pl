#!/usr/bin/env perl
# progress.pl [--out DIR]
#
# Reports how far each PA35 gate is from its limit, as passed rungs of the
# progress ladder gates.txt gives it (`from` and `step`: rungs at
# limit x (1 + step)^k for k = 0 .. K, the first k with limit x (1 + step)^k
# >= from being K). A measurement passes every rung at or above it, so a
# gate's rungs all pass exactly when the gate passes; a gate without a ladder,
# or an `==` gate, is one rung. It runs nothing: it reads the measurements
# check.pl left in DIR/values-<group>.tsv (default pa35/obj/check). A group's
# measurements count only when they are not older than an earlier group's,
# because check.pl runs the groups in order and a later group measured before
# the current earlier one describes an older tree; uncounted gates pass no
# rungs.
#
# The output is one `<gate>: PASS (n/m)` or `<gate>: FAIL (n/m)` line per
# gate, with the measurement beside it, and the course runners' summary line
# over all rungs, so progress toward the gates shows as a growing count.
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Getopt::Long;
use PA35;

my %o = (out => "$PA35::PA35/obj/check");
GetOptions(\%o, 'out=s') or die "usage: progress.pl [--out DIR]\n";

my (@GATES, @GROUPS, %seen);
for my $line (split /\n/, PA35::slurp("$PA35::PA35/gates.txt")) {
	next if $line =~ /^\s*(#|$)/;
	my ($group, $name, $metric, $op, $limit, $from, $step) = split ' ', $line;
	push @GATES, { group => $group, name => $name, metric => $metric, op => $op, limit => $limit,
		from => $from, step => $step };
	push @GROUPS, $group unless $seen{$group}++;
}

# The groups whose measurements count, and their values.
my (%value, %counted, %why);
my $newest_earlier;
for my $group (@GROUPS) {
	my $file = "$o{out}/values-$group.tsv";
	if (!-f $file) {
		$why{$group} = 'not measured';
		next;
	}
	my $mtime = (stat $file)[9];
	if (defined $newest_earlier && $mtime < $newest_earlier) {
		$why{$group} = 'measured before the earlier groups';
		next;
	}
	$newest_earlier = $mtime;
	$counted{$group} = 1;
	for my $line (split /\n/, PA35::slurp($file)) {
		my ($metric, $v) = split /\t/, $line;
		$value{$metric} = $v if defined $v && $v =~ /^-?[\d.]+$/;
	}
}

# A gate's rung thresholds, from the limit upward.
sub rungs {
	my ($g) = @_;
	return ($g->{limit}) if $g->{op} ne '<=' || !$g->{from} || !$g->{step};
	my @t = ($g->{limit});
	push @t, $t[-1] * (1 + $g->{step}) while $t[-1] < $g->{from} * (1 - 1e-12);
	return @t;
}

my ($passed, $total, $all_gates, $report) = (0, 0, 1, '');
$report .= "===== pa35 =====\n";
for my $g (@GATES) {
	my @t = rungs($g);
	my $v = $counted{ $g->{group} } ? $value{ $g->{metric} } : undef;
	my $n = 0;
	if (defined $v) {
		if ($g->{op} eq '<=') {
			$n = grep { $v <= $_ * (1 + 1e-12) } @t;
		} elsif ($g->{op} eq '==') {
			$n = $v == $g->{limit} ? 1 : 0;
		} else {
			$n = $v >= $g->{limit} ? 1 : 0;
		}
	}
	my $ok = $n == @t;
	$all_gates &&= $ok;
	$passed += $n;
	$total += @t;
	$report .= sprintf("%s: %s (%d/%d)\n", $g->{name}, $ok ? 'PASS' : 'FAIL', $n, scalar @t);
	my $detail = !defined $v ? ($why{ $g->{group} } // 'not measured')
		: sprintf('value %s, limit %s %s', $v, $g->{op}, $g->{limit});
	$detail .= sprintf('; next rung %.4f', $t[ @t - $n - 1 ]) if defined $v && !$ok && @t > 1;
	$report .= "    $detail\n";
}
$report .= "\n";
$report .= $all_gates
	? "===== ALL TESTS PASSED SUCCESSFULLY! ($passed/$total) =====\n"
	: "===== TEST SUMMARY: $passed / $total TESTS PASSED =====\n";
print $report;
exit($all_gates ? 0 : 1);
