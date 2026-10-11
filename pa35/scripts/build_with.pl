#!/usr/bin/env perl
# build_with.pl --compiler C --out DIR [--repo R] [--jobs N]
#
# Builds the cppgm++ in R/dev (default: this repository) with compiler C:
# every object of the g++ build's cppgm++ link, compiled with the g++
# build's own flags and defines, then linked with the g++ build's link line
# (same host libraries, same malloc). Writes DIR/cppgm++.
#
# Uses: the self build (C = dev/cppgm++), and codegen A/B at constant work:
# build the same source with an old and a new compiler, then compare the
# two results; only code generation differs between them.
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Getopt::Long;
use PA35;

my %a;
GetOptions(\%a, 'compiler=s', 'out=s', 'repo=s', 'jobs=i') && $a{compiler} && $a{out}
	or die "usage: build_with.pl --compiler C --out DIR [--repo R] [--jobs N]\n";
my ($binary, $count) = PA35::build_with(%a);
print "built $binary from $count objects\n";
