#!/usr/bin/perl

# The LowIR contract audit: every public LowIR spelling has a reviewed ledger
# row (scripts/architecture/lowir-contract-ledger.tsv), is documented in
# pa8/lowir.md, and is both read and written by the implementation.
#
# The LowIR vocabulary is the shipped scaffold headers' enums and spelling
# tables: the metadata in dev/src/ir_symbol_model.h, and the types, opcodes,
# operations, structural keywords and instruction-fact words in
# dev/src/lowir_model.h.  The implementation reads text with
# dev/src/lowir/text_parser*.cpp and writes it with
# dev/src/lowir/text_writer.cpp, both through those tables
# (scripts/audit_scaffold_usage.pl and scripts/audit_scaffold_spellings.pl
# check that they build on the headers and keep no words of their own).

use strict;
use warnings;

use Cwd qw(abs_path);
use FindBin;

my $root = abs_path("$FindBin::Bin/..");

sub read_file
{
	my ($path) = @_;
	open(my $fh, '<', "$root/$path") or die "Unable to read $path: $!\n";
	local $/;
	my $data = <$fh>;
	close($fh) or die "Unable to close $path: $!\n";
	return defined($data) ? $data : '';
}

my $scaffold = read_file('dev/src/ir_symbol_model.h');
my $vocabulary = read_file('dev/src/lowir_model.h');
my $parser = read_file('dev/src/lowir/text_parser.cpp') .
	read_file('dev/src/lowir/text_parser_body.cpp');
my $writer = read_file('dev/src/lowir/text_writer.cpp');
my $documentation = read_file('pa8/lowir.md');
my $ledger = read_file('scripts/architecture/lowir-contract-ledger.tsv');

my @errors;
my @rows;
my %row_identity;
my @lines = split /\n/, $ledger;
my $header = shift @lines;
push @errors, "ledger header does not name the eight contract columns"
	if !defined($header) || $header ne
		"category\tsurface\tproducer\tconsumer\towner\tproperty_test\tdisposition\tevidence";

for (my $index = 0; $index < scalar(@lines); ++$index)
{
	next if $lines[$index] eq '';
	my @fields = split /\t/, $lines[$index], -1;
	my $line_number = $index + 2;
	if (scalar(@fields) != 8)
	{
		push @errors, "ledger line $line_number has " . scalar(@fields) .
			" fields instead of 8";
		next;
	}
	my %row;
	@row{qw(category surface producer consumer owner property disposition evidence)} =
		@fields;
	$row{line} = $line_number;
	my $identity = "$row{category}\t$row{surface}";
	push @errors, "duplicate ledger surface at line $line_number: $identity"
		if $row_identity{$identity}++;
	push @errors, "unresolved ledger disposition '$row{disposition}' at line $line_number"
		if $row{disposition} ne 'keep' && $row{disposition} ne 'remove';
	push @rows, \%row;
}

my @retained = grep { $_->{disposition} eq 'keep' } @rows;
for my $row (@retained)
{
	push @errors, "retained $row->{category} '$row->{surface}' has no semantic producer"
		if $row->{producer} eq '' || $row->{producer} eq 'none' ||
			$row->{producer} eq 'removed' ||
			$row->{producer} eq 'explicit LowIR only';
	push @errors, "retained $row->{category} '$row->{surface}' has no non-transport consumer"
		if $row->{consumer} eq '' || $row->{consumer} eq 'none' ||
			$row->{consumer} =~ /^transport\b/;

	for my $owner (split /;/, $row->{owner})
	{
		push @errors, "retained $row->{category} '$row->{surface}' has missing owner $owner"
			if $owner eq '' || !-f "$root/$owner";
	}
	for my $property (split /;/, $row->{property})
	{
		push @errors, "retained $row->{category} '$row->{surface}' has missing property $property"
			if $property eq '' || $property eq 'MISSING' ||
				!-f "$root/$property";
	}
}

