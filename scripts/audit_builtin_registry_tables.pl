#!/usr/bin/perl

# The GNU builtin tables of dev/src/lex/gnu_builtins.cpp.
#
# FindGnuBuiltin answers a builtin lookup with a binary search over
# kGnuBuiltins, so that table has to stay sorted by spelling (strcmp order),
# and kLibmFunctions keeps the same order.  An entry added in the wrong place
# does not fail to build and does not fail most tests: the builtin simply
# stops being recognised, and the failure surfaces far away as an unknown name
# in whichever header happened to use it.
#
# FindGnuTrait returns the position of a spelling in kTraitNames as a
# GnuTrait, so the table has to list the traits in the enum's order.  A
# mismatch does not fail to build either (a static_assert checks only the
# count): every later trait silently answers as some other one.

use strict;
use warnings;

use Cwd qw(abs_path);
use FindBin;

my $root = abs_path("$FindBin::Bin/..");
my $source = 'dev/src/lex/gnu_builtins.cpp';
my $header = 'dev/src/lex/gnu_builtins.h';

sub read_lines
{
	my ($path) = @_;
	open(my $fh, '<', "$root/$path") or die "unable to read $path: $!\n";
	my @lines = <$fh>;
	close($fh);
	return @lines;
}

my @lines = read_lines($source);
my @errors;

# 1. The sorted tables.
my %sorted = map { $_ => 1 } qw(kGnuBuiltins kLibmFunctions);
my %entries;
my ($table, $table_line, $previous);
for (my $i = 0; $i < @lines; ++$i)
{
	my $line = $lines[$i];
	if ($line =~ /(?:^|\}\s*)(k\w+)\[\]\s*=\s*\{/)
	{
		$table = $sorted{$1} ? $1 : undef;
		$table_line = $i + 1;
		$previous = undef;
		$entries{$table} = 0 if defined($table);
		next;
	}
	next if !defined($table);
	if ($line =~ /^\s*\};/)
	{
		$table = undef;
		next;
	}
	while ($line =~ /\{\s*"([^"]+)"/g)
	{
		my $spelling = $1;
		++$entries{$table};
		push @errors, "$table (declared at line $table_line): \"$spelling\" at line " .
			($i + 1) . " sorts before \"$previous\""
			if defined($previous) && $spelling lt $previous;
		$previous = $spelling;
	}
}
for my $name (sort keys %sorted)
{
	push @errors, "$source has no table $name" if !$entries{$name};
}

# 2. kTraitNames against the GnuTrait enum.  A spelling names its enumerator
# when both reduce to the same letters (`__is_class`, IsClass); the aliases
# list the spellings that do not.
my %trait_alias = (
	'__is_nullptr' => 'IsNullPointer',
	'__builtin_offsetof' => 'Offsetof',
	'__remove_reference_t' => 'RemoveReference',
	'__remove_reference' => 'RemoveReferenceGnu',
	'_Atomic' => 'AddAtomic',
	'__builtin_va_arg' => 'VaArg',
	'__builtin_convertvector' => 'ConvertVector',
	'__builtin_bit_cast' => 'BitCast',
);
my $text = join('', @lines);
my @trait_names;
if ($text =~ /\bkTraitNames\[\]\s*=\s*\{(.*?)\};/s)
{
	my $body = $1;
	@trait_names = $body =~ /"([^"]+)"/g;
}
else
{
	push @errors, "$source has no table kTraitNames";
}
my $header_text = join('', read_lines($header));
my @traits;
if ($header_text =~ /enum class GnuTrait\b[^{]*\{(.*?)\};/s)
{
	my $body = $1;
	$body =~ s{//[^\n]*}{}g;
	@traits = $body =~ /\b([A-Z]\w*)\b/g;
}
else
{
	push @errors, "$header has no enum GnuTrait";
}
sub letters
{
	my ($name) = @_;
	(my $reduced = lc $name) =~ s/[^a-z0-9]//g;
	return $reduced;
}
for (my $i = 0; $i < @trait_names; ++$i)
{
	my $want = $trait_alias{$trait_names[$i]};
	my $have = $i < @traits ? $traits[$i] : '<none>';
	my $matches = defined($want) ? $have eq $want : letters($trait_names[$i]) eq letters($have);
	next if $matches;
	push @errors, "kTraitNames entry $i is \"$trait_names[$i]\" but GnuTrait $i is $have";
	last;
}

if (@errors)
{
	print STDERR "builtin registry table audit: FAIL\n";
	print STDERR "  $_\n" for @errors;
	exit 1;
}

print "builtin registry table audit: PASS (" .
	join(', ', map { "$_ $entries{$_} entries" } sort keys %entries) .
	" in order; " . scalar(@trait_names) . " trait spellings in GnuTrait order)\n";
exit 0;
