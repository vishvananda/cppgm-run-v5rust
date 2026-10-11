#!/usr/bin/perl

# The LowIR text writer and reader and the MIR dumper spell their words from
# the shipped vocabulary headers (lowir_model.h, ir_symbol_model.h,
# mir_model.h, native/mir/registers.h): a word chosen by a value (an opcode
# head, an operation, a metadata key or value, a register) always comes from
# a header's table.  A fixed word at a fixed call site may stay a literal
# ("declare global ", "  slot $"), provided it is a header spelling; the
# format's own section words are listed below.  This audit checks every word
# of those literals, so the literals cannot drift from the vocabulary.
#
# Literals that are not output words are skipped: format strings, file modes,
# includes and diagnostic messages.
#
# Usage: audit_scaffold_spellings.pl [--root DIR]

use strict;
use warnings;

use Cwd qw(abs_path);
use FindBin;
use Getopt::Long;

my $root = abs_path("$FindBin::Bin/..");
GetOptions('root=s' => \$root) or die "usage: $0 [--root DIR]\n";
my $src = "$root/dev/src";

my @vocabulary = qw(ir_symbol_model.h lowir_model.h mir_model.h native/mir/registers.h);

# Fixed format words, not vocabulary: LowIR's `!dbg(file, line, column)`
# marker, and the MIR dump's section and field words.
my %fixed_lowir = map { $_ => 1 } qw(dbg);
my %fixed_mir = map { $_ => 1 } qw(
	machine_ir x86_64 linux startup storage data scalar item init abi param frame stack_size scratch_bytes
	frame_pointer omit keep epilogues duplicated shared aligned_base preserve atexit_drain args stack switch
	_dynamic return global function block i128 dbg);

# The code just before a literal that is not an output word.
my @skip_contexts = (qr/snprintf\([^;]*,\s*\z/, qr/fopen\([^;]*,\s*\z/, qr/#\s*include\s*\z/,
	qr/internal_error\(\s*\z/, qr/InternalError\(\s*\z/, qr/throw\s+[\w:]+\(\s*\z/, qr/fail\(\s*\z/,
	qr/static_assert\([^;]*,\s*\z/, qr/strpbrk\([^;]*,\s*\z/);

sub read_text
{
	my ($path) = @_;
	open(my $fh, '<', $path) or die "unable to read $path: $!\n";
	local $/;
	my $text = <$fh>;
	close($fh);
	return $text;
}

# The string literals of C++ source text outside comments, each with the
# code before it (comments removed).
sub literals
{
	my ($text) = @_;
	my @out;
	my $code = '';
	while ($text =~ m{\G(?:(//[^\n]*)|(/\*.*?\*/)|("(?:[^"\\\n]|\\.)*")|('(?:[^'\\\n]|\\.)*')|([^/"']+|/))}gsc)
	{
		if (defined $3)
		{
			my $body = substr($3, 1, -1);
			push @out, [$body, substr($code, -120)];
			$code .= $3;
		}
		elsif (defined $4 || defined $5)
		{
			$code .= defined $4 ? $4 : $5;
		}
	}
	return @out;
}

sub words_of
{
	my ($literal) = @_;
	$literal =~ s/\\[nt]/ /g;
	return $literal =~ /([A-Za-z_][A-Za-z0-9_-]*)/g;
}

# Every word of the vocabulary headers' literals.
my (%spellings, %words);
for my $header (@vocabulary)
{
	for my $literal (literals(read_text("$src/$header")))
	{
		my ($body, $before) = @$literal;
		next if grep { $before =~ $_ } @skip_contexts;
		$spellings{$body} = 1;
		$words{$_} = 1 for words_of($body);
	}
}

my @problems;
my %checked;

sub check_words
{
	my ($label, $fixed, @found) = @_;
	my %seen;
	for my $word (map { words_of($_) } @found)
	{
		next if $seen{$word}++;
		++$checked{$label};
		next if $words{$word} || $fixed->{$word};
		# A named literal with its floating suffix (`INFINITYL`, `nanf`).
		next if $word =~ /\A(.+)[fL]\z/ && $spellings{$1};
		push @problems, "$label: word '$word' is neither a vocabulary spelling nor a fixed format word";
	}
}

sub output_literals
{
	my (@files) = @_;
	my @found;
	for my $file (@files)
	{
		for my $literal (literals(read_text("$src/$file")))
		{
			my ($body, $before) = @$literal;
			push @found, $body if !grep { $before =~ $_ } @skip_contexts;
		}
	}
	return @found;
}

check_words('LowIR writer', \%fixed_lowir, output_literals('lowir/text_writer.cpp', 'lowir/literals.cpp'));
check_words('MIR dumper', \%fixed_mir, output_literals('native/mir/mir_writer.cpp'));

# The reader's matches outside the vocabulary tables.
my @reader;
for my $file ('lowir/text_parser.cpp', 'lowir/text_parser_body.cpp', 'lowir/text_lexer.cpp')
{
	my $text = read_text("$src/$file");
	push @reader, $text =~ /at_word\("([^"]*)"\)/g;
	push @reader, $text =~ /memcmp\([^,]+,\s*"([^"]*)"/g;
	push @reader, $text =~ /expect_keyword\([^,]+,\s*"([^"]*)"\)/g;
	push @reader, $text =~ /starts_with\([^,]+,[^,]+,\s*"([^"]*)"\)/g;
}
check_words('LowIR reader', \%fixed_lowir, @reader);

if (@problems)
{
	print STDERR "Scaffold spelling audit: FAIL (" . scalar(@problems) . " issue(s))\n";
	print STDERR "  $_\n" for @problems;
	exit 1;
}
printf "Scaffold spelling audit: PASS (words of the LowIR writer %d, LowIR reader %d, MIR dumper %d are vocabulary spellings or fixed format words)\n",
	$checked{'LowIR writer'} // 0, $checked{'LowIR reader'} // 0, $checked{'MIR dumper'} // 0;
