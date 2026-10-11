#!/usr/bin/perl
# Derive outcome lines for an expectation sidecar from what an optimizer
# did to a LowIR fixture: the calls, loads, stores, phis and definitions
# the course solution removed must stay removed.  The lines are facts about
# the input and the output, not about the pass that produced them, so any
# design that reaches the same outcome passes.
#
#   make_ir_outcome.pl <input.t> <output.lowir>
use strict;
use warnings;

my ($input_path, $output_path) = @ARGV;
die "usage: make_ir_outcome.pl <input.t> <output.lowir>\n"
	if !defined($input_path) || !defined($output_path);

sub facts
{
	my ($path) = @_;
	open(my $fh, '<', $path) or die "Unable to read $path: $!\n";
	my %f = (functions => {}, callees => {}, kinds => {}, per_function_callees => {});
	my $function = '';
	while (my $line = <$fh>)
	{
		chomp($line);
		if ($line =~ /^function\s+\@([^\s(]+)/) { $function = $1; $f{functions}{$1} = 1; next; }
		if ($line =~ /^(?:declare|global)\b/ || $line =~ /^\}/) { $function = ''; next; }
		next if $function eq '' || $line !~ /^\s{4}(\S.*)$/;
		my $body = $1;
		$body =~ s/^%[^\s=]+\s*=\s*//;
		my ($kind) = split(/[\s,()]+/, $body);
		next if !defined($kind);
		++$f{kinds}{$kind};
		if ($kind eq 'call' && $body =~ /\@([^\s(]+)/)
		{
			++$f{callees}{$1};
			++$f{per_function_callees}{$function}{$1};
		}
	}
	close($fh);
	return \%f;
}

my $in = facts($input_path);
my $out = facts($output_path);
my $name = $output_path;
$name =~ s{.*/}{};
my @lines;
for my $callee (sort keys %{$in->{callees}})
{
	next if exists($out->{callees}{$callee});
	next if !exists($in->{functions}{$callee});   # only calls into the fixture's own definitions
	push @lines, "none(call \@$callee)";
}
for my $function (sort keys %{$in->{per_function_callees}})
{
	next if !exists($out->{functions}{$function});
	for my $callee (sort keys %{$in->{per_function_callees}{$function}})
	{
		next if !exists($in->{functions}{$callee});
		next if !exists($out->{callees}{$callee});      # gone everywhere: covered above
		next if exists($out->{per_function_callees}{$function}{$callee});
		push @lines, "none(call \@$callee) in \@$function";
	}
}
for my $kind (qw(load store phi copyobj zeroinit))
{
	push @lines, "none($kind)" if $in->{kinds}{$kind} && !$out->{kinds}{$kind};
}
for my $function (sort keys %{$in->{functions}})
{
	push @lines, "absent(\@$function)" if !exists($out->{functions}{$function});
}
exit(0) if !@lines;
print "# outcomes from $name: what the course solution removed stays removed (a call inlined, memory promoted, a definition pruned)\n";
print "$_\n" for @lines;
