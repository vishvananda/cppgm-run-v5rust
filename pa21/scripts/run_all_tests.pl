#!/usr/bin/perl
use strict;
use warnings;
use FindBin;
use File::Basename qw(dirname);
my $repo_root = dirname(dirname($FindBin::Bin));
exec("perl", "$repo_root/scripts/run_all_tests_common.pl", "text_t", @ARGV)
	or die "exec failed: $!";
