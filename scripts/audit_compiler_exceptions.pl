#!/usr/bin/perl

# The compiler exception audit ("E8") over dev/: the implementation's error
# types and the phases that throw them are listed below.
#
# Policy (spec.md sections 1 and 3, AGENTS.md):
# - only typed exceptions are thrown: no std::logic_error, runtime_error,
#   length_error or other generic standard type, and no helper returning one;
# - every thrown type is one of the compiler's error types, thrown by the
#   phase that owns it; std::bad_alloc only by the two allocators that
#   implement the allocator protocol;
# - no catch (...), and no catch of a generic standard type inside dev/src:
#   cleanup is scoped ownership (RAII), not catch-and-rethrow;
# - a tool's main may end with one catch of std::exception, its last-resort
#   boundary, paired with its catch of InternalError;
# - no exceptions as control flow: nothing is thrown or caught that is not
#   an error, and nothing caught is rethrown;
# - diagnostic text is presentation: searching or comparing what() is
#   forbidden.

use strict;
use warnings;

use Cwd qw(abs_path);
use File::Find qw(find);
use File::Spec;
use FindBin;
use Getopt::Long qw(GetOptions);

my $quiet = 0;
GetOptions('quiet' => \$quiet) or die "usage: $0 [--quiet]\n";
die "usage: $0 [--quiet]\n" if @ARGV;

my $root = abs_path("$FindBin::Bin/..");

my %ceiling = (
	logic_throw => 0,
	runtime_throw => 0,
	generic_throw_files => 0,
	generic_return_helper => 0,
	catch_all => 0,
	internal_standard_catch => 0,
	legacy_not_implemented => 0,
	message_policy => 0,
	terminal_untyped_standard_catch => 0,
	foreign_explicit_throw => 0,
	foreign_catch => 0,
	rethrow => 0,
);

# The error types and where they may be thrown (a path prefix under the
# repository root; '' anywhere). Namespace qualifiers other than std:: are
# ignored.
my %explicit_throw_type = (
	'InternalError' => '',
	'InvocationError' => '',
	'InputOutputError' => '',
	'LimitError' => '',
	'SourceError' => 'dev/src/preprocess/',
	'PreprocessError' => 'dev/src/preprocess/',
	'ParseError' => 'dev/src/syntax/',
	'SemanticError' => 'dev/src/sema/',
	'LoweringError' => 'dev/src/lower/',
	'Failure' => 'dev/src/lowir/',
	'FactError' => 'dev/src/abi/facts/',
	'EncodingError' => 'dev/src/abi/itanium/',
	'BackendError' => 'dev/src/native/',
	'std::bad_alloc' => 'dev/src/support/memory/',
);

# `throw <unit>.locate(e)`: an InternalError rethrown as a new error with its
# translation-unit location rendered (syntax::SyntaxUnit::locate).
my $locate_type = 'locate';

# Every generic standard catch is a last-resort executable boundary paired
# with an InternalError catch. Counts make additions and moves explicit.
my %terminal_allowlist = (
	'dev/abimangle.cpp' => 1,
	'dev/cppgm++.cpp' => 1,
	'dev/ppexpr.cpp' => 1,
	'dev/lowir.cpp' => 1,
	'dev/lowir2native.cpp' => 1,
	'dev/lowiropt.cpp' => 1,
	'dev/posttoken.cpp' => 1,
	'dev/pptoken.cpp' => 1,
	'dev/preproc.cpp' => 1,
);

# The allocator protocol's std::bad_alloc: the arena and the zeroed
# allocator, one throw each.
my %bad_alloc_allowlist = (
	'dev/src/support/memory/arena.cpp' => 1,
	'dev/src/support/memory/zeroed_allocator.h' => 1,
);

sub read_text
{
	my ($path) = @_;
	open(my $fh, '<', $path) or die "unable to read $path: $!\n";
	local $/;
	my $text = <$fh>;
	close($fh) or die "unable to close $path: $!\n";
	return defined($text) ? $text : '';
}

sub mask_segment
{
	my ($segment) = @_;
	$segment =~ s/[^\n]/ /g;
	return $segment;
}

sub mask_comments_and_literals
{
	my ($text) = @_;
	$text =~ s{
		(?:u8|u|U|L)?R"([^\s()\\]{0,16})\(
		(.*?)
		\)\1"
	}{mask_segment($&)}gsex;
	$text =~ s{
		//[^\n]*
		|/\*.*?\*/
		|"(?:\\.|[^"\\])*"
		|'(?:\\.|[^'\\])*'
	}{mask_segment($&)}gsex;
	return $text;
}

