#!/usr/bin/perl
# Evaluate an expectation sidecar against a LowIR or machine-IR text file.
#
#   expect_ir.pl <ir-file> <expect-file>
#
# Exit status 0 when every predicate holds; 1 with one "FAIL: ..." line per
# failing predicate on standard output otherwise; 2 on a malformed sidecar.
#
# The sidecar is one predicate per line, `#` to end of line is a comment:
#
#   count(<pattern>) <op> <number> [in <scope>]
#   has(<pattern>) [in <scope>]            count >= 1
#   none(<pattern>) [in <scope>]           count == 0
#   absent(@function)                      no definition of that function
#   <metric> <op> <number> [in <scope>]    shorthand for count(<metric>)
#
# <op> is one of == != <= < >= >.  <scope> is `@function`, `^block`, or
# `@function ^block`; without a scope the whole file is counted.  A function
# the file does not define has no instructions: every count in it is 0, so
# an upper bound on a function the optimizer removed holds, while `has`, a
# lower bound or a nonzero equality fails.
#
# <pattern> is one of:
#   instructions        every instruction line in scope
#   blocks              block headers in scope
#   functions           function definitions in scope
#   call                every call
#   call @name          calls whose callee is @name
#   <kind>              instructions of that kind: LowIR `load`, `store`,
#                       `phi`, `branch`, `cmp`, `binary`, ... or a machine
#                       mnemonic such as `mov`, `push`, `lea`, `jmp`
#   <kind> <word>       instructions of that kind whose next token is <word>,
#                       e.g. `binary add`, `cmp ult`, `mov.i64`
#   mem                 memory operands `[...]` across the instructions
#   frame               memory operands based on rbp or rsp
#   stack_size          the machine frame's stack_size (per function; the
#                       largest in scope)
#   preserve            the number of callee-saved registers a machine
#                       function's `preserve` line names, in its abi or
#                       frame section (per function; the largest in scope)
#   /regex/             instruction lines matching a Perl regex
use strict;
use warnings;

my ($ir_path, $expect_path) = @ARGV;
die "usage: expect_ir.pl <ir-file> <expect-file>\n"
	if !defined($ir_path) || !defined($expect_path);

sub read_lines
{
	my ($path) = @_;
	open(my $fh, '<', $path) or die "Unable to read $path: $!\n";
	my @lines = <$fh>;
	close($fh);
	chomp(@lines);
	return @lines;
}

# ---- parse the IR into (function, block, kind, tokens, text) records ----

my @records;
my %functions;
my %stack_size;
my %preserve;
{
	my $function = '';
	my $block = '';
	my $section = '';
	for my $line (read_lines($ir_path))
	{
		if ($line =~ /^function\s+\@([^\s(]+)/)
		{
			$function = $1;
			$block = '';
			$section = '';
			$functions{$function} = 1;
			next;
		}
		if ($line =~ /^(?:declare|global|machine_ir|startup)\b/ || $line =~ /^\}/)
		{
			$function = '' if $line =~ /^(?:declare|global|machine_ir|startup|\})/;
			$block = '';
			$section = '';
			next;
		}
		next if $function eq '';
		if ($line =~ /^\s+block\s+\^([^\s:]+)/)
		{
			$block = $1;
			$section = 'block';
			next;
		}
		if ($line =~ /^\s{2}(abi|frame)\s*$/)
		{
			$section = $1;
			$block = '';
			next;
		}
		if ($section eq 'frame' && $line =~ /^\s+stack_size\s+(\d+)/)
		{
			$stack_size{$function} = $1;
			next;
		}
		if (($section eq 'abi' || $section eq 'frame') && $line =~ /^\s+preserve\b(.*)$/)
		{
			my @registers = split(/\s+/, $1);
			$preserve{$function} = scalar(grep { $_ ne '' } @registers);
			next;
		}
		next if $section ne 'block';
		next if $line !~ /^\s{4}\S/;
		my $text = $line;
		$text =~ s/^\s+//;
		my $body = $text;
		$body =~ s/^%[^\s=]+\s*=\s*//;
		my @tokens = split(/[\s,()]+/, $body);
		my $kind = defined($tokens[0]) ? $tokens[0] : '';
		my $mnemonic = $kind;
		$mnemonic =~ s/\..*$//;
		push @records, {
			function => $function,
			block => $block,
			kind => $kind,
			mnemonic => $mnemonic,
			tokens => \@tokens,
			text => $text,
		};
	}
}

# ---- evaluate ----

sub in_scope
{
	my ($record, $scope) = @_;
	return 0 if defined($scope->{function}) && $record->{function} ne $scope->{function};
	return 0 if defined($scope->{block}) && $record->{block} ne $scope->{block};
	return 1;
}

sub callee_of
{
	my ($record) = @_;
	for my $token (@{$record->{tokens}})
	{
		return $1 if $token =~ /^\@(.+)$/;
	}
	return undef;
}

