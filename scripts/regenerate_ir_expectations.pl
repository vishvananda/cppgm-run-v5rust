#!/usr/bin/perl
# Rewrite the generated blocks of expectation sidecars from the current
# references, keeping their hand-written lines.
#
#   regenerate_ir_expectations.pl [--tolerance N] [--add-budgets] [--write] <root>...
#
# For every x.t under the roots with an x.ref.expect sidecar and a reference
# output (x.ref.mir for machine IR, else x.ref for LowIR):
#   - the budget block (make_ir_expect.pl at the tolerance, default 10%) is
#     regenerated, and no line of it is made tighter than the current one:
#     a bound is the larger of the two, and a line the generator no longer
#     writes is dropped;
#   - the outcome block (make_ir_outcome.pl, LowIR only) is regenerated;
#   - a contract block (make_ir_contract.pl, machine IR only) is regenerated
#     with the flags the current block shows (--preserve when it bounds
#     `preserve`, --memory when it has `count(load) >=` or `count(store) >=`).
# A block is the lines up to a blank line; a block that begins with
# `# budget`, `# outcome` or `# contract` is generated, every other block is
# hand-written and kept after the generated ones.  A sidecar with no budget
# block keeps having none unless --add-budgets is given.  Without --write
# the changed sidecars are listed; with it they are rewritten.
use strict;
use warnings;
use File::Basename qw(dirname basename);
use File::Find;
use FindBin;
use Getopt::Long;

my %o = (tolerance => 10);
GetOptions(\%o, 'tolerance=i', 'add-budgets', 'write') && @ARGV
	or die "usage: regenerate_ir_expectations.pl [--tolerance N] [--add-budgets] [--write] <root>...\n";

sub slurp
{
	my ($path) = @_;
	open(my $fh, '<', $path) or die "cannot read $path: $!\n";
	local $/;
	my $text = <$fh>;
	close($fh);
	return $text;
}

sub generate
{
	my (@command) = @_;
	open(my $fh, '-|', @command) or die "cannot run @command: $!\n";
	my @lines = <$fh>;
	close($fh);
	die "@command failed\n" if $?;
	chomp(@lines);
	return @lines;
}

# The blocks of a sidecar: [kind, [lines]] where kind is budget, outcome,
# contract or hand.
sub blocks_of
{
	my ($text) = @_;
	my @blocks;
	my @current;
	for my $line (split(/\n/, $text), '')
	{
		if ($line =~ /^\s*$/)
		{
			push @blocks, [@current] if @current;
			@current = ();
			next;
		}
		push @current, $line;
	}
	return map {
		my $kind = $_->[0] =~ /^# budget/ ? 'budget'
			: $_->[0] =~ /^# outcome/ ? 'outcome'
			: $_->[0] =~ /^# contract/ ? 'contract' : 'hand';
		[$kind, $_];
	} @blocks;
}

# A budget line's key (the line with its bound replaced) and bound.
sub budget_key
{
	my ($line) = @_;
	return ($line =~ /^(.*?)(<=|<)\s*(\d+)(.*)$/) ? ("$1$2 N$4", $3) : (undef, undef);
}

my @sidecars;
find({ no_chdir => 1, wanted => sub { push @sidecars, $_ if /\.ref\.expect$/ && -f $_ } }, @ARGV);
my $changed = 0;
for my $sidecar (sort @sidecars)
{
	(my $base = $sidecar) =~ s/\.ref\.expect$//;
	my $input = "$base.t";
	my ($reference, $machine);
	if (-f "$base.ref.mir") { ($reference, $machine) = ("$base.ref.mir", 1); }
	elsif (-f "$base.ref") { ($reference, $machine) = ("$base.ref", 0); }
	else { print "no reference output for $sidecar\n"; next; }

	my $old = slurp($sidecar);
	my @blocks = blocks_of($old);
	my ($old_budget) = map { $_->[1] } grep { $_->[0] eq 'budget' } @blocks;
	my ($old_outcome) = map { $_->[1] } grep { $_->[0] eq 'outcome' } @blocks;
	my ($old_contract) = map { $_->[1] } grep { $_->[0] eq 'contract' } @blocks;
	my @hand = map { $_->[1] } grep { $_->[0] eq 'hand' } @blocks;

	my @out;
	if (defined($old_budget) || $o{'add-budgets'})
	{
		my %old_bound;
		for my $line (@{$old_budget || []})
		{
			my ($key, $bound) = budget_key($line);
			$old_bound{$key} = $bound if defined($key);
		}
		my @budget = generate($^X, "$FindBin::Bin/make_ir_expect.pl", $reference, $o{tolerance});
		for my $line (@budget)
		{
			my ($key, $bound) = budget_key($line);
			next if !defined($key) || !exists($old_bound{$key}) || $old_bound{$key} <= $bound;
			$line =~ s/(<=|<)\s*\d+/$1 $old_bound{$key}/;
		}
		push @out, [@budget];
	}
	if (!$machine && -f $input)
	{
		my @outcome = generate($^X, "$FindBin::Bin/make_ir_outcome.pl", $input, $reference);
		push @out, [@outcome] if @outcome;
	}
	elsif (defined($old_outcome))
	{
		push @out, $old_outcome;
	}
	if ($machine && defined($old_contract))
	{
		my @flags;
		push @flags, '--preserve' if grep { /^preserve\s*<=/ } @$old_contract;
		push @flags, '--memory' if grep { /^count\((?:load|store)\)\s*>=/ } @$old_contract;
		my @contract = generate($^X, "$FindBin::Bin/make_ir_contract.pl", @flags, $reference);
		push @out, [@contract] if @contract > 1;
	}
	push @out, @hand;
	my $new = join("\n\n", map { join("\n", @$_) } @out) . "\n";
	next if $new eq $old;
	$changed++;
	print "changed: $sidecar\n";
	if ($o{write})
	{
		open(my $fh, '>', $sidecar) or die "cannot write $sidecar: $!\n";
		print $fh $new;
		close($fh);
	}
}
print "$changed of " . scalar(@sidecars) . " sidecars " . ($o{write} ? 'rewritten' : 'would change') . "\n";
