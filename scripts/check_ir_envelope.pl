#!/usr/bin/perl
# The performance envelope for generated programs.
#
#   check_ir_envelope.pl [--write | --require-sidecars] <tolerance-percent> <test-root>...
#
# For every `x.t` under the roots whose generated program `x.my.program`
# exists, run it under Cachegrind and read its instruction count.  With
# `--write`, record the count in `x.ref.ir`; otherwise compare against the
# recorded count and fail when the program executes more than the tolerance
# above it.  A program without a sidecar is skipped, or, with
# `--require-sidecars` (for suites that track a sidecar for every program),
# fails.  When valgrind is not installed, every root is skipped with a notice
# and the exit status is 0: the envelope is a quality bar, not a build
# dependency.
use strict;
use warnings;
use File::Find;
use POSIX qw(ceil);

my $usage = "usage: check_ir_envelope.pl [--write | --require-sidecars] <tolerance-percent> <test-root>...\n";
my $write = 0;
my $require_sidecars = 0;
while (@ARGV && $ARGV[0] =~ /^--/)
{
	my $option = shift @ARGV;
	if ($option eq '--write') { $write = 1; }
	elsif ($option eq '--require-sidecars') { $require_sidecars = 1; }
	else { die $usage; }
}
my ($tolerance, @roots) = @ARGV;
die $usage if !defined($tolerance) || !@roots || ($write && $require_sidecars);

my $valgrind = `which valgrind 2>/dev/null`;
chomp($valgrind);
if ($valgrind eq '')
{
	print "performance envelope: skipped (valgrind not installed)\n";
	exit(0);
}

sub instruction_count
{
	my ($program) = @_;
	my $args = '';
	my $args_file = $program;
	$args_file =~ s/\.my\.program$/.args/;
	if (-f $args_file)
	{
		open(my $fh, '<', $args_file) or die "Unable to read $args_file: $!\n";
		$args = join(' ', map { chomp; $_ } <$fh>);
		close($fh);
	}
	my $stdin = '/dev/null';
	my $stdin_file = $args_file;
	$stdin_file =~ s/\.args$/.stdin/;
	$stdin = $stdin_file if -f $stdin_file;
	my $output = `$valgrind --tool=cachegrind --cache-sim=no --vgdb=no --cachegrind-out-file=/dev/null "$program" $args < "$stdin" 2>&1 1>/dev/null`;
	return undef if $output !~ /I\s+refs:\s+([\d,]+)/;
	my $count = $1;
	$count =~ s/,//g;
	return $count;
}

my ($checked, $failed, $written, $skipped) = (0, 0, 0, 0);
for my $root (@roots)
{
	next if !-d $root;
	my @tests;
	find(sub { push @tests, $File::Find::name if /\.t$/ }, $root);
	for my $test (sort @tests)
	{
		my $base = $test;
		$base =~ s/\.t$//;
		my $program = "$base.my.program";
		my $sidecar = "$base.ref.ir";
		if (!-x $program) { ++$skipped; next; }
		my $count = instruction_count($program);
		if (!defined($count)) { ++$skipped; next; }
		if ($write)
		{
			open(my $fh, '>', $sidecar) or die "Unable to write $sidecar: $!\n";
			print $fh "$count\n";
			close($fh);
			++$written;
			next;
		}
		if (!-f $sidecar)
		{
			if (!$require_sidecars) { ++$skipped; next; }
			++$checked;
			++$failed;
			print "$test: no $sidecar; record it with `make test-perf-refs`\n";
			next;
		}
		open(my $fh, '<', $sidecar) or die "Unable to read $sidecar: $!\n";
		my $reference = <$fh>;
		close($fh);
		chomp($reference);
		++$checked;
		my $limit = ceil($reference * (100 + $tolerance) / 100);
		if ($count > $limit)
		{
			++$failed;
			printf "%s: %d instructions, envelope %d (reference %d, %s%% tolerance)\n",
				$test, $count, $limit, $reference, $tolerance;
		}
	}
}
if ($write)
{
	print "performance envelope: wrote $written sidecar(s), skipped $skipped\n";
	exit(0);
}
printf "performance envelope: %s (%d/%d within %s%%, %d skipped)\n",
	($failed ? 'FAIL' : 'PASS'), $checked - $failed, $checked, $tolerance, $skipped;
exit($failed ? 1 : 0);
