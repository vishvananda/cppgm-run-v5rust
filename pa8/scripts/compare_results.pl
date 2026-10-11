#!/usr/bin/perl
use strict;
use warnings;
use FindBin;
use File::Basename qw(dirname);
my $repo_root = dirname(dirname($FindBin::Bin));
die "Usage: compare_results.pl <ref-suffix> <my-suffix> <test-spec>\n" if @ARGV != 3;
my ($reference, $candidate, $tests) = @ARGV;
# The `cppgm++ --emit-lowir` debug lanes compare LowIR by the relaxed rules
# and require the reference's `!dbg` locations; the other debug lanes compare
# tool output text.
exec("perl", "$repo_root/scripts/run_routed_test_spec.pl", $tests,
  "*tests/debuginfo/source/*|*tests/debuginfo/source-g0/*::perl $repo_root/scripts/compare_results_common.pl lowir_debuginfo_t $reference $candidate {tests}",
  "*tests/debuginfo/*::perl $repo_root/scripts/compare_results_common.pl text_t $reference $candidate {tests}",
  "*::perl $repo_root/scripts/compare_results_common.pl lowir_roundtrip_t $reference $candidate {tests}")
  or die "exec failed: $!";