sub count_pattern
{
	my ($pattern, $scope) = @_;
	if ($pattern eq 'blocks')
	{
		my %seen;
		for my $r (@records)
		{
			next if !in_scope($r, $scope);
			$seen{"$r->{function}\0$r->{block}"} = 1;
		}
		return scalar(keys %seen);
	}
	if ($pattern eq 'functions')
	{
		return scalar(grep { !defined($scope->{function}) || $_ eq $scope->{function} } keys %functions);
	}
	if ($pattern eq 'preserve')
	{
		my $largest = 0;
		for my $function (keys %preserve)
		{
			next if defined($scope->{function}) && $function ne $scope->{function};
			$largest = $preserve{$function} if $preserve{$function} > $largest;
		}
		return $largest;
	}
	if ($pattern eq 'stack_size')
	{
		my $largest = 0;
		for my $function (keys %stack_size)
		{
			next if defined($scope->{function}) && $function ne $scope->{function};
			$largest = $stack_size{$function} if $stack_size{$function} > $largest;
		}
		return $largest;
	}
	my @scoped = grep { in_scope($_, $scope) } @records;
	return scalar(@scoped) if $pattern eq 'instructions';
	if ($pattern eq 'mem')
	{
		my $n = 0;
		$n += () = $_->{text} =~ /\[[^\]]*\]/g for @scoped;
		return $n;
	}
	if ($pattern eq 'frame')
	{
		my $n = 0;
		$n += () = $_->{text} =~ /\[(?:rbp|rsp)[^\]]*\]/g for @scoped;
		return $n;
	}
	if ($pattern =~ m{^/(.*)/$})
	{
		my $regex = qr/$1/;
		return scalar(grep { $_->{text} =~ $regex } @scoped);
	}
	if ($pattern =~ /^call\s+\@(\S+)$/)
	{
		my $name = $1;
		return scalar(grep {
			$_->{mnemonic} eq 'call' && defined(callee_of($_)) && callee_of($_) eq $name
		} @scoped);
	}
	if ($pattern =~ /^(\S+)\s+(\S+)$/)
	{
		my ($kind, $word) = ($1, $2);
		return scalar(grep {
			($_->{kind} eq $kind || $_->{mnemonic} eq $kind) &&
			defined($_->{tokens}[1]) && $_->{tokens}[1] eq $word
		} @scoped);
	}
	return scalar(grep { $_->{kind} eq $pattern || $_->{mnemonic} eq $pattern } @scoped);
}

sub parse_scope
{
	my ($text) = @_;
	my %scope;
	return \%scope if !defined($text) || $text eq '';
	for my $part (split(/\s+/, $text))
	{
		if ($part =~ /^\@(.+)$/) { $scope{function} = $1; }
		elsif ($part =~ /^\^(.+)$/) { $scope{block} = $1; }
		else { return undef; }
	}
	return \%scope;
}

sub compare_values
{
	my ($actual, $op, $expected) = @_;
	return $actual == $expected if $op eq '==';
	return $actual != $expected if $op eq '!=';
	return $actual <= $expected if $op eq '<=';
	return $actual <  $expected if $op eq '<';
	return $actual >= $expected if $op eq '>=';
	return $actual >  $expected if $op eq '>';
	return undef;
}

my $failures = 0;
my $predicates = 0;
my $line_number = 0;
for my $raw (read_lines($expect_path))
{
	++$line_number;
	my $line = $raw;
	$line =~ s/#.*$//;
	$line =~ s/^\s+|\s+$//g;
	next if $line eq '';
	my ($pattern, $op, $expected, $scope_text);
	if ($line =~ /^count\((.+?)\)\s*(==|!=|<=|<|>=|>)\s*(\d+)(?:\s+in\s+(.+))?$/)
	{
		($pattern, $op, $expected, $scope_text) = ($1, $2, $3, $4);
	}
	elsif ($line =~ /^has\((.+?)\)(?:\s+in\s+(.+))?$/)
	{
		($pattern, $op, $expected, $scope_text) = ($1, '>=', 1, $2);
	}
	elsif ($line =~ /^none\((.+?)\)(?:\s+in\s+(.+))?$/)
	{
		($pattern, $op, $expected, $scope_text) = ($1, '==', 0, $2);
	}
	elsif ($line =~ /^absent\(\@(\S+)\)$/)
	{
		++$predicates;
		if (exists($functions{$1}))
		{
			print "FAIL: $line: function \@$1 is defined\n";
			++$failures;
		}
		next;
	}
	elsif ($line =~ /^(instructions|blocks|functions|mem|frame|stack_size|preserve)\s*(==|!=|<=|<|>=|>)\s*(\d+)(?:\s+in\s+(.+))?$/)
	{
		($pattern, $op, $expected, $scope_text) = ($1, $2, $3, $4);
	}
	else
	{
		print "ERROR: $expect_path:$line_number: cannot parse expectation: $raw\n";
		exit(2);
	}
	my $scope = parse_scope($scope_text);
	if (!defined($scope))
	{
		print "ERROR: $expect_path:$line_number: bad scope in: $raw\n";
		exit(2);
	}
	# A function the output does not define has no instructions: every
	# count in it is 0 (see the header).
	my $actual = defined($scope->{function}) && !exists($functions{$scope->{function}})
		? 0 : count_pattern($pattern, $scope);
	++$predicates;
	if (!compare_values($actual, $op, $expected))
	{
		print "FAIL: $line: actual $actual\n";
		++$failures;
	}
}
print "ERROR: $expect_path holds no predicate\n" if $predicates == 0;
exit($failures || $predicates == 0 ? 1 : 0);