sub relative_path
{
	my ($path) = @_;
	my $relative = File::Spec->abs2rel($path, $root);
	$relative =~ s{\\}{/}g;
	return $relative;
}

sub line_number
{
	my ($text, $offset) = @_;
	return 1 + (substr($text, 0, $offset) =~ tr/\n/\n/);
}

# A type as written, without spaces and without namespace qualifiers other
# than std::.
sub type_name
{
	my ($type) = @_;
	$type =~ s/\s+//g;
	return $type if $type =~ /\Astd::\w+\z/;
	$type =~ s/\A.*:://;
	return $type;
}

my @files;
find({
	wanted => sub {
		return if !-f $_;
		return if $_ !~ /\.(?:c|cc|cpp|cxx|h|hh|hpp|hxx)\z/;
		# The student scaffolds' not-implemented exception is theirs, not
		# the compiler's.
		return if $File::Find::name =~ m{/support/not_implemented\.h\z};
		push @files, $File::Find::name;
	},
	no_chdir => 1,
}, "$root/dev/src");
for my $path (glob "$root/dev/*")
{
	# The student scaffolds are the export's starting points, not the
	# implementation; they are outside this audit.
	next if $path =~ /-scaffold\.cpp\z/;
	push @files, $path if -f $path &&
		$path =~ /\.(?:c|cc|cpp|cxx|h|hh|hpp|hxx)\z/;
}
@files = sort @files;

my %count = map { $_ => 0 } keys %ceiling;
my %generic_file;
my @message_policy;
my @cleanup_catch;
my @terminal_catch;
my %terminal_seen;
my @foreign_explicit_throw;
my @foreign_catch;
my @internal_standard_catch;
my @rethrow;
my $explicit_throw_sites = 0;
my $typed_catch_sites = 0;
my %bad_alloc_seen;

