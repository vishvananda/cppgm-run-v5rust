#!/usr/bin/perl
# Write the contract lines of an expectation sidecar from a machine-IR dump:
# the exception-region structure, the calls, the callee-saved preservation
# and the memory accesses each function keeps.  These are the facts an
# unwinder, a caller or a volatile access depends on, independent of any
# placement or selection choice.
#
#   make_ir_contract.pl [--preserve] [--memory] <reference.mir>
#
# By default only the exception regions and the calls are contract: a
# different allocator may keep more or fewer values in registers.  Pass
# --preserve when the fixture is about callee-saved pruning (a ceiling on
# the preserve list) and --memory when it is about accesses that must be
# emitted (floors on loads and stores, as for volatile accesses).
use strict;
use warnings;

my ($with_preserve, $with_memory) = (0, 0);
while (@ARGV && $ARGV[0] =~ /^--/)
{
	my $flag = shift @ARGV;
	if ($flag eq '--preserve') { $with_preserve = 1; }
	elsif ($flag eq '--memory') { $with_memory = 1; }
	else { die "unknown option $flag\n"; }
}
my ($path) = @ARGV;
die "usage: make_ir_contract.pl [--preserve] [--memory] <reference.mir>\n" if !defined($path);
open(my $fh, '<', $path) or die "Unable to read $path: $!\n";
my @lines = <$fh>;
close($fh);
chomp(@lines);

my @order;
my %facts;
my $function = '';
my $section = '';
for my $line (@lines)
{
	if ($line =~ /^function\s+\@([^\s(]+)/)
	{
		$function = $1;
		$section = '';
		push @order, $function;
		$facts{$function} = { eh_push => 0, eh_pop => 0, calls => {}, preserve => 0, loads => 0, stores => 0 };
		next;
	}
	if ($line =~ /^(?:declare|global|machine_ir|startup)\b/) { $function = ''; $section = ''; next; }
	next if $function eq '';
	if ($line =~ /^\s+block\s+\^/) { $section = 'block'; next; }
	if ($line =~ /^\s{2}(abi|frame)\s*$/) { $section = $1; next; }
	if (($section eq 'abi' || $section eq 'frame') && $line =~ /^\s+preserve\b(.*)$/)
	{
		$facts{$function}{preserve} = scalar(grep { $_ ne '' } split(/\s+/, $1));
		next;
	}
	next if $section ne 'block' || $line !~ /^\s{4}(\S.*)$/;
	my $body = $1;
	my ($mnemonic) = split(/[\s,]+/, $body);
	$mnemonic =~ s/\..*$//;
	++$facts{$function}{eh_push} if $mnemonic eq 'eh_push';
	++$facts{$function}{eh_pop} if $mnemonic eq 'eh_pop';
	++$facts{$function}{loads} if $mnemonic eq 'load';
	++$facts{$function}{stores} if $mnemonic eq 'store';
	++$facts{$function}{calls}{$1} if $mnemonic eq 'call' && $body =~ /\@(\S+)/;
}

my $name = $path;
$name =~ s{.*/}{};
print "# contract from $name: the exception regions, calls, callee-saved registers and memory accesses the fixture requires\n";
for my $function (@order)
{
	my $f = $facts{$function};
	print "count(eh_push) == $f->{eh_push} in \@$function\n" if $f->{eh_push};
	print "count(eh_pop) == $f->{eh_pop} in \@$function\n" if $f->{eh_pop};
	for my $callee (sort keys %{$f->{calls}})
	{
		print "count(call \@$callee) == $f->{calls}{$callee} in \@$function\n";
	}
	print "preserve <= $f->{preserve} in \@$function\n" if $with_preserve;
	print "count(load) >= $f->{loads} in \@$function\n" if $with_memory && $f->{loads};
	print "count(store) >= $f->{stores} in \@$function\n" if $with_memory && $f->{stores};
}
