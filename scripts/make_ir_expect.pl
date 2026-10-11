#!/usr/bin/perl
# Write an expectation sidecar that bounds an output's size from a reference
# output, for scripts/expect_ir.pl.
#
#   make_ir_expect.pl <reference-ir-file> [tolerance-percent]
#
# Prints the sidecar on standard output.  Every count is the reference's
# count widened by the tolerance (default 10%) and rounded up, plus one, and
# at least three more than the count, so a design that is as good as the
# reference or slightly larger passes (a retyping copy or an unmerged jump
# does not fail a tiny function) and one that is materially larger does
# not.  Hand-written property lines belong after the generated block;
# scripts/regenerate_ir_expectations.pl rewrites the block and keeps them.
use strict;
use warnings;
use POSIX qw(ceil);

my ($ir_path, $tolerance) = @ARGV;
die "usage: make_ir_expect.pl <reference-ir-file> [tolerance-percent]\n"
	if !defined($ir_path);
$tolerance = 10 if !defined($tolerance);

open(my $fh, '<', $ir_path) or die "Unable to read $ir_path: $!\n";
my @lines = <$fh>;
close($fh);
chomp(@lines);

my $machine = scalar(grep { /^machine_ir\b/ } @lines) ? 1 : 0;
my %per_function;
my @order;
my %stack_size;
my %total;
{
	my $function = '';
	my $section = '';
	for my $line (@lines)
	{
		if ($line =~ /^function\s+\@([^\s(]+)/)
		{
			$function = $1;
			$section = '';
			push @order, $function if !exists($per_function{$function});
			$per_function{$function} ||= { instructions => 0, blocks => 0 };
			next;
		}
		if ($line =~ /^(?:declare|global|machine_ir|startup)\b/ || $line =~ /^\}/)
		{
			$function = '';
			$section = '';
			next;
		}
		next if $function eq '';
		if ($line =~ /^\s+block\s+\^/)
		{
			$section = 'block';
			++$per_function{$function}{blocks};
			++$total{blocks};
			next;
		}
		if ($line =~ /^\s{2}(abi|frame)\s*$/) { $section = $1; next; }
		if ($section eq 'frame' && $line =~ /^\s+stack_size\s+(\d+)/)
		{
			$stack_size{$function} = $1;
			next;
		}
		next if $section ne 'block';
		next if $line !~ /^\s{4}\S/;
		my $body = $line;
		$body =~ s/^\s+//;
		$body =~ s/^%[^\s=]+\s*=\s*//;
		my ($kind) = split(/[\s,()]+/, $body);
		$kind = '' if !defined($kind);
		$kind =~ s/\..*$// if $machine;
		++$per_function{$function}{instructions};
		++$total{instructions};
		++$total{call} if $kind eq 'call';
		if ($machine)
		{
			$total{mem} += () = $body =~ /\[[^\]]*\]/g;
		}
		else
		{
			++$total{load} if $kind eq 'load';
			++$total{store} if $kind eq 'store';
			++$total{phi} if $kind eq 'phi';
		}
	}
}

sub widen
{
	my ($n) = @_;
	my $widened = ceil($n * (100 + $tolerance) / 100) + 1;
	return $widened > $n + 3 ? $widened : $n + 3;
}

my $name = $ir_path;
$name =~ s{.*/}{};
print "# budget from $name: the course solution's count, widened by $tolerance% and rounded up, plus one, and at least three more than the count\n";
print "instructions <= " . widen($total{instructions} || 0) . "\n";
print "blocks <= " . widen($total{blocks} || 0) . "\n";
print "count(call) <= " . widen($total{call} || 0) . "\n";
if ($machine)
{
	print "mem <= " . widen($total{mem} || 0) . "\n";
}
else
{
	print "count(load) <= " . widen($total{load} || 0) . "\n";
	print "count(store) <= " . widen($total{store} || 0) . "\n";
	print "count(phi) <= " . widen($total{phi} || 0) . "\n";
}
for my $function (@order)
{
	print "instructions <= " . widen($per_function{$function}{instructions}) . " in \@$function\n";
	# A frame layout is a design choice; the envelope leaves it room in
	# slots (two at 10%, more as the tolerance grows) rather than a share.
	print "stack_size <= " . (ceil($stack_size{$function} * (100 + $tolerance) / 100) + 32 * ceil($tolerance / 10)) . " in \@$function\n"
		if $machine && exists($stack_size{$function});
}