for my $path (@files)
{
	my $relative = relative_path($path);
	my $code = mask_comments_and_literals(read_text($path));
	my $internal = $relative =~ m{\Adev/src/};
	my ($file_logic, $file_runtime) = (0, 0);
	my ($file_terminal_standard, $file_terminal_typed) = (0, 0);

	$count{legacy_not_implemented} += () =
		$code =~ /\b(?:NotImplementedException|CPPGM_EXIT_NOT_IMPLEMENTED)\b/g;

	while ($code =~ /\bthrow\b(\s*;)?/g)
	{
		next if !defined($1);
		++$count{rethrow};
		push @rethrow, "$relative:" . line_number($code, $-[0]);
	}

	while ($code =~ /\bthrow\s+(?!;)\s*
		(?:(?:[A-Za-z_]\w*\s*\.\s*)?(locate)\s*\(
		|((?:[A-Za-z_]\w*\s*::\s*)*[A-Za-z_]\w*))/gx)
	{
		++$explicit_throw_sites;
		my $line = line_number($code, $-[0]);
		my $type = defined($1) ? $locate_type : type_name($2);
		if ($type eq $locate_type)
		{
			next;
		}
		if ($type eq 'std::bad_alloc')
		{
			++$bad_alloc_seen{$relative};
		}
		if ($type =~ /\A(?:std::)?logic_error\z/)
		{
			++$count{logic_throw};
			++$file_logic;
			$generic_file{$relative} = 1;
		}
		elsif ($type =~ /\A(?:std::)?runtime_error\z/)
		{
			++$count{runtime_throw};
			++$file_runtime;
			$generic_file{$relative} = 1;
		}
		my $scope = $explicit_throw_type{$type};
		if (!defined($scope) || index($relative, $scope) != 0)
		{
			++$count{foreign_explicit_throw};
			push @foreign_explicit_throw, "$relative:$line ($type)";
		}
	}
	print "$relative logic=$file_logic runtime=$file_runtime\n"
		if $ENV{CPPGM_EXCEPTION_AUDIT_VERBOSE} &&
		($file_logic || $file_runtime);

	while ($code =~ /\breturn\s+(?:std\s*::\s*)?
		(?:logic_error|runtime_error|length_error|out_of_range|
			invalid_argument|domain_error|range_error|overflow_error|
			underflow_error|exception)\s*[({]/gx)
	{
		++$count{generic_return_helper};
	}

	while ($code =~ /\bcatch\s*\(\s*\.\.\.\s*\)/g)
	{
		++$count{catch_all};
		push @cleanup_catch, "$relative:" . line_number($code, $-[0]);
	}

	while ($code =~ /\bcatch\s*\(\s*(?:const\s+)?
		((?:[A-Za-z_]\w*\s*::\s*)*[A-Za-z_]\w*)\s*(?:const\s*)?&/gx)
	{
		my $line = line_number($code, $-[0]);
		my $written = $1;
		$written =~ s/\s+//g;
		my $type = type_name($written);
		if ($type =~ /\A(?:std::)?exception\z/ && !$internal)
		{
			++$file_terminal_standard;
			++$terminal_seen{$relative};
			push @terminal_catch, "$relative:$line";
			next;
		}
		if ($written =~ /\Astd::/ || $type =~ /\A(?:exception|logic_error|
			runtime_error|length_error|out_of_range|invalid_argument|
			domain_error|range_error|overflow_error|underflow_error|
			bad_alloc)\z/x)
		{
			++$count{internal_standard_catch};
			push @internal_standard_catch, "$relative:$line ($written)";
			next;
		}
		if (!exists($explicit_throw_type{$type}))
		{
			++$count{foreign_catch};
			push @foreign_catch, "$relative:$line ($written)";
			next;
		}
		++$typed_catch_sites;
		++$file_terminal_typed if !$internal && $type eq 'InternalError';
	}
	if (!$internal)
	{
		$count{terminal_untyped_standard_catch} +=
			$file_terminal_standard - $file_terminal_typed
			if $file_terminal_standard > $file_terminal_typed;
	}

	# Diagnostic text may be printed at a terminal boundary. Searching or
	# comparing it is policy and is forbidden. Masked literals keep message
	# prose and test strings from becoming false positives.
	while ($code =~ /([^\n;{}]*\.what\s*\(\s*\)[^\n;{}]*)/g)
	{
		my $statement = $1;
		next if $statement !~ /(?:==|!=|\.find\s*\(|\.compare\s*\(|regex)/;
		++$count{message_policy};
		push @message_policy, "$relative:" . line_number($code, $-[1]);
	}
}

$count{generic_throw_files} = scalar(keys %generic_file);

my @error;
for my $kind (sort keys %ceiling)
{
	push @error, "$kind count $count{$kind} exceeds architecture limit $ceiling{$kind}"
		if $count{$kind} > $ceiling{$kind};
}
push @error, "exception-message policy at $_" for @message_policy;
push @error, "foreign explicit exception type at $_"
	for @foreign_explicit_throw;
push @error, "catch of a type that is not an error at $_" for @foreign_catch;
push @error, "generic standard catch at $_"
	for @internal_standard_catch;
push @error, "catch-all at $_" for @cleanup_catch;
push @error, "rethrow at $_" for @rethrow;
for my $path (sort keys %bad_alloc_seen)
{
	push @error, "unreviewed allocator-protocol bad_alloc in $path"
		if !exists($bad_alloc_allowlist{$path});
}
for my $path (sort keys %bad_alloc_allowlist)
{
	my $actual = $bad_alloc_seen{$path} || 0;
	push @error, "allocator-protocol bad_alloc count in $path is $actual, " .
		"expected $bad_alloc_allowlist{$path}"
		if $actual != $bad_alloc_allowlist{$path};
}
for my $path (sort keys %terminal_seen)
{
	push @error, "unreviewed terminal standard catch in $path"
		if !exists($terminal_allowlist{$path});
}
for my $path (sort keys %terminal_allowlist)
{
	my $actual = $terminal_seen{$path} || 0;
	push @error, "terminal catch count in $path is $actual, expected " .
		$terminal_allowlist{$path}
		if $actual != $terminal_allowlist{$path};
}

if (@error)
{
	print STDERR "Compiler exception audit failed with " . scalar(@error) .
		" error(s):\n";
	print STDERR "  $_\n" for @error;
	exit 1;
}

exit 0 if $quiet;

print "Compiler exception audit passed at E8 architecture limits: " .
	"$count{logic_throw} logic throws, $count{runtime_throw} runtime throws, " .
	"$count{generic_throw_files} files, " .
	"$count{generic_return_helper} generic return helpers, " .
	"$count{catch_all} catch-all sites, " .
	"$count{internal_standard_catch} internal standard catches, " .
	"$count{foreign_catch} catches of non-error types, " .
	"$count{rethrow} rethrows, and " .
	"$count{message_policy} message-policy sites; " .
	"$count{legacy_not_implemented} legacy not-implemented references and " .
	"$count{terminal_untyped_standard_catch} untyped terminal fallbacks.\n";
print "Reviewed inventory: " . scalar(@terminal_catch) .
	" terminal standard catches, $typed_catch_sites typed catches and " .
	"$explicit_throw_sites explicit typed throw sites.\n";
