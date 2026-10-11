#!/usr/bin/perl
use strict;
use warnings;
use FindBin;
use File::Basename qw(dirname);
my $repo_root = dirname(dirname($FindBin::Bin));
exec("perl", "$repo_root/scripts/run_cpptoolchain_tests_worker.pl", @ARGV)
	or die "exec failed: $!";
