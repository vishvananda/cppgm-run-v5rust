#!/usr/bin/perl
use strict;
use warnings;
use FindBin;
use File::Basename qw(dirname);
my $repo_root = dirname(dirname($FindBin::Bin));
# PA32's lanes compare optimized LowIR: the facts the C++ lowering states are
# validated but not compared, since an optimizer may add a fact it proves.
# The -O0 driver lane is the C++ lowering's own output and compares them, as
# PA10-PA23 do.
my $mode = (scalar(@ARGV) == 3 && $ARGV[2] =~ m{(?:^|/)host/o1(?:/|$)})
	? "link_program_t"
	: (scalar(@ARGV) == 3 && $ARGV[2] =~ m{(?:^|/)driver/o0(?:/|$)})
	? "lowir_t" : "lowir_optimized_t";
exec("perl", "$repo_root/scripts/compare_results_common.pl", $mode, @ARGV)
	or die "exec failed: $!";
