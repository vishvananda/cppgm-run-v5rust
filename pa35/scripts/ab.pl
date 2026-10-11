#!/usr/bin/env perl
# ab.pl OLD [NEW] [--levels O0,O3] [--counter auto|callgrind|perf] [--sim] [--no-cache] [--out DIR]
#
# Codegen comparison at constant work: builds this repository's current
# source with compiler OLD and with compiler NEW (default dev/cppgm++), then
# counts both builds on the frozen workload. Both builds run the same
# source, so they do the same work and must write identical objects; any
# difference is the generated code. Prints NEW / OLD per level (with
# callgrind also the instruction-class mix, calls and functions in DIR;
# --sim selects callgrind with cache and branch simulation and adds the
# cycle proxy; --no-cache measures again even when a cached result exists).
#
# Typical use: freeze a reference before a change (make -C pa35 freeze
# LABEL=before), make the change, `make build`, then
#   perl pa35/scripts/ab.pl pa35/obj/bin/seed-before
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Path qw(remove_tree);
use Getopt::Long;
use PA35;

my %o = (levels => 'O0,O3', counter => 'auto', out => "$PA35::PA35/obj/ab");
GetOptions(\%o, 'levels=s', 'counter=s', 'sim', 'no-cache', 'out=s') && (@ARGV == 1 || @ARGV == 2)
	or die "usage: ab.pl OLD [NEW] [--levels O0,O3] [--counter auto|callgrind|perf] [--sim] [--no-cache] [--out DIR]\n";
$o{counter} = 'callgrind' if $o{sim};
my ($old, $new) = (abs_path($ARGV[0]), abs_path($ARGV[1] // "$PA35::ROOT/dev/cppgm++"));
-x $old or die "not executable: $ARGV[0]\n";
-x $new or die "not executable: $new\n";
my %built;
for my $k (['old', $old], ['new', $new]) {
	remove_tree("$o{out}/$k->[0]-build");
	print "building the current source with $k->[1] ...\n";
	($built{ $k->[0] }) = PA35::build_with(compiler => $k->[1], out => "$o{out}/$k->[0]-build");
}
my ($st, $log) = PA35::run_capture($^X, "$FindBin::Bin/measure.pl", '--seed', $built{old}, '--self', $built{new},
	'--out', "$o{out}/measure", '--levels', $o{levels}, '--counter', $o{counter}, '--labels', 'old,new',
	($o{sim} ? '--sim' : ()), ($o{'no-cache'} ? '--no-cache' : ()));
print $log;
exit $st;