my %count = (values => 0, keys => 0, roles => 0, instructions => 0, types => 0);

# The string literals of `name[...] = { ... };` in $text.
sub table_strings
{
	my ($text, $name, $where) = @_;
	if ($text !~ /\b\Q$name\E\[[^\]]*\]\s*=\s*\{(.*?)\};/s)
	{
		push @errors, "cannot locate the spelling table $name in $where";
		return ();
	}
	my $body = $1;
	return $body =~ /"([^"]*)"/g;
}

# 1. Each public scaffold enum and its spelling table agree value for value:
# the default enumerator has no spelling (it is the omitted metadata) and
# every other one is spelled as its lower-cased name without the prefix.
my %table_for_enum = (
	SymbolRole => 'kRoles',
	LanguageLinkageMode => 'kLinkages',
	SymbolBindingMode => 'kBindings',
	GlobalStorageMode => 'kStorages',
	CallArityMode => 'kArities',
	CallEffectsMode => 'kEffects',
	CallUnwindMode => 'kUnwinds',
	CallReturnMode => 'kReturns',
	ParamPassingMode => 'kPasses',
	ParamAliasMode => 'kAliases',
	IndexProjectionKind => 'kProjections',
);
my %enum_spellings;
while ($scaffold =~ /enum\s+(\w+)\s*(?::\s*[\w:]+\s*)?\{(.*?)\};/sg)
{
	my ($enum, $body) = ($1, $2);
	my $table = $table_for_enum{$enum};
	next if !defined($table);
	my @names = $body =~ /\b([A-Z]+_[A-Z0-9_]+)\b/g;
	my @spelled = table_strings($scaffold, $table, 'ir_symbol_model.h');
	next if !@spelled;
	push @errors, "$enum has " . scalar(@names) . " enumerators but $table has " .
		scalar(@spelled) . " spellings"
		if scalar(@names) != scalar(@spelled);
	push @errors, "$enum default $names[0] is spelled '$spelled[0]'; it must be omitted"
		if $spelled[0] ne '';
	for my $index (1 .. $#names)
	{
		(my $want = lc $names[$index]) =~ s/^[a-z]+_//;
		my $have = $index < scalar(@spelled) ? $spelled[$index] : '<none>';
		push @errors, "$names[$index] is spelled '$have' in $table; the scaffold implies '$want'"
			if $have ne $want;
		++$count{values};
	}
	$enum_spellings{$enum} = [@spelled[1 .. $#spelled]];
}
for my $enum (sort keys %table_for_enum)
{
	push @errors, "ir_symbol_model.h has no enum $enum" if !$enum_spellings{$enum};
}

# 2. Metadata keys: the key table and the words the parser recognizes as keys
# against the ledger, pa8/lowir.md and the writer.
my %key_source = map { $_ => 'table' }
	table_strings($scaffold, 'kKeys', 'ir_symbol_model.h');
while ($parser =~ /(?:at_word\("|memcmp\([^,]+,\s*")([a-z_]+)"/g)
{
	$key_source{$1} //= 'parser';
}
my %ledger_key;
for my $row (@retained)
{
	next if $row->{category} ne 'metadata';
	(my $key = $row->{surface}) =~ s/=.*//;
	$ledger_key{$key} = 1;
}
for my $key (sort keys %ledger_key)
{
	push @errors, "retained ledger metadata key '$key' is not recognized by the parser"
		if !$key_source{$key};
}
for my $key (sort keys %key_source)
{
	# Other words the parser matches (`zero`, `copy`, ...) are not keys.
	next if $key_source{$key} eq 'parser' && !$ledger_key{$key};
	push @errors, "metadata key '$key' has no retained ledger row"
		if !$ledger_key{$key};
	push @errors, "metadata key '$key' is absent from pa8/lowir.md"
		if $documentation !~ /\b\Q$key\E\b/;
	push @errors, "metadata key '$key' is not written by lowir/text_writer.cpp"
		if $key_source{$key} eq 'parser' && $writer !~ /\Q$key\E=/;
	++$count{keys};
}

# 3. Roles: the scaffold's SymbolRole spellings against the ledger and
# pa8/lowir.md.
my %role = map { $_ => 1 } @{$enum_spellings{SymbolRole} || []};
my %ledger_role = map { $_->{surface} => $_->{disposition} }
	grep { $_->{category} eq 'role' } @rows;
for my $name (sort keys %role)
{
	push @errors, "role '$name' has no retained ledger row"
		if ($ledger_role{$name} // '') ne 'keep';
	push @errors, "role '$name' is absent from pa8/lowir.md"
		if $documentation !~ /\b\Q$name\E\b/;
	++$count{roles};
}
for my $name (sort keys %ledger_role)
{
	push @errors, "retained ledger role '$name' is not a SymbolRole of ir_symbol_model.h"
		if $ledger_role{$name} eq 'keep' && !$role{$name};
}

# 4. Instructions: every word of a retained instruction row is a vocabulary
# word: an opcode head, a unary operation, a structural keyword, a storage
# word or an instruction-fact word, which the parser classifies and the
# writer spells through the scaffold tables.  Every opcode head has a
# retained row.
my @opcodes = table_strings($vocabulary, 'kOpcodes', 'lowir_model.h');
my @fact_words;
if ($vocabulary =~ /spelling\(InstructionFlag \w+\)\s*\{(.*?)\n\}/s)
{
	@fact_words = $1 =~ /return\s+"([^"]+)"/g;
}
push @errors, 'cannot locate the instruction-fact words of lowir_model.h' if !@fact_words;
my %keyword = map { $_ => 1 } @opcodes, @fact_words,
	table_strings($vocabulary, 'kKeywords', 'lowir_model.h'),
	table_strings($vocabulary, 'kUnaryOps', 'lowir_model.h'),
	grep { $_ ne '' } table_strings($scaffold, 'kStorages', 'ir_symbol_model.h');
my %ledger_head;
for my $row (@retained)
{
	next if $row->{category} ne 'instruction';
	for my $form (split /\|/, $row->{surface})
	{
		my @words = split /\s+/, $form;
		$ledger_head{$words[0]} = 1 if @words;
		for my $word (@words)
		{
			next if $word eq 'clause';
			push @errors, "ledger instruction word '$word' is not a LowIR vocabulary word"
				if !$keyword{$word};
		}
		++$count{instructions};
	}
}
for my $head (@opcodes)
{
	push @errors, "opcode '$head' has no retained ledger instruction row" if !$ledger_head{$head};
}

# 5. Types: every retained type row is a type spelling of lowir_model.h, and
# every type spelling has a retained row.
my %type = map { $_ => 1 } table_strings($vocabulary, 'kTypeKinds', 'lowir_model.h');
my %ledger_type;
for my $row (@retained)
{
	next if $row->{category} ne 'type';
	(my $name = $row->{surface}) =~ s/<.*//;
	$ledger_type{$name} = 1;
	push @errors, "ledger type '$name' has no spelling in lowir_model.h"
		if !$type{$name};
	++$count{types};
}
for my $name (sort keys %type)
{
	push @errors, "type '$name' has no retained ledger row" if !$ledger_type{$name};
}

if (@errors)
{
	print STDERR "LowIR contract audit: FAIL (" . scalar(@errors) .
		" issue(s))\n";
	print STDERR "  $_\n" for @errors;
	exit 1;
}

print "LowIR contract audit: PASS (" . scalar(@rows) . " ledger rows, " .
	scalar(@retained) . " retained, $count{values} public enum values, " .
	"$count{keys} metadata keys, $count{roles} roles, " .
	"$count{instructions} instruction forms, $count{types} types)\n";
