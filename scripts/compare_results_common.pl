#!/usr/bin/perl

use strict;
use warnings;
use Cwd qw(getcwd);
use File::Find;
use File::Basename qw(basename dirname);
use Scalar::Util qw(looks_like_number);
use Text::ParseWords qw(shellwords);
use FindBin;
use lib $FindBin::Bin;
use CppgmBatchWorker ();
use POSIX qw(WNOHANG);
use Time::HiRes ();

sub collect_tests
{
	my @tests;
	my ($spec, $pattern) = @_;
	my @roots = shellwords($spec);
	die "Test path '$spec' does not exist\n" if scalar(@roots) == 0;

	my %seen;
	for my $term (@roots)
	{
		my @matches = $term =~ /[*?\[]/ ? glob($term) : ($term);
		die "Test path '$term' does not exist\n" if scalar(@matches) == 0;
		for my $root (sort @matches)
		{
			die "Test path '$root' does not exist\n" if !-e $root;
			my @found;
			if (-f $root)
			{
				@found = $root =~ $pattern ? ($root) : ();
			}
			elsif (-d $root)
			{
				find(sub {
					return if !-f $_;
					push @found, $File::Find::name if $File::Find::name =~ $pattern;
				}, $root);
				@found = sort @found;
			}
			for my $test (@found)
			{
				next if $seen{$test}++;
				push @tests, $test;
			}
		}
	}
	return @tests;
}

sub getdata
{
	my ($file) = @_;
	return undef if !-e $file;
	open(my $fh, '<', $file) or return undef;
	local $/;
	my $data = <$fh>;
	close($fh);
	$data = '' if !defined($data);
	$data =~ s/\s+$//;
	return $data;
}

sub getrawdata
{
	my ($file) = @_;
	return undef if !-e $file;
	open(my $fh, '<', $file) or return undef;
	local $/;
	my $data = <$fh>;
	close($fh);
	return '' if !defined($data);
	return $data;
}

sub env_flag_enabled
{
	my ($name) = @_;
	return 0 if !defined($ENV{$name});
	my $value = $ENV{$name};
	return 0 if $value eq '';
	return 0 if $value eq '0';
	return 0 if $value =~ /^(?:false|no|off)$/i;
	return 1;
}

sub normalize_machine_ir
{
	my ($data) = @_;
	return undef if !defined($data);
	$data =~ s/^machine_ir x86_64 \S+$/machine_ir x86_64 <host-target>/m;
	return $data;
}

sub putrawdata
{
	my ($file, $data) = @_;
	open(my $fh, '>', $file) or die "Unable to write $file\n";
	print $fh $data;
	close($fh);
}

sub canonical_machine_ir_memory_operand
{
	my ($base, $sign, $disp, $state) = @_;
	return "[$base]" if !defined($sign) || !defined($disp);
	my $key = "$base|$sign|$disp";
	my $bucket = "$base|$sign";
	if (!exists($state->{disp_map}{$key}))
	{
		my $next = $state->{next_disp}{$bucket} || 0;
		$state->{disp_map}{$key} = $next;
		$state->{next_disp}{$bucket} = $next + 1;
	}
	my $index = $state->{disp_map}{$key};
	return "[$base$sign<off$index>]";
}

sub canonical_machine_ir_indexed_memory_operand
{
	my ($base, $index, $scale, $sign, $disp, $state) = @_;
	my $address = "$base+$index";
	$address .= "*$scale" if defined($scale);
	return "[$address]" if !defined($sign) || !defined($disp);
	my $key = "$address|$sign|$disp";
	my $bucket = "$address|$sign";
	if (!exists($state->{disp_map}{$key}))
	{
		my $next = $state->{next_disp}{$bucket} || 0;
		$state->{disp_map}{$key} = $next;
		$state->{next_disp}{$bucket} = $next + 1;
	}
	my $offset = $state->{disp_map}{$key};
	return "[$address$sign<off$offset>]";
}

sub canonical_machine_ir_free_gpr
{
	my ($reg, $state) = @_;
	if (!exists($state->{gpr_map}{$reg}))
	{
		my $next = $state->{next_gpr};
		$state->{gpr_map}{$reg} = "<gpr$next>";
		$state->{next_gpr} = $next + 1;
	}
	return $state->{gpr_map}{$reg};
}

sub canonical_machine_ir_free_xmm
{
	my ($reg, $state) = @_;
	if (!exists($state->{xmm_map}{$reg}))
	{
		my $next = $state->{next_xmm};
		$state->{xmm_map}{$reg} = "<xmm$next>";
		$state->{next_xmm} = $next + 1;
	}
	return $state->{xmm_map}{$reg};
}

sub canonicalize_machine_ir
{
	my ($data) = @_;
	return undef if !defined($data);
	$data = normalize_machine_ir($data);
	# Focused PA33 properties validate non-default entry alignment.  Keep the
	# legacy structural MIR oracle concerned with instruction and frame shape.
	$data =~ s/^\s+code_alignment\s+\d+\n//mg;
	# The frame's total size is a layout choice like its displacements: a
	# structural oracle keeps the bindings and their order, not the number.
	$data =~ s/^(\s+stack_size)\s+\d+/$1 N/mg;
	my %state = (
		disp_map => {},
		next_disp => {},
		gpr_map => {},
		next_gpr => 0,
		xmm_map => {},
		next_xmm => 0,
	);
	$data =~ s/\[([A-Za-z_][A-Za-z0-9_]*)\+([A-Za-z_][A-Za-z0-9_]*)(?:\*([1248]))?([+-])(\d+)\]/canonical_machine_ir_indexed_memory_operand($1, $2, $3, $4, $5, \%state)/ge;
	$data =~ s/\[([A-Za-z_][A-Za-z0-9_]*)([+-])(\d+)\]/canonical_machine_ir_memory_operand($1, $2, $3, \%state)/ge;
	$data =~ s/\b(rbx|r10|r11|r12|r13|r14|r15)\b/canonical_machine_ir_free_gpr($1, \%state)/ge;
	$data =~ s/\b(xmm2|xmm3|xmm4|xmm5|xmm6|xmm7)\b/canonical_machine_ir_free_xmm($1, \%state)/ge;
	return $data;
}

sub compare_strict_machine_ir
{
	my ($ref, $my) = @_;
	my $ref_mir = normalize_machine_ir(getdata("$ref.mir"));
	return (1, undef, undef) if !defined($ref_mir);
	my $my_mir = normalize_machine_ir(getdata("$my.mir"));
	return (0, "ERROR: missing generated machine IR dump (.mir)", undef) if !defined($my_mir);
	return (1, undef, undef) if $ref_mir eq $my_mir;
	my $hint = "To compare strict machine IR dumps:\n\n    \$ diff " .
		rooted_path("$ref.mir") . " " . rooted_path("$my.mir") . "\n\n";
	return (0, "ERROR: strict machine IR dumps do not match (.mir)", $hint);
}

sub clear_structural_machine_ir_failure_artifacts
{
	my ($ref, $my) = @_;
	unlink("$my.cmir");
}

sub compare_structural_machine_ir
{
	my ($ref, $my) = @_;
	clear_structural_machine_ir_failure_artifacts($ref, $my);
	my $ref_raw = getrawdata("$ref.cmir");
	return (0, "ERROR: missing structural machine IR oracle (.ref.cmir)", undef) if !defined($ref_raw);
	my $my_raw = getrawdata("$my.mir");
	return (0, "ERROR: missing generated machine IR dump (.mir)", undef) if !defined($my_raw);

	my $ref_mir = canonicalize_machine_ir($ref_raw);
	my $my_mir = canonicalize_machine_ir($my_raw);
	return (1, undef, undef) if $ref_mir eq $my_mir;

	putrawdata("$my.cmir", $my_mir);
	my $hint = "To compare structural machine IR oracles:\n\n    \$ diff " .
		rooted_path("$ref.cmir") . " " . rooted_path("$my.cmir") . "\n\n";
	return (0, "ERROR: structural machine IR dumps do not match after canonicalization", $hint);
}

sub parse_link_map
{
	my ($file) = @_;
	open(my $fh, '<', $file) or die "Unable to open $file\n";
	my @lines;
	while (my $line = <$fh>)
	{
		chomp($line);
		next if $line =~ /^\s*$/;
		push @lines, $line;
	}
	close($fh);

	die "Invalid link map $file\n" if scalar(@lines) < 4;

	my %map = (
		objects => [],
		symbols => [],
	);

	if ($lines[0] !~ /^link_map x86_64 (\S+)$/)
	{
		die "Invalid link_map header in $file\n";
	}
	$map{target} = $1;

	if ($lines[1] !~ /^startup_size (\d+)$/)
	{
		die "Invalid startup_size in $file\n";
	}
	$map{startup_size} = 0 + $1;

	if ($lines[2] !~ /^code_size (\d+)$/)
	{
		die "Invalid code_size in $file\n";
	}
	$map{code_size} = 0 + $1;

	if ($lines[3] !~ /^data_size (\d+)$/)
	{
		die "Invalid data_size in $file\n";
	}
	$map{data_size} = 0 + $1;

	for (my $i = 4; $i < scalar(@lines); ++$i)
	{
		my $line = $lines[$i];
		if ($line =~ /^object (\d+) code_offset (\d+) data_offset (\d+)$/)
		{
			push @{$map{objects}}, {
				index => 0 + $1,
				code_offset => 0 + $2,
				data_offset => 0 + $3,
			};
		}
		elsif ($line =~ /^symbol (\S+) (code|data) (\d+)$/)
		{
			push @{$map{symbols}}, {
				name => $1,
				section => $2,
				offset => 0 + $3,
			};
		}
		else
		{
			die "Invalid link-map line in $file: $line\n";
		}
	}

	return \%map;
}

sub validate_link_map
{
	my ($map, $file) = @_;

	die "Unsupported target in $file\n"
		if $map->{target} ne 'macos' &&
		   $map->{target} ne 'linux';

	die "Invalid sizes in $file\n"
		if !looks_like_number($map->{startup_size}) ||
		   !looks_like_number($map->{code_size}) ||
		   !looks_like_number($map->{data_size});

	die "Inconsistent size ordering in $file\n"
		if $map->{startup_size} > $map->{code_size} ||
		   $map->{code_size} > $map->{data_size};

	for (my $i = 0; $i < scalar(@{$map->{objects}}); ++$i)
	{
		my $object = $map->{objects}[$i];
		die "Object index ordering mismatch in $file\n"
			if $object->{index} != $i;
		die "Object code offset out of range in $file\n"
			if $object->{code_offset} < $map->{startup_size} ||
			   $object->{code_offset} > $map->{code_size};
		die "Object data offset out of range in $file\n"
			if $object->{data_offset} < $map->{code_size} ||
			   $object->{data_offset} > $map->{data_size};
	}

	for (my $i = 0; $i < scalar(@{$map->{symbols}}); ++$i)
	{
		my $symbol = $map->{symbols}[$i];
		if ($symbol->{section} eq 'code')
		{
			die "Code symbol offset out of range in $file\n"
				if $symbol->{offset} < $map->{startup_size} ||
				   $symbol->{offset} >= $map->{code_size};
		}
		elsif ($symbol->{section} eq 'data')
		{
			die "Data symbol offset out of range in $file\n"
				if $symbol->{offset} < $map->{code_size} ||
				   $symbol->{offset} >= $map->{data_size};
		}
		else
		{
			die "Invalid symbol section in $file\n";
		}
		if ($i > 0 && $map->{symbols}[$i - 1]{name} gt $symbol->{name})
		{
			die "Symbol ordering mismatch in $file\n";
		}
	}
}

sub compare_link_maps_structural
{
	my ($ref_file, $my_file) = @_;
	my $ref = parse_link_map($ref_file);
	my $my = parse_link_map($my_file);

	validate_link_map($ref, $ref_file);
	validate_link_map($my, $my_file);

	return 0 if scalar(@{$ref->{objects}}) != scalar(@{$my->{objects}});
	return 0 if scalar(@{$ref->{symbols}}) != scalar(@{$my->{symbols}});

	for (my $i = 0; $i < scalar(@{$ref->{objects}}); ++$i)
	{
		return 0 if $ref->{objects}[$i]{index} != $my->{objects}[$i]{index};
	}

	for (my $i = 0; $i < scalar(@{$ref->{symbols}}); ++$i)
	{
		return 0 if $ref->{symbols}[$i]{name} ne $my->{symbols}[$i]{name};
		return 0 if $ref->{symbols}[$i]{section} ne $my->{symbols}[$i]{section};
	}

	return 1;
}

sub append_keep_going_summary
{
	my ($repo_root, $cwd, $npass, $suite_total, $failed) = @_;
	# A tool that runs the harness many times (scripts/check_lowir_seams.py)
	# names its own counts file so concurrent runs do not mix their lines.
	my $counts_path = defined($ENV{CPPGM_TEST_COUNTS_FILE}) && $ENV{CPPGM_TEST_COUNTS_FILE} ne ''
		? $ENV{CPPGM_TEST_COUNTS_FILE} : "$repo_root/.test_counts";
	if (open(my $fh, '>>', $counts_path))
	{
		print $fh "$npass $suite_total\n";
		close($fh);
	}
	if ($failed)
	{
		system('touch', "$cwd/.test_failed");
	}
}

# An expectation sidecar (`x.ref.expect`, see scripts/expect_ir.pl) states
# the outcome a fixture tests and the size envelope its output must stay in.
# When one is present it replaces the comparison of the output's shape with
# the reference; the reference may stay beside it as an informational example.
sub expectation_sidecar
{
	my ($testbase) = @_;
	my $path = "$testbase.ref.expect";
	return -f $path ? $path : undef;
}

sub evaluate_expectations
{
	my ($expect_path, $ir_path) = @_;
	return (0, "ERROR: missing output to evaluate expectations against ($ir_path)", undef)
		if !-f $ir_path;
	my $evaluator = dirname(__FILE__) . "/expect_ir.pl";
	my $output = `perl "$evaluator" "$ir_path" "$expect_path" 2>&1`;
	my $status = $? >> 8;
	return (1, undef, undef) if $status == 0;
	my $hint = "To evaluate the expectations by hand:\n\n    \$ perl " .
		rooted_path($evaluator) . " " . rooted_path($ir_path) . " " .
		rooted_path($expect_path) . "\n\n";
	chomp($output);
	$output =~ s/\n/; /g;
	return (0, "ERROR: expectations not met ($output)", $hint);
}

# With CPPGM_LOWER_CHECK_APP naming a `cppgm++`, every successful LowIR
# output must also lower through the object path at -O0: the optimizer's
# output is not correct until the backend accepts it.
sub lowir_lowers
{
	my ($app, $path, $input, $log) = @_;
	my $object = "$input.o";
	my $data = getdata($path);
	return undef if !defined($data);
	putrawdata($input, $data);
	my $status = system("\"$app\" -c -O0 -o \"$object\" \"$input\" > /dev/null 2> \"$log\"");
	unlink($object);
	return $status == 0 ? 1 : 0;
}

sub check_lowir_lowers
{
	my ($my, $source_file) = @_;
	my $app = $ENV{CPPGM_LOWER_CHECK_APP};
	return (1, undef, undef) if !defined($app) || $app eq '';
	# The driver recognizes LowIR input by its extension.  A fixture whose
	# input the backend does not accept (a handwritten shape outside the
	# object path's contract) has no floor to preserve.
	my $input = "$my.lower.lowir";
	my $log = "$my.lower.stderr";
	if (defined($source_file) && -f $source_file && source_is_lowir($source_file))
	{
		my $source_input = "$my.lower.input.lowir";
		my $source_log = "$my.lower.input.stderr";
		my $source_lowers = lowir_lowers($app, $source_file, $source_input, $source_log);
		unlink($source_input, $source_log);
		return (1, undef, undef) if !$source_lowers;
	}
	my $lowers = lowir_lowers($app, $my, $input, $log);
	return (0, "ERROR: missing output file ($my)", undef) if !defined($lowers);
	if ($lowers)
	{
		unlink($input, $log);
		return (1, undef, undef);
	}
	my $reason = getdata($log);
	$reason = '' if !defined($reason);
	$reason =~ s/\s+/ /g;
	return (0, "ERROR: optimized LowIR does not lower at -O0 ($reason)",
		"To reproduce:\n\n    \$ $app -c -O0 -o /dev/null " . rooted_path($input) . "\n\n");
}

# With CPPGM_LOWIR_BEHAVIOR_APP naming a `lowir2native` (PA32's lanes name
# the reference backend), a handwritten LowIR fixture with an entry function
# is also run before and after optimization: the input and the optimized
# output are each built through that backend at -O0 and executed, and the
# optimized program must exit the way the input's does and print what it
# prints.  An input the backend does not build, or whose program does not
# finish, has no behaviour to preserve and is skipped; an output that does
# not build, or whose program ends or prints differently, fails.  The size
# budgets of a sidecar say nothing about results, so without this an
# optimizer that changed a program's answer passed.
# ----------------------------------------------------------------------
# The LowIR behaviour check (CPPGM_LOWIR_BEHAVIOR_APP, PA32): the fixture
# and the optimized output are each built into a program through the
# reference backend and run, and the two programs must end the same way
# and print the same output.
#
# What runs.  The fixture's entry (`@main`, else a `role=entry` function)
# runs as written when it takes no parameters.  Otherwise a generated
# driver is the entry: it calls a parameterized entry with fixed scalar
# arguments, or, in a fixture without an entry, every function of
# external binding whose parameters and result are scalars, in definition
# order, and returns the sum of their results.  A helper the fixture
# declares and nothing defines gets a provider that returns zero; when its
# declaration allows unwinding, both programs also run with a provider
# that throws, so the handlers run too.  An `x.driver.lowir` beside the
# fixture replaces the generated driver and providers: it is appended as
# written, its `@main` is the entry and the fixture's own `@main` is
# renamed `@__behavior_entry`.  The lane's summary says how many inputs
# ran, how, and why the others did not.
our %lowir_behavior_tally;

sub lowir_scalar_type
{
	my ($type) = @_;
	return defined($type) && $type =~ /^[iu](?:1|8|16|32|64)$/ ? 1 : 0;
}

# The function headers of a LowIR text: name, parameters [[name, type, text]...],
# result type, metadata (key => value), the raw bracket text and whether
# the header is a declaration.
sub lowir_function_headers
{
	my ($data) = @_;
	my @headers;
	return @headers if !defined($data);
	while ($data =~ /^(declare[ \t]+)?function[ \t]+\@([^\s(]+)\(([^)]*)\)[ \t]*->[ \t]*(\S+)(\s*\[[^\]\n]*\])?/mg)
	{
		my ($declared, $name, $params, $result, $bracket) = ($1, $2, $3, $4, $5);
		# A parameter: its name, its type, and its text with any metadata.
		my @params;
		while ($params =~ /(%\S+)\s*:\s*([^\s,\[]+)(\s*\[[^\]]*\])?/g)
		{
			push @params, [$1, $2, "$1 : $2" . (defined($3) ? $3 : '')];
		}
		my %meta;
		if (defined($bracket) && $bracket =~ /\[([^\]]*)\]/)
		{
			for my $item (split(/\s*,\s*/, $1))
			{
				my ($key, $value) = split(/=/, $item, 2);
				$meta{$key} = defined($value) ? $value : 1;
			}
		}
		push @headers, { declared => defined($declared) ? 1 : 0, name => $name, params => \@params,
			result => $result, meta => \%meta, bracket => defined($bracket) ? $bracket : '' };
	}
	return @headers;
}

sub lowir_all_scalar_params
{
	my ($header) = @_;
	for my $param (@{ $header->{params} })
	{
		return 0 if !lowir_scalar_type($param->[1]);
	}
	return 1;
}

# A driver entry that calls each of @functions with fixed arguments and
# returns the sum of their integer results.
sub lowir_behavior_driver
{
	my (@functions) = @_;
	my @values = (3, 5, 7, 11, 13, 17, 19, 23);
	my @body = ('    %sum0 = const i64 0');
	my $sum = '%sum0';
	my $n = 0;
	for my $function (@functions)
	{
		my @arguments;
		for my $i (0 .. $#{ $function->{params} })
		{
			my $type = $function->{params}[$i][1];
			push @body, "    %arg${n}_$i = const $type " . ($type eq 'i1' ? 1 : $values[$i % @values]);
			push @arguments, "%arg${n}_$i";
		}
		my $call = "call $function->{result} \@$function->{name}(" . join(', ', @arguments) . ")";
		if ($function->{result} eq 'void')
		{
			push @body, "    $call";
		}
		else
		{
			push @body, "    %result$n = $call";
			my $wide = "%result$n";
			if ($function->{result} ne 'i64')
			{
				my $extend = $function->{result} =~ /^i(?:8|16|32)$/ ? 'sext' : 'zext';
				push @body, "    %wide$n = convert $extend i64 $function->{result} %result$n";
				$wide = "%wide$n";
			}
			push @body, "    %sum" . ($n + 1) . " = binary add i64 $sum, $wide";
			$sum = '%sum' . ($n + 1);
		}
		++$n;
	}
	push @body, "    return i64 $sum";
	return "function \@main() -> i64 [role=entry] {\n  block ^entry:\n" . join("\n", @body) . "\n}\n";
}

sub lowir_provider_may_unwind
{
	my ($header) = @_;
	return ($header->{meta}{unwind} || '') ne 'no' && ($header->{meta}{return} || '') ne 'noreturn';
}

# A provider for a declared helper nothing defines: it returns zero, never
# returns when declared noreturn, or throws when $throwing and it may
# unwind.  Undefined when no generated provider can define the helper.
sub lowir_behavior_provider
{
	my ($header, $throwing) = @_;
	return undef if exists($header->{meta}{arity});
	my $body;
	if (($header->{meta}{return} || '') eq 'noreturn')
	{
		$body = '    unreachable';
	}
	elsif ($throwing && lowir_provider_may_unwind($header))
	{
		$body = "    %thrown = const i64 7\n    throw i64 %thrown";
	}
	elsif ($header->{result} eq 'void')
	{
		$body = '    return void';
	}
	elsif (lowir_scalar_type($header->{result}))
	{
		$body = "    return $header->{result} 0";
	}
	else
	{
		return undef;
	}
	my $params = join(', ', map { $_->[2] } @{ $header->{params} });
	return "function \@$header->{name}($params) -> $header->{result}$header->{bracket} {\n  block ^entry:\n$body\n}\n";
}

# How a fixture's programs are assembled (rename its `@main`, the function
# whose entry role to drop, the driver text, the names the driver defines,
# how to describe it), or `skip` with why it cannot run.
sub lowir_behavior_plan
{
	my ($data, $sidecar) = @_;
	my @defined = grep { !$_->{declared} } lowir_function_headers($data);
	my %plan = (rename => 0, strip_role => undef, driver => '', provided => [], how => 'as written');
	my ($main) = grep { $_->{name} eq 'main' } @defined;
	my ($entry) = $main ? ($main) : grep { ($_->{meta}{role} || '') eq 'entry' } @defined;
	if (defined($sidecar))
	{
		my @provided = map { $_->{name} } grep { !$_->{declared} } lowir_function_headers($sidecar);
		if (grep { $_ eq 'main' } @provided)
		{
			$plan{rename} = 1 if $main;
			$plan{strip_role} = $main ? '__behavior_entry' : $entry->{name} if $entry;
		}
		@plan{qw(driver provided how)} = ($sidecar, [grep { $_ ne 'main' } @provided], 'with its driver sidecar');
		return \%plan;
	}
	if ($entry)
	{
		return \%plan if !@{ $entry->{params} };
		return { skip => 'an entry parameter that is not a scalar' } if !lowir_all_scalar_params($entry);
		return { skip => 'an entry result that is not a scalar' }
			if $entry->{result} ne 'void' && !lowir_scalar_type($entry->{result});
		my %target = %$entry;
		if ($entry->{name} eq 'main')
		{
			$plan{rename} = 1;
			$target{name} = '__behavior_entry';
		}
		$plan{strip_role} = $target{name};
		$plan{driver} = lowir_behavior_driver(\%target);
		$plan{how} = 'through a generated driver';
		return \%plan;
	}
	my @callable = grep {
		($_->{meta}{binding} || 'strong') ne 'internal' && !exists($_->{meta}{role}) &&
			($_->{meta}{return} || '') ne 'noreturn' && !exists($_->{meta}{arity}) &&
			($_->{result} eq 'void' || lowir_scalar_type($_->{result})) && lowir_all_scalar_params($_)
	} @defined;
	return { skip => 'no function a driver can call' } if !@callable;
	$plan{driver} = lowir_behavior_driver(@callable);
	$plan{how} = 'through a generated driver';
	return \%plan;
}

# The text of one program: the fixture (or its output) with its `@main`
# renamed and the entry role dropped as the plan says, the declarations
# of the provided names removed, then the driver and the providers.
sub lowir_behavior_program_text
{
	my ($data, $plan, $providers) = @_;
	my $text = $data;
	$text =~ s/(?<![\w\@.])\@main\b/\@__behavior_entry/g if $plan->{rename};
	if (defined($plan->{strip_role}))
	{
		my $name = $plan->{strip_role};
		$text =~ s{^(function[ \t]+\@\Q$name\E\([^)]*\)[ \t]*->[ \t]*\S+)\s*\[([^\]\n]*)\]}{
			my ($head, $items) = ($1, $2);
			my @kept = grep { !/^role=entry$/ } split(/\s*,\s*/, $items);
			$head . (@kept ? ' [' . join(', ', @kept) . ']' : '')
		}me;
	}
	for my $name (@{ $plan->{provided} }, keys %$providers)
	{
		$text =~ s/^declare[ \t]+function[ \t]+\@\Q$name\E\([^\n]*\n(?:[ \t]*\[[^\]\n]*\][ \t]*\n)?//mg;
	}
	my $tail = $plan->{driver};
	$tail .= $providers->{$_} for sort keys %$providers;
	return "$text\n$tail";
}

sub process_ending
{
	my ($status) = @_;
	return 'could not run' if $status == -1;
	return 'was killed by signal ' . ($status & 127) if $status & 127;
	return 'exited with status ' . ($status >> 8);
}

# Runs a program with no input and its output to a file, under the
# program time bound.  A program that exits with status 124 is not taken
# for a timeout, as GNU timeout's own status would have it.
sub run_lowir_program
{
	my ($program, $stdout_path) = @_;
	my $timeout = CppgmBatchWorker::get_timeout_from_env('CPPGM_PROGRAM_TEST_TIMEOUT_SEC', 10);
	my $pid = fork();
	return { status => -1, timed_out => 0 } if !defined($pid);
	if ($pid == 0)
	{
		setpgrp(0, 0);
		open(STDIN, '<', '/dev/null');
		open(STDOUT, '>', $stdout_path);
		open(STDERR, '>', '/dev/null');
		exec { $program } $program;
		exit(127);
	}
	my $deadline = Time::HiRes::time() + $timeout;
	while (1)
	{
		my $reaped = waitpid($pid, WNOHANG);
		return { status => $?, timed_out => 0 } if $reaped == $pid;
		return { status => -1, timed_out => 0 } if $reaped == -1;
		if (Time::HiRes::time() >= $deadline)
		{
			kill('KILL', -$pid);
			kill('KILL', $pid);
			waitpid($pid, 0);
			return { timed_out => 1 };
		}
		Time::HiRes::sleep(0.01);
	}
}

sub build_lowir_program
{
	my ($app, $lowir, $program, $log) = @_;
	my $build_timeout = CppgmBatchWorker::get_timeout_from_env('CPPGM_BUILD_TEST_TIMEOUT_SEC', 30);
	return system("timeout $build_timeout \"$app\" -O0 -o \"$program\" \"$lowir\" > /dev/null 2> \"$log\"") == 0;
}

sub lowir_undefined_symbols
{
	my ($log) = @_;
	my $text = getdata($log);
	return () if !defined($text) || $text !~ /undefined symbols?: (.*)/;
	my $list = $1;
	return $list =~ /(\S+) \(referenced from/g;
}

sub lowir_behavior_skip
{
	my ($reason) = @_;
	++$lowir_behavior_tally{'not run'}{$reason};
	return (1, undef, undef);
}

# Assembles, builds and runs one program (the input's or the output's),
# generating providers for the helpers the link reports undefined when
# %$providers has none for them.  Returns {built, timed_out, status,
# ending, stdout, providers} or, when no generated provider can define a
# helper the input declares, {skip => reason}.
sub lowir_behavior_outcome
{
	my ($app, $data, $plan, $providers, $throwing, $base, $headers) = @_;
	my ($lowir, $program, $stdout_path, $log) = map { "$base.$_" } qw(lowir program stdout stderr);
	my %providers = %$providers;
	for my $attempt (1, 2)
	{
		putrawdata($lowir, lowir_behavior_program_text($data, $plan, \%providers));
		last if build_lowir_program($app, $lowir, $program, $log);
		return { built => 0, log => $log } if $attempt == 2;
		my @undefined = lowir_undefined_symbols($log);
		return { built => 0, log => $log } if !@undefined;
		for my $name (@undefined)
		{
			# The link names the symbol, which is the declaration's `object=`
			# when it has one.
			my ($header) = grep { $_->{declared} && ($_->{name} eq $name || ($_->{meta}{object} || '') eq $name) } @$headers;
			return { skip => "a declared \@$name no generated provider can define" }
				if !defined($header) || exists($providers{ $header->{name} });
			my $provider = lowir_behavior_provider($header, $throwing);
			return { skip => "a declared \@$header->{name} returning $header->{result}, which no generated provider can define" }
				if !defined($provider);
			$providers{ $header->{name} } = $provider;
		}
	}
	my $run = run_lowir_program($program, $stdout_path);
	return { built => 1, timed_out => 1, providers => \%providers } if $run->{timed_out};
	my $stdout = getdata($stdout_path);
	return { built => 1, timed_out => 0, status => $run->{status}, ending => process_ending($run->{status}),
		stdout => defined($stdout) ? $stdout : '', providers => \%providers };
}

sub check_lowir_behavior
{
	my ($my, $source_file) = @_;
	my $app = $ENV{CPPGM_LOWIR_BEHAVIOR_APP};
	return (1, undef, undef) if !defined($app) || $app eq '' || !defined($source_file);
	return (1, undef, undef) if !-f $source_file || !source_is_lowir($source_file);
	my $data = getdata($source_file);
	return (1, undef, undef) if !defined($data);
	(my $sidecar_path = $source_file) =~ s/\.t$/.driver.lowir/;
	my $sidecar = $sidecar_path ne $source_file && -f $sidecar_path ? getdata($sidecar_path) : undef;
	my $plan = lowir_behavior_plan($data, $sidecar);
	return lowir_behavior_skip($plan->{skip}) if defined($plan->{skip});
	my $output = getdata($my);
	return (0, "ERROR: missing output file ($my)", undef) if !defined($output);
	my @headers = lowir_function_headers($data);
	my $base = "$my.behavior";
	my @variants = ('input', 'output', 'input.throwing', 'output.throwing');
	my @artifacts = map { my $v = $_; map { "$base.$v.$_" } qw(lowir program stdout stderr) } @variants;
	unlink(@artifacts);
	my $hint = "To reproduce (the reference backend builds the assembled input and output at -O0; "
		. "the assembled programs stay beside the output):\n\n"
		. "    \$ $app -O0 -o input.program " . rooted_path("$base.input.lowir") . " && ./input.program; echo \$?\n"
		. "    \$ $app -O0 -o output.program " . rooted_path("$base.output.lowir") . " && ./output.program; echo \$?\n\n";
	my %providers;
	for my $throwing (0, 1)
	{
		my $suffix = $throwing ? '.throwing' : '';
		my $before = lowir_behavior_outcome($app, $data, $plan, {}, $throwing, "$base.input$suffix", \@headers);
		if (defined($before->{skip}))
		{
			unlink(@artifacts);
			return lowir_behavior_skip($before->{skip});
		}
		if (!$before->{built})
		{
			my $reason = getdata($before->{log});
			$reason = '' if !defined($reason);
			$reason =~ s/\s+/ /g;
			return (0, "ERROR: the input does not build through the reference backend ($reason)", $hint);
		}
		%providers = %{ $before->{providers} } if !$throwing;
		if ($before->{timed_out})
		{
			unlink(@artifacts);
			return lowir_behavior_skip('a program that does not finish');
		}
		# A trap, fault or arithmetic signal in the input program is undefined
		# behaviour the driver's arguments reached (an `unreachable`, a null
		# dereference), which the output need not reproduce; an abort is the
		# runtime's defined ending of an uncaught exception and is compared.
		if ($before->{status} != -1 && ($before->{status} & 127) && grep { ($before->{status} & 127) == $_ } (4, 7, 8, 11))
		{
			unlink(@artifacts);
			return lowir_behavior_skip('a program the driver\'s arguments crash');
		}
		my $with = $throwing ? ' (with a provider that throws)' : '';
		my $after = lowir_behavior_outcome($app, $output, $plan, $before->{providers}, $throwing,
			"$base.output$suffix", \@headers);
		return (0, "ERROR: missing output file ($my)", undef) if defined($after->{skip});
		if (!$after->{built})
		{
			my $reason = getdata($after->{log});
			$reason = '' if !defined($reason);
			$reason =~ s/\s+/ /g;
			return (0, "ERROR: optimized LowIR does not build through the reference backend ($reason)$with", $hint);
		}
		return (0, "ERROR: the optimized program did not finish; the input's program $before->{ending}$with", $hint)
			if $after->{timed_out};
		return (0, "ERROR: the optimized program $after->{ending}; the input's program $before->{ending}$with", $hint)
			if $before->{status} != $after->{status};
		return (0, "ERROR: the optimized program's output differs from the input program's$with "
			. "(see $base.input$suffix.stdout and $base.output$suffix.stdout)", $hint)
			if $before->{stdout} ne $after->{stdout};
		++$lowir_behavior_tally{'with a provider that throws'} if $throwing;
		last if !grep { lowir_provider_may_unwind($_) } grep { $_->{declared} && exists($providers{ $_->{name} }) } @headers;
	}
	++$lowir_behavior_tally{run};
	++$lowir_behavior_tally{how}{ $plan->{how} };
	++$lowir_behavior_tally{'with generated providers'} if %providers;
	unlink(@artifacts);
	return (1, undef, undef);
}

# One line for the lane: how many inputs the behaviour check ran, how,
# and why it did not run the others.
sub lowir_behavior_summary
{
	my %tally = %lowir_behavior_tally;
	return '' if !%tally;
	my $run = $tally{run} || 0;
	my @how = map { "$tally{how}{$_} $_" } sort keys %{ $tally{how} || {} };
	push @how, "$tally{$_} $_" for grep { $tally{$_} } ('with generated providers', 'with a provider that throws');
	my $line = "behaviour: ran $run input" . ($run == 1 ? '' : 's') . (@how ? ' (' . join('; ', @how) . ')' : '');
	my $not = $tally{'not run'} || {};
	$line .= '; not run: ' . join(', ', map { "$not->{$_} with $_" } sort keys %$not) if %$not;
	return "$line\n";
}

# With CPPGM_BEHAVIOR_CHECK_APP naming a `cppgm++`, a source fixture whose
# optimized LowIR the driver emitted is also built and run two ways: the
# source at the lane's level, and the emitted LowIR through the object
# path at -O0; both
# programs must print the same output and exit the same way.  A source that
# does not build and link at -O0 has no behaviour to preserve and is
# skipped.  Programs run with a ten-second limit and no input.
sub source_is_lowir
{
	my ($path) = @_;
	my $data = getdata($path);
	return 1 if !defined($data);
	for my $line (split(/\n/, $data))
	{
		next if $line =~ /^\s*$/ || $line =~ /^\s*#/;
		return $line =~ /^(?:declare|global|function|machine_ir)\b/ ? 1 : 0;
	}
	return 1;
}

# A program is bounded like the runners' program runs
# (CPPGM_PROGRAM_TEST_TIMEOUT_SEC, 10 seconds by default).
sub run_program_capture
{
	my ($program, $stdout_path) = @_;
	my $timeout = CppgmBatchWorker::get_timeout_from_env('CPPGM_PROGRAM_TEST_TIMEOUT_SEC', 10);
	my $status = system("timeout $timeout \"$program\" < /dev/null > \"$stdout_path\" 2>/dev/null");
	return $status == -1 ? undef : ($status >> 8);
}

sub check_source_behavior
{
	my ($my, $source_file) = @_;
	my $app = $ENV{CPPGM_BEHAVIOR_CHECK_APP};
	return (1, undef, undef) if !defined($app) || $app eq '' || !defined($source_file);
	return (1, undef, undef) if !-f $source_file || source_is_lowir($source_file);
	my $source_program = "$my.behavior.source";
	my $lowir_input = "$my.behavior.lowir";
	my $lowir_program = "$my.behavior.optimized";
	my $log = "$my.behavior.stderr";
	# The source is built at the lane's level, read from the lane's name
	# (`.../o1/...`), so that what the source sees (`__OPTIMIZE__`) matches
	# what the emitted LowIR saw; the emitted LowIR is lowered at -O0.
	my $level = $my =~ m{(?:^|/)o([0-3])(?:/|$)} ? "-O$1" : '-O0';
	my $built = system("\"$app\" $level -o \"$source_program\" \"$source_file\" > /dev/null 2> \"$log\"") == 0;
	if (!$built)
	{
		unlink($source_program, $log);
		return (1, undef, undef);
	}
	my $data = getdata($my);
	return (0, "ERROR: missing output file ($my)", undef) if !defined($data);
	putrawdata($lowir_input, $data);
	if (system("\"$app\" -O0 -o \"$lowir_program\" \"$lowir_input\" > /dev/null 2> \"$log\"") != 0)
	{
		my $reason = getdata($log);
		$reason = '' if !defined($reason);
		$reason =~ s/\s+/ /g;
		unlink($source_program);
		return (0, "ERROR: optimized LowIR does not build into a program ($reason)",
			"To reproduce:\n\n    \$ $app -O0 -o /tmp/optimized " . rooted_path($lowir_input) . "\n\n");
	}
	my $source_status = run_program_capture($source_program, "$source_program.stdout");
	my $lowir_status = run_program_capture($lowir_program, "$lowir_program.stdout");
	my $source_out = getdata("$source_program.stdout");
	my $lowir_out = getdata("$lowir_program.stdout");
	$source_out = '' if !defined($source_out);
	$lowir_out = '' if !defined($lowir_out);
	my $same = defined($source_status) && defined($lowir_status) &&
		$source_status == $lowir_status && $source_out eq $lowir_out;
	if ($same)
	{
		unlink($source_program, "$source_program.stdout", $lowir_input, $lowir_program, "$lowir_program.stdout", $log);
		return (1, undef, undef);
	}
	return (0, "ERROR: optimized program behaves differently from the source program (exit " .
		(defined($source_status) ? $source_status : 'none') . " -> " .
		(defined($lowir_status) ? $lowir_status : 'none') . ")",
		"To compare:\n\n    \$ diff " . rooted_path("$source_program.stdout") . " " .
		rooted_path("$lowir_program.stdout") . "\n\n");
}

sub compare_text
{
	my ($suffix_pattern, $ref_suffix, $my_suffix, $testbase, $optional_output) = @_;
	my $ref = "$testbase.$ref_suffix";
	my $my = "$testbase.$my_suffix";
	my $ref_status = getdata("$ref.exit_status");
	my $my_status = getdata("$my.exit_status");
	my @missing_status = ();
	push @missing_status, "$ref.exit_status" if !defined($ref_status);
	push @missing_status, "$my.exit_status" if !defined($my_status);
	return (0, "ERROR: missing exit status output (" . join(', ', @missing_status) . ")")
		if scalar(@missing_status) != 0;
	return (0, status_mismatch_message("tool", $ref_status, $my_status))
		if $ref_status ne $my_status;
	return (1, undef) if $ref_status ne 'EXIT_SUCCESS';
	my $ref_data = getdata($ref);
	my $my_data = getdata($my);
	$ref_data = '' if $optional_output && !defined($ref_data);
	$my_data = '' if $optional_output && !defined($my_data);
	my @missing_output = ();
	push @missing_output, $ref if !defined($ref_data);
	push @missing_output, $my if !defined($my_data);
	return (0, "ERROR: missing output file (" . join(', ', @missing_output) . ")")
		if scalar(@missing_output) != 0;
	return ($ref_data eq $my_data
		? (1, undef)
		: (0, "ERROR: checked output does not match reference"));
}

sub compare_lowir_roundtrip
{
	my ($ref_suffix, $my_suffix, $testbase) = @_;
	my ($ok, $message) = compare_text(qr/\.t$/, $ref_suffix, $my_suffix, $testbase);
	return ($ok, $message) if $ok ||
		$message ne 'ERROR: checked output does not match reference' ||
		env_flag_enabled('CPPGM_LOWIR_DIRECT_TEXT_COMPARE');

	# Parse with the supplied LowIR reader rather than applying source-lowering
	# relaxations, which may discard declarations or impose backend constraints.
	my $checker = $ENV{CPPGM_LOWIR_CHECK_APP} // '../dev/lowir-ref';
	CppgmBatchWorker::ensure_test_app_available($checker, 'ref', "$testbase.t");
	my @canonical;
	for my $suffix ($ref_suffix, $my_suffix)
	{
		my $output = "$testbase.$my_suffix.roundtrip.$suffix.lowir";
		unlink($output);
		my $status = CppgmBatchWorker::run_command_capture(
			cmd => [$checker, '-o', $output, "$testbase.$suffix"],
			stdout => "$output.stdout",
			stderr => "$output.stderr",
			timeout => CppgmBatchWorker::get_timeout_from_env('CPPGM_BUILD_TEST_TIMEOUT_SEC', 30));
		return (0, "ERROR: $suffix LowIR cannot be read; inspect $output.stderr")
			if $status != 0 || !-f $output;
		push @canonical, getdata($output);
	}
	return $canonical[0] eq $canonical[1] ? (1, undef) :
		(0, 'ERROR: LowIR roundtrip changed the represented program');
}

sub lowir_symbol_basename
{
	my ($symbol) = @_;
	my $base = $symbol;
	$base =~ s/__ov\d+$//;
	$base =~ s/^.*___//;
	$base =~ s/^.*__//;
	return $base;
}

sub lowir_symbol_is_allowed_external
{
	my ($symbol, $source_text, $for_addr) = @_;
	return 1 if $symbol eq '__cxa_pure_virtual';
	return 1 if $symbol =~ /^__builtin_/;
	return 0;
}

sub lowir_trim
{
	my ($text) = @_;
	return '' if !defined($text);
	$text =~ s/^\s+//;
	$text =~ s/\s+$//;
	return $text;
}

sub lowir_type_pattern
{
	return qr/(?:[A-Za-z0-9_]+|obj<\d+x\d+>)/;
}

sub parse_lowir_object_type
{
	my ($type) = @_;
	return undef if !defined($type);
	return undef if $type !~ /^obj<(\d+)x(\d+)>$/;
	return {
		size => 0 + $1,
		align => 0 + $2,
	};
}

sub parse_lowir_param_list
{
	my ($param_text) = @_;
	$param_text = lowir_trim($param_text);
	return (1, []) if $param_text eq '';

	my @raw_params;
	my $current = '';
	my $bracket_depth = 0;
	for my $ch (split(//, $param_text))
	{
		if ($ch eq '[')
		{
			++$bracket_depth;
			$current .= $ch;
			next;
		}
		if ($ch eq ']')
		{
			return (0, "unterminated parameter metadata")
				if $bracket_depth <= 0;
			--$bracket_depth;
			$current .= $ch;
			next;
		}
		if ($ch eq ',' && $bracket_depth == 0)
		{
			my $trimmed = lowir_trim($current);
			return (0, "invalid parameter syntax: $current")
				if $trimmed eq '';
			push @raw_params, $trimmed;
			$current = '';
			next;
		}
		$current .= $ch;
	}
	return (0, "unterminated parameter metadata")
		if $bracket_depth != 0;
	my $trimmed = lowir_trim($current);
	return (0, "invalid parameter syntax: $current")
		if $trimmed eq '';
	push @raw_params, $trimmed;

	my @params;
	my $type_pattern = lowir_type_pattern();
	for my $param (@raw_params)
	{
		my ($name, $type, $metadata_suffix) =
			($param =~ /^%([A-Za-z0-9_]+)\s*:\s*($type_pattern)((?:\s+\[[^\]]+\])?)$/);
		return (0, "invalid parameter syntax: $param")
			if !defined($name);
		my ($metadata_ok, $metadata_or_error) =
			parse_lowir_parameter_metadata_suffix($metadata_suffix);
		return (0, $metadata_or_error) if !$metadata_ok;
		my $pass = $metadata_or_error->{pass};
		my $capture = $metadata_or_error->{capture};
		my $access = $metadata_or_error->{access};
		my $alias = $metadata_or_error->{alias};
		my $object_bytes = $metadata_or_error->{object_bytes};
		return (0, "unknown parameter pass mode '$pass'")
			if $pass !~ /^(?:direct|indirect_result|by_address|reference|decay)$/;
		return (0, "parameter %$name with pass mode '$pass' must have type ptr")
			if $pass ne 'direct' && $type ne 'ptr';
		return (0, "parameter %$name with capture mode '$capture' must have type ptr")
			if $capture ne '' && $type ne 'ptr';
		return (0, "parameter %$name with access mode '$access' must have type ptr")
			if $access ne '' && $type ne 'ptr';
		return (0, "parameter %$name with alias mode '$alias' must have type ptr")
			if $alias ne '' && $type ne 'ptr';
		return (0, "parameter %$name with object extent '$object_bytes' must have type ptr")
			if $object_bytes ne '' && $type ne 'ptr';
		push @params, {
			name => $name,
			type => $type,
			pass => $pass,
			capture => $capture,
			access => $access,
			alias => $alias,
			object_bytes => $object_bytes,
		};
	}
	return (1, \@params);
}

sub parse_lowir_parameter_metadata_suffix
{
	my ($suffix) = @_;
	my %metadata = (
		pass => 'direct',
		capture => '',
		access => '',
		alias => '',
		object_bytes => '',
	);
	return (1, \%metadata) if lowir_trim($suffix) eq '';
	return (0, "invalid parameter metadata syntax")
		if $suffix !~ /^\s+\[([^\]]+)\]$/;
	my %saw;
	for my $item (split(/\s*,\s*/, $1))
	{
		my ($key, $value) = ($item =~ /^([A-Za-z0-9_]+)\s*=\s*([A-Za-z0-9_]+)$/);
		return (0, "invalid parameter metadata item '$item'")
			if !defined($key);
		return (0, "duplicate parameter metadata key '$key'")
			if $saw{$key};
		if ($key eq 'pass')
		{
			return (0, "unknown parameter pass mode '$value'")
				if $value !~ /^(?:direct|indirect_result|by_address|reference|decay)$/;
			$metadata{pass} = $value;
		}
		elsif ($key eq 'capture')
		{
			return (0, "unknown parameter capture mode '$value'")
				if $value !~ /^(?:nocapture|maycapture)$/;
			$metadata{capture} = $value;
		}
		elsif ($key eq 'access')
		{
			return (0, "unknown parameter access mode '$value'")
				if $value !~ /^(?:read|write|readwrite)$/;
			$metadata{access} = $value;
		}
		elsif ($key eq 'alias')
		{
			return (0, "unknown parameter alias mode '$value'")
				if $value !~ /^(?:noalias)$/;
			$metadata{alias} = $value;
		}
		elsif ($key eq 'object_bytes')
		{
			return (0, "invalid parameter object extent '$value'")
				if $value !~ /^[1-9][0-9]*$/;
			$metadata{object_bytes} = $value;
		}
		elsif ($key eq 'classes')
		{
			return (0, "invalid parameter classes '$value'")
				if $value !~ /^(?:memory|(?:none|integer|sse)(?:_(?:none|integer|sse))?)$/;
		}
		else
		{
			return (0, "unknown parameter metadata key '$key'");
		}
		$saw{$key} = 1;
	}
	return (1, \%metadata);
}

sub parse_lowir_symbol_metadata_suffix
{
	my ($suffix) = @_;
	my %metadata = (
		storage => '',
		role => '',
		linkage => '',
		binding => '',
		object => '',
		keep_alias => '',
		prefer_local => '',
		object_root => '',
	);
	my %saw;
	pos($suffix) = 0;
	while ($suffix =~ /\G\s+\[([^\]]+)\]/gc)
	{
		my $group = $1;
		for my $item (split(/\s*,\s*/, $group))
		{
			my ($key, $value) = ($item =~ /^([A-Za-z0-9_]+)\s*=\s*([^\],\s]+)$/);
			return (0, "invalid symbol metadata item '$item'")
				if !defined($key);
			return (0, "duplicate symbol metadata key '$key'")
				if $saw{$key};
			if ($key eq 'role')
			{
				$metadata{role} = $value;
			}
			elsif ($key eq 'storage')
			{
				return (0, "unknown global storage '$value'")
					if $value !~ /^(?:writable|readonly|thread_local)$/;
				$metadata{storage} = $value;
			}
			elsif ($key eq 'linkage')
			{
				return (0, "unknown language linkage '$value'")
					if $value !~ /^(?:c|cpp)$/;
				$metadata{linkage} = $value;
			}
			elsif ($key eq 'binding')
			{
				return (0, "unknown symbol binding '$value'")
					if $value !~ /^(?:internal|strong|weak)$/;
				$metadata{binding} = $value;
			}
			elsif ($key eq 'object')
			{
				$metadata{object} = $value;
			}
			elsif ($key eq 'keep_alias')
			{
				return (0, "unknown keep_alias mode '$value'")
					if $value !~ /^(?:yes|no)$/;
				$metadata{keep_alias} = $value;
			}
			elsif ($key eq 'prefer_local')
			{
				return (0, "unknown prefer_local mode '$value'")
					if $value !~ /^(?:yes|no)$/;
				$metadata{prefer_local} = $value;
			}
			elsif ($key eq 'object_root')
			{
				return (0, "unknown object_root mode '$value'")
					if $value !~ /^(?:yes|no)$/;
				$metadata{object_root} = $value;
			}
			elsif ($key eq 'replaceable')
			{
				return (0, "unknown replaceable mode '$value'")
					if $value ne 'yes';
			}
			else
			{
				return (0, "unknown symbol metadata key '$key'");
			}
			$saw{$key} = 1;
		}
	}
	my $rest = substr($suffix, pos($suffix) || 0);
	return (0, "invalid trailing symbol metadata '$rest'")
		if lowir_trim($rest) ne '';
	return (1, \%metadata);
}

sub parse_lowir_function_metadata_suffix
{
	my ($suffix) = @_;
	my %metadata = (
		arity => 'fixed',
		effects => '',
		unwind => '',
		return => '',
		role => '',
		linkage => '',
		binding => '',
		object => '',
		tls_for => '',
		keep_alias => '',
		prefer_local => '',
		object_root => '',
		trivial_lifecycle => '',
		force_inline => '',
		inline_hint => '',
		no_inline => '',
	);
	my %saw;
	pos($suffix) = 0;
	while ($suffix =~ /\G\s+\[([^\]]+)\]/gc)
	{
		my $group = $1;
		for my $item (split(/\s*,\s*/, $group))
		{
			my ($key, $value) = ($item =~ /^([A-Za-z0-9_]+)\s*=\s*([^\],\s]+)$/);
			return (0, "invalid function metadata item '$item'")
				if !defined($key);
			return (0, "duplicate function metadata key '$key'")
				if $saw{$key};
			if ($key eq 'arity')
			{
				return (0, "unknown function arity mode '$value'")
					if $value !~ /^(?:fixed|variadic)$/;
				$metadata{arity} = $value;
			}
			elsif ($key eq 'effects')
			{
				return (0, "unknown function effects mode '$value'")
					if $value !~ /^(?:readnone|readonly|readwrite)$/;
				$metadata{effects} = $value;
			}
			elsif ($key eq 'unwind')
			{
				return (0, "unknown function unwind mode '$value'")
					if $value !~ /^(?:no|may)$/;
				$metadata{unwind} = $value;
			}
			elsif ($key eq 'return')
			{
				return (0, "unknown function return mode '$value'")
					if $value !~ /^(?:noreturn|returns)$/;
				$metadata{return} = $value;
			}
			elsif ($key eq 'role')
			{
				$metadata{role} = $value;
			}
			elsif ($key eq 'linkage')
			{
				return (0, "unknown language linkage '$value'")
					if $value !~ /^(?:c|cpp)$/;
				$metadata{linkage} = $value;
			}
			elsif ($key eq 'binding')
			{
				return (0, "unknown symbol binding '$value'")
					if $value !~ /^(?:internal|strong|weak)$/;
				$metadata{binding} = $value;
			}
			elsif ($key eq 'object')
			{
				$metadata{object} = $value;
			}
			elsif ($key eq 'tls_for')
			{
				return (0, "invalid tls_for target '$value'")
					if $value !~ /^@[A-Za-z0-9_]+$/;
				$metadata{tls_for} = $value;
			}
			elsif ($key eq 'keep_alias')
			{
				return (0, "unknown keep_alias mode '$value'")
					if $value !~ /^(?:yes|no)$/;
				$metadata{keep_alias} = $value;
			}
			elsif ($key eq 'prefer_local')
			{
				return (0, "unknown prefer_local mode '$value'")
					if $value !~ /^(?:yes|no)$/;
				$metadata{prefer_local} = $value;
			}
			elsif ($key eq 'object_root')
			{
				return (0, "unknown object_root mode '$value'")
					if $value !~ /^(?:yes|no)$/;
				$metadata{object_root} = $value;
			}
			elsif ($key eq 'trivial_lifecycle')
			{
				return (0, "unknown trivial_lifecycle mode '$value'")
					if $value !~ /^(?:yes|no)$/;
				$metadata{trivial_lifecycle} = $value;
			}
			elsif ($key eq 'force_inline')
			{
				return (0, "unknown force_inline mode '$value'")
					if $value !~ /^(?:yes|no)$/;
				$metadata{force_inline} = $value;
			}
			elsif ($key eq 'inline_hint')
			{
				return (0, "unknown inline_hint mode '$value'")
					if $value !~ /^(?:yes|no)$/;
				$metadata{inline_hint} = $value;
			}
			elsif ($key eq 'no_inline')
			{
				return (0, "unknown no_inline mode '$value'")
					unless $value eq 'yes' || $value eq 'no';
				$metadata{no_inline} = $value;
			}
			elsif ($key eq 'replaceable')
			{
				return (0, "unknown replaceable mode '$value'")
					if $value ne 'yes';
			}
			elsif ($key eq 'result_classes')
			{
				return (0, "invalid result classes '$value'")
					if $value !~ /^(?:memory|(?:none|integer|sse)(?:_(?:none|integer|sse))?)$/;
			}
			else
			{
				return (0, "unknown function metadata key '$key'");
			}
			$saw{$key} = 1;
		}
	}
	my $rest = substr($suffix, pos($suffix) || 0);
	return (0, "invalid trailing function metadata '$rest'")
		if lowir_trim($rest) ne '';
	return (1, \%metadata);
}

sub parse_lowir_call_signature_suffix
{
	my ($suffix) = @_;
	my %signature = (
		has_signature => 0,
		ret => undef,
		params => [],
		arity => 'fixed',
	);
	return (1, \%signature) if lowir_trim($suffix) eq '';
	my $type_pattern = lowir_type_pattern();
	return (0, "invalid call signature syntax")
		if $suffix !~ /^\s+as\s+\((.*?)\)\s*->\s*($type_pattern)((?:\s+\[[^\]]+\])*)$/;
	my ($params_text, $ret_type, $metadata_suffix) = ($1, $2, $3);
	my ($params_ok, $params_or_error) = parse_lowir_param_list($params_text);
	return (0, $params_or_error) if !$params_ok;
	my ($metadata_ok, $metadata_or_error) =
		parse_lowir_function_metadata_suffix($metadata_suffix);
	return (0, $metadata_or_error) if !$metadata_ok;
	return (0, "call signature metadata does not allow symbol metadata")
		if $metadata_or_error->{role} ne '' ||
		   $metadata_or_error->{linkage} ne '' ||
		   $metadata_or_error->{binding} ne '' ||
		   $metadata_or_error->{object} ne '' ||
		   $metadata_or_error->{tls_for} ne '' ||
		   $metadata_or_error->{keep_alias} ne '' ||
		   $metadata_or_error->{prefer_local} ne '' ||
		   $metadata_or_error->{object_root} ne '' ||
		   $metadata_or_error->{trivial_lifecycle} ne '';
	$signature{has_signature} = 1;
	$signature{ret} = $ret_type;
	$signature{params} = $params_or_error;
	$signature{arity} = $metadata_or_error->{arity};
	return (1, \%signature);
}

sub parse_lowir_call_suffix
{
	my ($suffix) = @_;
	my $copy_elision = 0;
	if ($suffix =~ s/^\s+\[\s*elision\s*=\s*copy\s*\]//)
	{
		$copy_elision = 1;
	}
	my ($ok, $signature) = parse_lowir_call_signature_suffix($suffix);
	return ($ok, $signature) if !$ok;
	$signature->{copy_elision} = $copy_elision;
	return (1, $signature);
}

sub split_lowir_args
{
	my ($arg_text) = @_;
	$arg_text = lowir_trim($arg_text);
	return () if $arg_text eq '';
	return map { lowir_trim($_) } split(/\s*,\s*/, $arg_text);
}

sub validate_lowir_operand
{
	my ($operand, $state, $errors, $context) = @_;
	return if !defined($operand) || $operand eq '';
	return if $operand =~ /^-?(?:0x[0-9A-Fa-f]+|\d+)(?:[uUlL]*)$/;
	return if $operand =~ /^-?(?:\d+\.\d*|\d*\.\d+|\d+)(?:[eE][+-]?\d+)?[fFlL]?$/;
	return if $operand eq 'zero';
	if ($operand =~ /^%([A-Za-z0-9_]+)$/)
	{
		push @$errors, "$context uses undefined temporary %$1"
			if !exists($state->{temps}{$1});
		return;
	}
	if ($operand =~ /^\$([A-Za-z0-9_]+)$/)
	{
		push @$errors, "$context uses undefined slot \$$1"
			if !exists($state->{slots}{$1});
		return;
	}
	if ($operand =~ /^\^([A-Za-z0-9_]+)$/)
	{
		push @{$state->{block_targets}}, $1;
		return;
	}
	if ($operand =~ /^\@([A-Za-z0-9_]+)$/)
	{
		my $name = $1;
		return if exists($state->{all_symbols}{$name});
		return if lowir_symbol_is_allowed_external($name, $state->{source_text}, 1);
		push @$errors, "$context references undefined symbol \@$name";
		return;
	}
}

sub validate_lowir_pointer_operand
{
	my ($operand, $state, $errors, $context) = @_;
	validate_lowir_operand($operand, $state, $errors, $context);
	if ($operand =~ /^%([A-Za-z0-9_]+)$/ && exists($state->{temps}{$1}))
	{
		my $type = $state->{temps}{$1};
		push @$errors, "$context expects pointer-valued operand, got %$1 : $type"
			if defined($type) && $type ne 'ptr';
	}
}

sub validate_lowir_value_operand_type
{
	my ($operand, $expected_type, $state, $errors, $context) = @_;
	validate_lowir_operand($operand, $state, $errors, $context);
}

sub validate_lowir_copyobj_source_operand
{
	my ($byte_count, $byte_alignment, $operand, $state, $errors, $context) = @_;
	validate_lowir_operand($operand, $state, $errors, $context);
	if ($operand =~ /^%([A-Za-z0-9_]+)$/ && exists($state->{temps}{$1}))
	{
		my $type = $state->{temps}{$1};
		return if defined($type) && $type eq 'ptr';
		my $object = parse_lowir_object_type($type);
		if (defined($object))
		{
			push @$errors, "$context object-valued source size $object->{size} does not match copyobj byte count $byte_count"
				if $object->{size} != $byte_count;
			push @$errors, "$context object-valued source alignment $object->{align} does not match copyobj alignment $byte_alignment"
				if $object->{align} != $byte_alignment;
			return;
		}
		push @$errors, "$context expects pointer-valued or object-valued source operand, got %$1 : $type";
	}
}

sub parse_lowir_storage_span
{
	my ($text) = @_;
	return undef if $text !~ /^(\d+)(?:x(\d+))?$/;
	my $size = 0 + $1;
	my $alignment = defined($2) ? (0 + $2) : 1;
	return undef if $size <= 0 || $alignment <= 0 || ($alignment & ($alignment - 1)) != 0;
	return {
		size => $size,
		alignment => $alignment,
	};
}

sub validate_lowir_storage_operand
{
	my ($storage, $expected_type, $state, $errors, $context) = @_;
	if ($storage =~ /^\$([A-Za-z0-9_]+)$/)
	{
		my $name = $1;
		if (!exists($state->{slots}{$name}))
		{
			push @$errors, "$context uses undefined slot \$$name";
			return;
		}
		return;
	}
	if ($storage =~ /^\@([A-Za-z0-9_]+)$/)
	{
		my $name = $1;
		if (!exists($state->{global_symbols}{$name}) &&
		    !lowir_symbol_is_allowed_external($name, $state->{source_text}, 1))
		{
			push @$errors, "$context uses undefined global \@$name";
			return;
		}
		return;
	}
	validate_lowir_pointer_operand($storage, $state, $errors, $context);
}

sub validate_lowir_instruction
{
	my ($line, $state, $errors) = @_;
	my $func = $state->{name};
	my $block = $state->{current_block};
	my $context = "function \@$func block ^$block";
	my $type_pattern = lowir_type_pattern();

	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*const\s+([A-Za-z0-9_]+)\s+(.+)$/)
	{
		$state->{temps}{$1} = $2;
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*phi\s+($type_pattern)\s+\[(.*)\]$/)
	{
		my ($destination, $type, $incoming_text) = ($1, $2, $3);
		my @incoming = split(/\s*,\s*/, lowir_trim($incoming_text));
		push @$errors, "$context phi requires an incoming edge"
			if scalar(@incoming) == 0;
		for my $incoming (@incoming)
		{
			if ($incoming !~ /^(\^[A-Za-z0-9_]+)\s*:\s*(.+)$/)
			{
				push @$errors, "$context phi has invalid incoming edge '$incoming'";
				next;
			}
			my ($label, $value) = ($1, $2);
			validate_lowir_operand($label, $state, $errors, "$context phi");
			if ($value =~ /^%([A-Za-z0-9_]+)$/ &&
			    !exists($state->{temps}{$1}))
			{
				push @$errors, "$context phi uses undefined temporary %$1"
					if !exists($state->{all_temps}{$1});
			}
			else
			{
				validate_lowir_value_operand_type(
					$value, $type, $state, $errors, "$context phi");
			}
		}
		$state->{temps}{$destination} = $type;
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*copy\s+([A-Za-z0-9_]+)\s+(.+)$/)
	{
		validate_lowir_value_operand_type($3, $2, $state, $errors, "$context copy");
		$state->{temps}{$1} = $2;
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*addr\s+(@[A-Za-z0-9_]+|\$[A-Za-z0-9_]+)$/)
	{
		validate_lowir_operand($2, $state, $errors, "$context addr");
		$state->{temps}{$1} = 'ptr';
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*load\s+(?:volatile\s+)?(?:typed\s+)?([A-Za-z0-9_]+)\s+(.+)$/)
	{
		validate_lowir_storage_operand($3, $2, $state, $errors, "$context load");
		$state->{temps}{$1} = $2;
		return 0;
	}
	if ($line =~ /^store\s+(?:volatile\s+)?(?:typed\s+)?([A-Za-z0-9_]+)\s+(.+?),\s+(.+)$/)
	{
		validate_lowir_value_operand_type($2, $1, $state, $errors, "$context store");
		validate_lowir_storage_operand($3, $1, $state, $errors, "$context store");
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*atomic_load\s+([A-Za-z0-9_]+)\s+(.+?),\s+(\d+)$/)
	{
		validate_lowir_pointer_operand($3, $state, $errors, "$context atomic_load");
		$state->{temps}{$1} = $2;
		return 0;
	}
	if ($line =~ /^atomic_store\s+([A-Za-z0-9_]+)\s+(.+?),\s+(.+?),\s+(\d+)$/)
	{
		validate_lowir_value_operand_type($2, $1, $state, $errors, "$context atomic_store");
		validate_lowir_pointer_operand($3, $state, $errors, "$context atomic_store");
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*atomic_exchange\s+([A-Za-z0-9_]+)\s+(.+?),\s+(.+?),\s+(\d+)$/)
	{
		validate_lowir_pointer_operand($3, $state, $errors, "$context atomic_exchange");
		validate_lowir_value_operand_type($4, $2, $state, $errors, "$context atomic_exchange");
		$state->{temps}{$1} = $2;
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*atomic_compare_exchange\s+([A-Za-z0-9_]+)\s+(.+?),\s+(.+?),\s+(.+?),\s+(\d+),\s+(\d+)$/)
	{
		validate_lowir_pointer_operand($3, $state, $errors, "$context atomic_compare_exchange");
		validate_lowir_pointer_operand($4, $state, $errors, "$context atomic_compare_exchange");
		validate_lowir_value_operand_type($5, $2, $state, $errors, "$context atomic_compare_exchange");
		$state->{temps}{$1} = undef;
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*index\s+([A-Za-z0-9_]+)(?:\s+\[projection=([A-Za-z0-9_]+)\])?\s+(.+?),\s+(.+)$/)
	{
		if (defined($3) &&
		    $3 ne 'array_element' &&
		    $3 ne 'field' &&
		    $3 ne 'base_subobject' &&
		    $3 ne 'reference_field')
		{
			push @$errors, "$context index has unknown projection '$3'";
		}
		validate_lowir_pointer_operand($4, $state, $errors, "$context index");
		validate_lowir_operand($5, $state, $errors, "$context index");
		$state->{temps}{$1} = 'ptr';
		return 0;
	}
	if ($line =~ /^copyobj\s+([0-9]+(?:x[0-9]+)?)\s+(.+?),\s+(.+)$/)
	{
		my $span = parse_lowir_storage_span($1);
		push @$errors, "$context copyobj must use positive byte count and power-of-two alignment"
			if !defined($span);
		validate_lowir_copyobj_source_operand($span->{size}, $span->{alignment}, $2, $state, $errors, "$context copyobj")
			if defined($span);
		validate_lowir_pointer_operand($3, $state, $errors, "$context copyobj");
		return 0;
	}
	if ($line =~ /^zeroinit\s+([0-9]+(?:x[0-9]+)?)\s+(.+)$/)
	{
		my $span = parse_lowir_storage_span($1);
		push @$errors, "$context zeroinit must use positive byte count and power-of-two alignment"
			if !defined($span);
		validate_lowir_pointer_operand($2, $state, $errors, "$context zeroinit");
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*unary\s+([A-Za-z0-9_]+)\s+([A-Za-z0-9_]+)\s+(.+)$/)
	{
		validate_lowir_value_operand_type($4, $3, $state, $errors, "$context unary");
		$state->{temps}{$1} = $3;
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*binary\s+([A-Za-z0-9_]+)(?:\s+nowrap)?(?:\s+exact)?\s+([A-Za-z0-9_]+)\s+(.+?),\s+(.+)$/)
	{
		validate_lowir_value_operand_type($4, $3, $state, $errors, "$context binary");
		validate_lowir_value_operand_type($5, $3, $state, $errors, "$context binary");
		$state->{temps}{$1} = $3;
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*select\s+([A-Za-z0-9_]+)\s+(.+?),\s+(.+?),\s+(.+)$/)
	{
		validate_lowir_operand($3, $state, $errors, "$context select");
		validate_lowir_value_operand_type($4, $2, $state, $errors, "$context select");
		validate_lowir_value_operand_type($5, $2, $state, $errors, "$context select");
		$state->{temps}{$1} = $2;
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*cmp\s+([A-Za-z0-9_]+)\s+([A-Za-z0-9_]+)\s+(.+?),\s+(.+)$/)
	{
		validate_lowir_value_operand_type($4, $3, $state, $errors, "$context cmp");
		validate_lowir_value_operand_type($5, $3, $state, $errors, "$context cmp");
		$state->{temps}{$1} = undef;
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*convert\s+([A-Za-z0-9_]+)\s+([A-Za-z0-9_]+)\s+([A-Za-z0-9_]+)\s+(.+)$/)
	{
		validate_lowir_value_operand_type($5, $4, $state, $errors, "$context convert");
		$state->{temps}{$1} = $3;
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*atomic_add_fetch\s+([A-Za-z0-9_]+)\s+(.+?),\s+(.+?),\s+(\d+)$/)
	{
		validate_lowir_pointer_operand($3, $state, $errors, "$context atomic_add_fetch");
		validate_lowir_value_operand_type($4, $2, $state, $errors, "$context atomic_add_fetch");
		$state->{temps}{$1} = $2;
		return 0;
	}
	if ($line =~ /^atomic_thread_fence\s+(\d+)$/ || $line =~ /^atomic_signal_fence\s+(\d+)$/)
	{
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*call\s+($type_pattern)\s+@([A-Za-z0-9_]+)\((.*?)\)(.*)$/)
	{
		my ($dst, $ret_type, $callee, $arg_text, $suffix) = ($1, $2, $3, $4, $5);
		my @args = split_lowir_args($arg_text);
		validate_lowir_operand($_, $state, $errors, "$context call \@$callee") for @args;
		my ($sig_ok, $call_sig) = parse_lowir_call_suffix($suffix);
		push @$errors, "$context call \@$callee has invalid signature metadata: $call_sig"
			if !$sig_ok;
		push @$errors, "$context value-returning call \@$callee cannot carry copy-elision permission"
			if $sig_ok && $call_sig->{copy_elision};
		if (exists($state->{function_symbols}{$callee}) && defined($state->{signatures}{$callee}))
		{
			my $sig = $state->{signatures}{$callee};
			push @$errors, "$context call \@$callee expects return type $sig->{ret}, got $ret_type"
				if $sig->{ret} ne $ret_type;
			if ($sig->{arity} eq 'variadic')
			{
				push @$errors, "$context call \@$callee expects at least " .
					scalar(@{$sig->{params}}) . " argument(s), got " . scalar(@args)
					if scalar(@args) < scalar(@{$sig->{params}});
			}
			else
			{
				push @$errors, "$context call \@$callee expects exactly " .
					scalar(@{$sig->{params}}) . " argument(s), got " . scalar(@args)
					if scalar(@args) != scalar(@{$sig->{params}});
			}
			if ($sig_ok && $call_sig->{has_signature})
			{
				push @$errors, "$context direct call \@$callee has mismatched explicit signature return type $call_sig->{ret}"
					if $call_sig->{ret} ne $sig->{ret};
				push @$errors, "$context direct call \@$callee has mismatched explicit signature arity $call_sig->{arity}"
					if $call_sig->{arity} ne $sig->{arity};
				push @$errors, "$context direct call \@$callee has mismatched explicit signature parameter count " .
					scalar(@{$call_sig->{params}}) . " (expected " . scalar(@{$sig->{params}}) . ")"
					if scalar(@{$call_sig->{params}}) != scalar(@{$sig->{params}});
			}
		}
		elsif (exists($state->{global_symbols}{$callee}))
		{
			push @$errors, "$context indirect global call \@$callee requires explicit call signature"
				if !$sig_ok || !$call_sig->{has_signature};
			if ($sig_ok && $call_sig->{has_signature})
			{
				push @$errors, "$context indirect global call \@$callee signature return type $call_sig->{ret} does not match call result type $ret_type"
					if $call_sig->{ret} ne $ret_type;
				if ($call_sig->{arity} eq 'variadic')
				{
					push @$errors, "$context indirect global call \@$callee expects at least " .
						scalar(@{$call_sig->{params}}) . " argument(s), got " . scalar(@args)
						if scalar(@args) < scalar(@{$call_sig->{params}});
				}
				else
				{
					push @$errors, "$context indirect global call \@$callee expects exactly " .
						scalar(@{$call_sig->{params}}) . " argument(s), got " . scalar(@args)
						if scalar(@args) != scalar(@{$call_sig->{params}});
				}
			}
		}
		elsif (!lowir_symbol_is_allowed_external($callee, $state->{source_text}, 0))
		{
			push @$errors, "$context calls undefined function \@$callee";
		}
		$state->{temps}{$dst} = $ret_type;
		return 0;
	}
	if ($line =~ /^call\s+void\s+@([A-Za-z0-9_]+)\((.*?)\)(.*)$/)
	{
		my ($callee, $arg_text, $suffix) = ($1, $2, $3);
		my @args = split_lowir_args($arg_text);
		validate_lowir_operand($_, $state, $errors, "$context call \@$callee") for @args;
		my ($sig_ok, $call_sig) = parse_lowir_call_suffix($suffix);
		push @$errors, "$context call \@$callee has invalid signature metadata: $call_sig"
			if !$sig_ok;
		push @$errors, "$context copy-elision call \@$callee needs destination and source arguments"
			if $sig_ok && $call_sig->{copy_elision} && scalar(@args) < 2;
		push @$errors, "$context copy-elision permission cannot carry an explicit call signature"
			if $sig_ok && $call_sig->{copy_elision} && $call_sig->{has_signature};
		push @$errors, "$context copy-elision permission requires a direct function call"
			if $sig_ok && $call_sig->{copy_elision} &&
			   !exists($state->{function_symbols}{$callee});
		if (exists($state->{function_symbols}{$callee}) && defined($state->{signatures}{$callee}))
		{
			my $sig = $state->{signatures}{$callee};
			push @$errors, "$context call \@$callee expects return type $sig->{ret}, got void"
				if $sig->{ret} ne 'void';
			if ($sig->{arity} eq 'variadic')
			{
				push @$errors, "$context call \@$callee expects at least " .
					scalar(@{$sig->{params}}) . " argument(s), got " . scalar(@args)
					if scalar(@args) < scalar(@{$sig->{params}});
			}
			else
			{
				push @$errors, "$context call \@$callee expects exactly " .
					scalar(@{$sig->{params}}) . " argument(s), got " . scalar(@args)
					if scalar(@args) != scalar(@{$sig->{params}});
			}
			if ($sig_ok && $call_sig->{has_signature})
			{
				push @$errors, "$context direct call \@$callee has mismatched explicit signature return type $call_sig->{ret}"
					if $call_sig->{ret} ne 'void';
				push @$errors, "$context direct call \@$callee has mismatched explicit signature arity $call_sig->{arity}"
					if $call_sig->{arity} ne $sig->{arity};
				push @$errors, "$context direct call \@$callee has mismatched explicit signature parameter count " .
					scalar(@{$call_sig->{params}}) . " (expected " . scalar(@{$sig->{params}}) . ")"
					if scalar(@{$call_sig->{params}}) != scalar(@{$sig->{params}});
			}
		}
		elsif (exists($state->{global_symbols}{$callee}))
		{
			push @$errors, "$context indirect global call \@$callee requires explicit call signature"
				if !$sig_ok || !$call_sig->{has_signature};
			if ($sig_ok && $call_sig->{has_signature})
			{
				push @$errors, "$context indirect global call \@$callee signature return type $call_sig->{ret} does not match call result type void"
					if $call_sig->{ret} ne 'void';
				if ($call_sig->{arity} eq 'variadic')
				{
					push @$errors, "$context indirect global call \@$callee expects at least " .
						scalar(@{$call_sig->{params}}) . " argument(s), got " . scalar(@args)
						if scalar(@args) < scalar(@{$call_sig->{params}});
				}
				else
				{
					push @$errors, "$context indirect global call \@$callee expects exactly " .
						scalar(@{$call_sig->{params}}) . " argument(s), got " . scalar(@args)
						if scalar(@args) != scalar(@{$call_sig->{params}});
				}
			}
		}
		elsif (!lowir_symbol_is_allowed_external($callee, $state->{source_text}, 0))
		{
			push @$errors, "$context calls undefined function \@$callee";
		}
		return 0;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*call\s+($type_pattern)\s+(.+?)\((.*?)\)(.*)$/)
	{
		my ($dst, $ret_type, $callee, $arg_text, $suffix) = ($1, $2, $3, $4, $5);
		validate_lowir_pointer_operand($callee, $state, $errors, "$context indirect call");
		my @args = split_lowir_args($arg_text);
		validate_lowir_operand($_, $state, $errors, "$context indirect call") for @args;
		my ($sig_ok, $call_sig) = parse_lowir_call_suffix($suffix);
		push @$errors, "$context indirect call has invalid signature metadata: $call_sig"
			if !$sig_ok;
		push @$errors, "$context indirect call cannot carry copy-elision permission"
			if $sig_ok && $call_sig->{copy_elision};
		push @$errors, "$context indirect call requires explicit call signature"
			if !$sig_ok || !$call_sig->{has_signature};
		if ($sig_ok && $call_sig->{has_signature})
		{
			push @$errors, "$context indirect call signature return type $call_sig->{ret} does not match call result type $ret_type"
				if $call_sig->{ret} ne $ret_type;
			if ($call_sig->{arity} eq 'variadic')
			{
				push @$errors, "$context indirect call expects at least " .
					scalar(@{$call_sig->{params}}) . " argument(s), got " . scalar(@args)
					if scalar(@args) < scalar(@{$call_sig->{params}});
			}
			else
			{
				push @$errors, "$context indirect call expects exactly " .
					scalar(@{$call_sig->{params}}) . " argument(s), got " . scalar(@args)
					if scalar(@args) != scalar(@{$call_sig->{params}});
			}
		}
		$state->{temps}{$dst} = $ret_type;
		return 0;
	}
	if ($line =~ /^call\s+void\s+(.+?)\((.*?)\)(.*)$/)
	{
		validate_lowir_pointer_operand($1, $state, $errors, "$context indirect call");
		my @args = split_lowir_args($2);
		validate_lowir_operand($_, $state, $errors, "$context indirect call") for @args;
		my ($sig_ok, $call_sig) = parse_lowir_call_suffix($3);
		push @$errors, "$context indirect call has invalid signature metadata: $call_sig"
			if !$sig_ok;
		push @$errors, "$context indirect call cannot carry copy-elision permission"
			if $sig_ok && $call_sig->{copy_elision};
		push @$errors, "$context indirect call requires explicit call signature"
			if !$sig_ok || !$call_sig->{has_signature};
		if ($sig_ok && $call_sig->{has_signature})
		{
			push @$errors, "$context indirect call signature return type $call_sig->{ret} does not match call result type void"
				if $call_sig->{ret} ne 'void';
			if ($call_sig->{arity} eq 'variadic')
			{
				push @$errors, "$context indirect call expects at least " .
					scalar(@{$call_sig->{params}}) . " argument(s), got " . scalar(@args)
					if scalar(@args) < scalar(@{$call_sig->{params}});
			}
			else
			{
				push @$errors, "$context indirect call expects exactly " .
					scalar(@{$call_sig->{params}}) . " argument(s), got " . scalar(@args)
					if scalar(@args) != scalar(@{$call_sig->{params}});
			}
		}
		return 0;
	}
	if ($line =~ /^jump\s+(\^[A-Za-z0-9_]+)$/)
	{
		validate_lowir_operand($1, $state, $errors, "$context jump");
		return 1;
	}
	if ($line =~ /^branch\s+(.+?),\s+(\^[A-Za-z0-9_]+),\s+(\^[A-Za-z0-9_]+)$/)
	{
		validate_lowir_operand($1, $state, $errors, "$context branch");
		validate_lowir_operand($2, $state, $errors, "$context branch");
		validate_lowir_operand($3, $state, $errors, "$context branch");
		return 1;
	}
	if ($line =~ /^switch\s+(.+?),\s+(\^[A-Za-z0-9_]+)(.*)$/)
	{
		my ($selector, $default_target, $rest) = ($1, $2, $3);
		validate_lowir_operand($selector, $state, $errors, "$context switch");
		validate_lowir_operand($default_target, $state, $errors, "$context switch");
		$rest = lowir_trim($rest);
		if ($rest ne '')
		{
			if ($rest !~ s/^,\s*//)
			{
				push @$errors, "$context switch has invalid case list: $rest";
				return 1;
			}
			for my $case (split(/\s*,\s*/, $rest))
			{
				if ($case !~ /^(.+):(\^[A-Za-z0-9_]+)$/)
				{
					push @$errors, "$context switch has invalid case arm: $case";
					next;
				}
				validate_lowir_operand($1, $state, $errors, "$context switch");
				validate_lowir_operand($2, $state, $errors, "$context switch");
			}
		}
		return 1;
	}
	if ($line =~ /^return\s+void$/)
	{
		push @$errors, "$context returns void from function returning $state->{ret}"
			if $state->{ret} ne 'void';
		return 1;
	}
	if ($line =~ /^return\s+($type_pattern)\s+(.+)$/)
	{
		validate_lowir_value_operand_type($2, $1, $state, $errors, "$context return");
		push @$errors, "$context returns $1 from function returning $state->{ret}"
			if $state->{ret} ne $1;
		return 1;
	}
	if ($line =~ /^eh_try\s+(\^[A-Za-z0-9_]+)$/ || $line =~ /^eh_cleanup\s+(\^[A-Za-z0-9_]+)$/)
	{
		validate_lowir_operand($1, $state, $errors, "$context handler setup");
		return 0;
	}
	if ($line =~ /^eh_end$/)
	{
		return 0;
	}
	if ($line =~ /^throw\s+($type_pattern)\s+(.+)$/)
	{
		validate_lowir_value_operand_type($2, $1, $state, $errors, "$context throw");
		return 1;
	}
	if ($line =~ /^resume$/)
	{
		return 1;
	}
	if ($line =~ /^unreachable$/)
	{
		return 1;
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=\s*exception\s+($type_pattern)$/)
	{
		$state->{temps}{$1} = $2;
		return 0;
	}

	my $rhs = $line;
	$rhs =~ s/^%[A-Za-z0-9_]+\s*=\s*//;
	for my $operand ($rhs =~ /(%[A-Za-z0-9_]+|\$[A-Za-z0-9_]+|\^[A-Za-z0-9_]+|@[A-Za-z0-9_]+)/g)
	{
		validate_lowir_operand($operand, $state, $errors, "$context instruction");
	}
	if ($line =~ /^%([A-Za-z0-9_]+)\s*=/)
	{
		$state->{temps}{$1} = undef;
	}
	return 0;
}

sub validate_lowir_structure
{
	my ($data, $source_text, $all_symbols, $function_symbols, $global_symbols, $global_types, $signatures) = @_;
	my @errors;
	my @lines = split(/\n/, $data);
	my $type_pattern = lowir_type_pattern();

	for (my $i = 0; $i < scalar(@lines); ++$i)
	{
		my $line = $lines[$i];
		next if $line =~ /^\s*$/;

		if ($line =~ /^global @([A-Za-z0-9_]+)(?:\s+readonly)?((?:\s+\[[^\]]+\])*)\s*=\s*\{$/)
		{
			++$i while $i + 1 < scalar(@lines) && $lines[$i + 1] !~ /^\}$/;
			next;
		}
		next if $line =~ /^declare global @([A-Za-z0-9_]+)(?:\s*:\s*$type_pattern)?((?:\s+\[[^\]]+\])*)$/;
		next if $line =~ /^global @([A-Za-z0-9_]+)(?:\s+readonly)?\s*:\s*$type_pattern((?:\s+\[[^\]]+\])*)\s*=/;

		next if $line !~ /^function @([A-Za-z0-9_]+)\((.*?)\) -> ($type_pattern)((?:\s+\[[^\]]+\])*) \{$/;
		my ($name, $param_text, $ret_type, $metadata_suffix) = ($1, $2, $3, $4);
		my ($params_ok, $params_or_error) = parse_lowir_param_list($param_text);
		if (!$params_ok)
		{
			push @errors, "function \@$name has invalid parameter list: $params_or_error";
			next;
		}
		my ($metadata_ok, $metadata_or_error) =
			parse_lowir_function_metadata_suffix($metadata_suffix);
		if (!$metadata_ok)
		{
			push @errors, "function \@$name has invalid metadata: $metadata_or_error";
			next;
		}
		for (my $pi = 0; $pi < scalar(@$params_or_error); ++$pi)
		{
			next if $params_or_error->[$pi]{pass} ne 'indirect_result';
			push @errors, "function \@$name indirect result parameter must appear first"
				if $pi != 0;
			push @errors, "function \@$name indirect result parameter requires return type void"
				if $ret_type ne 'void';
		}

		my %temps = map { $_->{name} => $_->{type} } @$params_or_error;
		my %all_temps = %temps;
		for (my $scan = $i + 1;
		     $scan < scalar(@lines) && $lines[$scan] !~ /^\}$/; ++$scan)
		{
			$all_temps{$1} = 1
				if $lines[$scan] =~ /^\s*%([A-Za-z0-9_]+)\s*=/;
		}
		my %slots;
		my %blocks;
		my %block_has_terminator;
		my @block_targets;
		my $current_block;
		my $saw_block = 0;

		++$i;
		for (; $i < scalar(@lines); ++$i)
		{
			my $body = $lines[$i];
			last if $body =~ /^\}$/;
			next if $body =~ /^\s*$/;

			if ($body =~ /^\s*slot \$([A-Za-z0-9_]+)\s*:\s*($type_pattern)$/)
			{
				push @errors, "function \@$name declares slot after blocks started"
					if $saw_block;
				push @errors, "function \@$name has duplicate slot \$$1"
					if exists($slots{$1});
				$slots{$1} = $2;
				next;
			}

			if ($body =~ /^\s*block \^([A-Za-z0-9_]+):$/)
			{
				push @errors, "function \@$name block ^$current_block is missing terminator"
					if defined($current_block) && !$block_has_terminator{$current_block};
				push @errors, "function \@$name has duplicate block ^$1"
					if exists($blocks{$1});
				$blocks{$1} = 1;
				$block_has_terminator{$1} = 0;
				$current_block = $1;
				$saw_block = 1;
				next;
			}

			if (!defined($current_block))
			{
				push @errors, "function \@$name has instruction outside of any block: $body";
				next;
			}

			if ($block_has_terminator{$current_block})
			{
				push @errors, "function \@$name block ^$current_block has instruction after terminator: $body";
				next;
			}

			my %state = (
				name => $name,
				ret => $ret_type,
				current_block => $current_block,
				temps => \%temps,
				all_temps => \%all_temps,
				slots => \%slots,
				block_targets => \@block_targets,
				all_symbols => $all_symbols,
				function_symbols => $function_symbols,
				global_symbols => $global_symbols,
				global_types => $global_types,
				signatures => $signatures,
				source_text => $source_text,
			);
			$block_has_terminator{$current_block} =
				validate_lowir_instruction(lowir_trim($body), \%state, \@errors);
		}

		push @errors, "function \@$name contains no blocks"
			if !$saw_block;
		push @errors, "function \@$name block ^$current_block is missing terminator"
			if defined($current_block) && !$block_has_terminator{$current_block};

		my %missing_blocks;
		for my $target (@block_targets)
		{
			$missing_blocks{$target} = 1 if !exists($blocks{$target});
		}
		push @errors, "function \@$name references undefined block target(s): " .
			join(', ', map { "^$_" } sort keys(%missing_blocks))
			if scalar(keys(%missing_blocks)) > 0;
	}

	return @errors;
}

sub lowir_top_level_order_kind
{
	my ($line) = @_;
	return ('declare global', 0, $1)
		if $line =~ /^declare global @([A-Za-z0-9_]+)\b/;
	return ('declare function', 1, $1)
		if $line =~ /^declare function @([A-Za-z0-9_]+)\b/;
	return ('global', 2, $1)
		if $line =~ /^global @([A-Za-z0-9_]+)\b/;
	return ('function', 3, $1)
		if $line =~ /^function @([A-Za-z0-9_]+)\b/;
	return ('alias object', 4, $1)
		if $line =~ /^alias object ([^\s=]+)\s*=/;
	return undef;
}

sub validate_lowir_top_level_order
{
	my ($data) = @_;
	my @errors;
	my @lines = split(/\n/, $data);
	my $last_rank = -1;
	my $last_kind = 'start of file';
	my $last_symbol = '';
	my $expected =
		'declare global, declare function, global, function, alias object';

	for (my $i = 0; $i < scalar(@lines); ++$i)
	{
		my ($kind, $rank, $symbol) = lowir_top_level_order_kind($lines[$i]);
		next if !defined($kind);
		if ($rank < $last_rank)
		{
			my $previous = $last_kind;
			$previous .= " \@$last_symbol" if $last_symbol ne '';
			push @errors, "top-level LowIR order violation at line " . ($i + 1) .
				": $kind \@$symbol appears after $previous; expected order is $expected";
		}
		$last_rank = $rank if $rank > $last_rank;
		$last_kind = $kind;
		$last_symbol = $symbol;
	}

	return @errors;
}

sub lowir_special_member_order_keys
{
	my ($object_symbol) = @_;
	return () if !defined($object_symbol) || $object_symbol eq '';

	if ($object_symbol =~ /^(.*)C([12])([^ ]*)$/)
	{
		my ($owner, $entry, $suffix) = ($1, $2, $3);
		my @keys;
		my $constructor_kind = '';
		if ($suffix =~ /^ERK/)
		{
			$constructor_kind = 'copy constructor';
			push @keys, {
				group => "$owner constructor copy-move",
				rank => 0,
				description => 'copy constructor',
				expectation => 'copy constructors must precede move constructors',
			};
		}
		elsif ($suffix =~ /^EO/)
		{
			$constructor_kind = 'move constructor';
			push @keys, {
				group => "$owner constructor copy-move",
				rank => 1,
				description => 'move constructor',
				expectation => 'copy constructors must precede move constructors',
			};
		}
		else
		{
			$constructor_kind = 'constructor';
		}
		my $entry_rank = $entry eq '2' ? 0 : 1;
		my $entry_description = $entry eq '2' ? 'base entry' : 'complete entry';
		push @keys, {
			group => "$owner constructor entry $suffix",
			rank => $entry_rank,
			description => "$constructor_kind $entry_description",
			expectation => 'constructor base entries must precede complete entries',
		};
		return @keys;
	}

	if ($object_symbol =~ /^(.*)aS([^ ]*)$/)
	{
		my ($owner, $suffix) = ($1, $2);
		if ($suffix =~ /^ERK/)
		{
			return {
				group => "$owner assignment copy-move",
				rank => 30,
				description => 'copy assignment',
				expectation => 'copy assignments must precede move assignments',
			};
		}
		if ($suffix =~ /^EO/)
		{
			return {
				group => "$owner assignment copy-move",
				rank => 40,
				description => 'move assignment',
				expectation => 'copy assignments must precede move assignments',
			};
		}
		return ();
	}

	if ($object_symbol =~ /^(.*)D([012])(?:Ev|E.*)$/)
	{
		my ($owner, $entry) = ($1, $2);
		my %rank = (
			2 => 50,
			0 => 51,
			1 => 52,
		);
		my %description = (
			2 => 'destructor base entry',
			0 => 'destructor deleting entry',
			1 => 'destructor complete entry',
		);
		return {
			group => "$owner destructor entry",
			rank => $rank{$entry},
			description => $description{$entry},
			expectation => 'destructor entries must be ordered base, deleting, complete',
		};
	}

	return ();
}

sub validate_lowir_special_member_order
{
	my ($data) = @_;
	my @errors;
	my %last_by_group;
	my $type_pattern = lowir_type_pattern();
	my @lines = split(/\n/, $data);

	for (my $i = 0; $i < scalar(@lines); ++$i)
	{
		my $line = $lines[$i];
		next if $line !~ /^function @([A-Za-z0-9_]+)\((.*?)\) -> ($type_pattern)((?:\s+\[[^\]]+\])*) \{$/;
		my ($symbol, $metadata_suffix) = ($1, $4);
		my ($ok, $metadata_or_error) = parse_lowir_function_metadata_suffix($metadata_suffix);
		next if !$ok;
		my @keys = lowir_special_member_order_keys($metadata_or_error->{object});
		for my $key (@keys)
		{
			my $group = $key->{group};
			if (exists($last_by_group{$group}) &&
			    $key->{rank} < $last_by_group{$group}{rank})
			{
				push @errors, "special member LowIR order violation at line " . ($i + 1) .
					": function \@$symbol ($key->{description}) appears after " .
					"$last_by_group{$group}{description}; $key->{expectation}";
			}
			$last_by_group{$group} = {
				rank => $key->{rank},
				description => $key->{description},
			};
		}
	}

	return @errors;
}

sub lowir_destructor_entry_from_object_symbol
{
	my ($object_symbol) = @_;
	return undef if !defined($object_symbol) || $object_symbol eq '';
	return undef if $object_symbol !~ /^(.*)D([012])(?:Ev|E.*)$/;
	return {
		owner => $1,
		entry => $2,
	};
}

sub validate_lowir_vtable_destructor_slot_order
{
	my ($data) = @_;
	my @errors;
	my %function_object;
	my $type_pattern = lowir_type_pattern();
	my @lines = split(/\n/, $data);

	for (my $i = 0; $i < scalar(@lines); ++$i)
	{
		my $line = $lines[$i];
		next if $line !~ /^function @([A-Za-z0-9_]+)\((.*?)\) -> ($type_pattern)((?:\s+\[[^\]]+\])*) \{$/;
		my ($symbol, $metadata_suffix) = ($1, $4);
		my ($ok, $metadata_or_error) = parse_lowir_function_metadata_suffix($metadata_suffix);
		next if !$ok;
		next if !exists($metadata_or_error->{object});
		$function_object{$symbol} = $metadata_or_error->{object};
	}

	for (my $i = 0; $i < scalar(@lines); ++$i)
	{
		next if $lines[$i] !~ /^global @([A-Za-z0-9_]+)\b.*=\s*\{$/;
		my $global = $1;
		next if $global !~ /vtable/;

		my @slots;
		for (++$i; $i < scalar(@lines); ++$i)
		{
			last if $lines[$i] =~ /^\}$/;
			next if $lines[$i] !~ /^\s*ptr addr @([A-Za-z0-9_]+)(?:\s|$)/;
			my $target = $1;
			my $object = $function_object{$target};
			my $entry = lowir_destructor_entry_from_object_symbol($object);
			push @slots, {
				symbol => $target,
				object => $object,
				owner => defined($entry) ? $entry->{owner} : '',
				entry => defined($entry) ? $entry->{entry} : '',
			};
		}

		for (my $slot = 0; $slot < scalar(@slots); ++$slot)
		{
			my $entry = $slots[$slot]{entry};
			next if $entry eq '';
			if ($entry eq '2')
			{
				push @errors, "vtable destructor slot order violation in \@$global: " .
					"function \@$slots[$slot]{symbol} is a base destructor entry; " .
					"vtable destructor slots must be complete, deleting";
				next;
			}
			if ($entry eq '1')
			{
				if ($slot + 1 >= scalar(@slots) ||
				    $slots[$slot + 1]{owner} ne $slots[$slot]{owner} ||
				    $slots[$slot + 1]{entry} ne '0')
				{
					push @errors, "vtable destructor slot order violation in \@$global: " .
						"function \@$slots[$slot]{symbol} (complete destructor entry) " .
						"must be immediately followed by the matching deleting destructor entry";
				}
				next;
			}
			if ($entry eq '0')
			{
				if ($slot == 0 ||
				    $slots[$slot - 1]{owner} ne $slots[$slot]{owner} ||
				    $slots[$slot - 1]{entry} ne '1')
				{
					push @errors, "vtable destructor slot order violation in \@$global: " .
						"function \@$slots[$slot]{symbol} (deleting destructor entry) " .
						"must appear immediately after the matching complete destructor entry";
				}
			}
		}
	}

	return @errors;
}

sub validate_lowir_function_role_order
{
	my ($data) = @_;
	my @errors;
	my $seen_fini = 0;
	my $fini_symbol = '';
	my $type_pattern = lowir_type_pattern();
	my @lines = split(/\n/, $data);

	for (my $i = 0; $i < scalar(@lines); ++$i)
	{
		my $line = $lines[$i];
		next if $line !~ /^function @([A-Za-z0-9_]+)\((.*?)\) -> ($type_pattern)((?:\s+\[[^\]]+\])*) \{$/;
		my ($symbol, $metadata_suffix) = ($1, $4);
		my ($ok, $metadata_or_error) = parse_lowir_function_metadata_suffix($metadata_suffix);
		next if !$ok;
		next if !exists($metadata_or_error->{role});
		if ($metadata_or_error->{role} eq 'fini')
		{
			$seen_fini = 1;
			$fini_symbol = $symbol;
			next;
		}
		if ($metadata_or_error->{role} eq 'init' && $seen_fini)
		{
			push @errors, "LowIR role order violation at line " . ($i + 1) .
				": init function \@$symbol appears after fini function \@$fini_symbol";
		}
	}

	return @errors;
}

sub lowir_role_owner_kind
{
	my ($role) = @_;
	return '' if !defined($role) || $role eq '';
	return 'function' if $role =~ /^(?:entry|init|fini|eh_allocate_exception|eh_begin_catch|eh_end_catch|eh_rethrow|eh_throw|eh_personality|eh_resume|allocate_memory|free_memory|terminate|pure_virtual|dynamic_cast|bad_cast|bad_typeid|bad_array_new_length|unreachable)$/;
	return 'global' if $role =~ /^(?:rtti_class|rtti_si|rtti_vmi|rtti_pointer|rtti_data)$/;
	return '';
}

sub validate_lowir_roles
{
	my ($data) = @_;
	my @errors;
	my %role_owner;
	my $type_pattern = lowir_type_pattern();

	for my $line (split(/\n/, $data))
	{
		next if $line =~ /^\s*$/;
		my ($symbol, $kind, $role);
		if ($line =~ /^declare global @([A-Za-z0-9_]+)(?:\s*:\s*$type_pattern)?((?:\s+\[[^\]]+\])*)$/)
		{
			my ($ok, $metadata_or_error) = parse_lowir_symbol_metadata_suffix($2);
			if (!$ok)
			{
				push @errors, "declare global \@$1 has invalid metadata: $metadata_or_error";
				next;
			}
			($symbol, $role) = ($1, $metadata_or_error->{role});
			$kind = 'global';
		}
		elsif ($line =~ /^global @([A-Za-z0-9_]+)(?:\s+readonly)?(?:\s*:\s*$type_pattern)?((?:\s+\[[^\]]+\])*)\s*=/)
		{
			my ($ok, $metadata_or_error) = parse_lowir_symbol_metadata_suffix($2);
			if (!$ok)
			{
				push @errors, "global \@$1 has invalid metadata: $metadata_or_error";
				next;
			}
			($symbol, $role) = ($1, $metadata_or_error->{role});
			$kind = 'global';
		}
		elsif ($line =~ /^declare function @([A-Za-z0-9_]+)\((.*?)\) -> ($type_pattern)((?:\s+\[[^\]]+\])*)$/)
		{
			my ($ok, $metadata_or_error) = parse_lowir_function_metadata_suffix($4);
			if (!$ok)
			{
				push @errors, "declare function \@$1 has invalid metadata: $metadata_or_error";
				next;
			}
			($symbol, $role) = ($1, $metadata_or_error->{role});
			$kind = 'function';
		}
		elsif ($line =~ /^function @([A-Za-z0-9_]+)\((.*?)\) -> ($type_pattern)((?:\s+\[[^\]]+\])*) \{$/)
		{
			my ($ok, $metadata_or_error) = parse_lowir_function_metadata_suffix($4);
			if (!$ok)
			{
				push @errors, "function \@$1 has invalid metadata: $metadata_or_error";
				next;
			}
			($symbol, $role) = ($1, $metadata_or_error->{role});
			$kind = 'function';
		}
		next if !defined($role) || $role eq '';

		my $expected_kind = lowir_role_owner_kind($role);
		if ($expected_kind eq '')
		{
			push @errors, "unknown LowIR role '$role' on \@$symbol";
			next;
		}
		if ($expected_kind ne $kind)
		{
			push @errors, "LowIR role '$role' has invalid $kind owner \@$symbol";
			next;
		}
		if ($role ne 'rtti_data' &&
			exists($role_owner{$role}) && $role_owner{$role} ne $symbol)
		{
			push @errors, "duplicate LowIR role '$role': \@$role_owner{$role}, \@$symbol";
			next;
		}
		$role_owner{$role} = $symbol;
	}

	return @errors;
}

sub validate_lowir_text
{
	my ($data, $source_file, $options) = @_;
	$options = {} if !defined($options);
	return (1, undef) if !defined($data) || $data eq '';

	my $source_text = getrawdata($source_file);
	my @errors;
	push @errors, "the symbol key trivial_lifecycle= is not defined by the LowIR format"
		if !$options->{allow_legacy_trivial_lifecycle} &&
		   $data =~ /\btrivial_lifecycle\s*=/;
	push @errors, "the function role unreachable is not defined by the LowIR format"
		if !$options->{allow_legacy_unreachable_role} &&
		   $data =~ /\brole\s*=\s*unreachable\b/;
	push @errors, "a decay operation or pass=decay is not defined by the LowIR format"
		if !$options->{allow_legacy_decay_surface} &&
		   ($data =~ /\bpass\s*=\s*decay\b/ ||
		    $data =~ /^\s*%[A-Za-z0-9_]+\s*=\s*unary\s+decay\s+ptr\b/m);
	push @errors, "the parameter label pass=reference is not defined by the LowIR format"
		if !$options->{allow_legacy_reference_pass} &&
		   $data =~ /\bpass\s*=\s*reference\b/;
	push @errors, "the parameter keys capture= and access= are not defined by the LowIR format"
		if !$options->{allow_legacy_capture_access} &&
		   $data =~ /\b(?:capture|access)\s*=/;
	push @errors, "the index projections base_subobject and reference_field are not defined by the LowIR format"
		if !$options->{allow_legacy_source_origin_projection} &&
		   $data =~ /\bprojection\s*=\s*(?:base_subobject|reference_field)\b/;

	my @tops = ($data =~ /^(?:declare\s+(?:function|global)|function|global)\s+@([A-Za-z0-9_]+)\b/gm);
	my %top_count;
	for my $name (@tops)
	{
		++$top_count{$name};
	}
	my @duplicates = sort grep { $top_count{$_} > 1 } keys(%top_count);
	push @errors, "duplicate LowIR symbol entries: " . join(', ', @duplicates) if scalar(@duplicates) > 0;

	my %alias_count;
	my @alias_targets;
	for my $line (split(/\n/, $data))
	{
		next if $line !~ /^alias object\b/;
		if ($line !~ /^alias object ([^\s=]+)\s*=\s*@([A-Za-z0-9_]+)\s*$/)
		{
			push @errors, "invalid object alias syntax: $line";
			next;
		}
		++$alias_count{$1};
		push @alias_targets, $2;
	}
	my @duplicate_aliases = sort grep { $alias_count{$_} > 1 } keys(%alias_count);
	push @errors, "duplicate LowIR object aliases: " . join(', ', @duplicate_aliases)
		if scalar(@duplicate_aliases) > 0;
	if ($options->{strict_presentation_order})
	{
		push @errors, validate_lowir_top_level_order($data);
		push @errors, validate_lowir_special_member_order($data);
		push @errors, validate_lowir_function_role_order($data);
	}
	push @errors, validate_lowir_vtable_destructor_slot_order($data);

	my %all_symbols;
	my %function_symbols;
	my %global_symbols;
	my %global_types;
	my $type_pattern = lowir_type_pattern();
	while ($data =~ /^global @([A-Za-z0-9_]+)(?:\s+readonly)?\s*:\s*($type_pattern)((?:\s+\[[^\]]+\])*)\s*=/gm)
	{
		$all_symbols{$1} = 1;
		$global_symbols{$1} = 1;
		$global_types{$1} = $2;
	}
	while ($data =~ /^global @([A-Za-z0-9_]+)(?:\s+readonly)?((?:\s+\[[^\]]+\])*)\s*=\s*\{$/gm)
	{
		$all_symbols{$1} = 1;
		$global_symbols{$1} = 1;
	}
	while ($data =~ /^declare global @([A-Za-z0-9_]+)\s*:\s*($type_pattern)((?:\s+\[[^\]]+\])*)$/gm)
	{
		$all_symbols{$1} = 1;
		$global_symbols{$1} = 1;
		$global_types{$1} = $2;
	}
	while ($data =~ /^declare global @([A-Za-z0-9_]+)((?:\s+\[[^\]]+\])*)$/gm)
	{
		$all_symbols{$1} = 1;
		$global_symbols{$1} = 1;
	}
	my %signatures;
	while ($data =~ /^function @([A-Za-z0-9_]+)\((.*?)\) -> ($type_pattern)((?:\s+\[[^\]]+\])*) \{$/gm)
	{
		$all_symbols{$1} = 1;
		$function_symbols{$1} = 1;
		my ($ok, $params_or_error) = parse_lowir_param_list($2);
		if (!$ok)
		{
			push @errors, "function \@$1 has invalid parameter list: $params_or_error";
			next;
		}
		my ($metadata_ok, $metadata_or_error) =
			parse_lowir_function_metadata_suffix($4);
		# Invalid metadata is reported, but the signature is still recorded
		# from the parameter list and return type so that a call to the
		# function is checked against something instead of nothing.
		push @errors, "function \@$1 has invalid metadata: $metadata_or_error"
			if !$metadata_ok;
		$signatures{$1} = {
			ret => $3,
			params => $params_or_error,
			arity => $metadata_ok ? $metadata_or_error->{arity} : 'fixed',
		};
	}
	while ($data =~ /^declare function @([A-Za-z0-9_]+)\((.*?)\) -> ($type_pattern)((?:\s+\[[^\]]+\])*)$/gm)
	{
		$all_symbols{$1} = 1;
		$function_symbols{$1} = 1;
		my ($ok, $params_or_error) = parse_lowir_param_list($2);
		if (!$ok)
		{
			push @errors, "declare function \@$1 has invalid parameter list: $params_or_error";
			next;
		}
		my ($metadata_ok, $metadata_or_error) =
			parse_lowir_function_metadata_suffix($4);
		push @errors, "declare function \@$1 has invalid metadata: $metadata_or_error"
			if !$metadata_ok;
		$signatures{$1} = {
			ret => $3,
			params => $params_or_error,
			arity => $metadata_ok ? $metadata_or_error->{arity} : 'fixed',
		};
	}

	my %missing_alias_targets;
	for my $name (@alias_targets)
	{
		$missing_alias_targets{$name} = 1 if !exists($all_symbols{$name});
	}
	push @errors, "object alias target(s) are not top-level symbols: " .
		join(', ', map { "\@$_" } sort keys(%missing_alias_targets))
		if scalar(keys(%missing_alias_targets)) > 0;

	my %missing_calls;
	while ($data =~ /call\s+(?:void|$type_pattern)\s+@([A-Za-z0-9_]+)\(/g)
	{
		my $name = $1;
		next if exists($function_symbols{$name});
		next if lowir_symbol_is_allowed_external($name, $source_text, 0);
		$missing_calls{$name} = 1;
	}
	push @errors, "undefined call target(s): " . join(', ', sort keys(%missing_calls))
		if scalar(keys(%missing_calls)) > 0;

	my %missing_addrs;
	while ($data =~ /addr\s+@([A-Za-z0-9_]+)/g)
	{
		my $name = $1;
		next if exists($all_symbols{$name});
		next if lowir_symbol_is_allowed_external($name, $source_text, 1);
		$missing_addrs{$name} = 1;
	}
	push @errors, "undefined address-taken symbol(s): " . join(', ', sort keys(%missing_addrs))
		if scalar(keys(%missing_addrs)) > 0;

	push @errors, validate_lowir_structure($data,
		$source_text,
		\%all_symbols,
		\%function_symbols,
		\%global_symbols,
		\%global_types,
		\%signatures);
	push @errors, validate_lowir_roles($data);

	return scalar(@errors) == 0 ? (1, undef) : (0, join('; ', @errors));
}

# The source-to-LowIR lanes (PA10-PA23 and PA8's `cppgm++ --emit-lowir`
# debug-information lanes) compare the facts the C++ lowering states by rule
# (pa8/lowir.md, "Facts The C++ Lowering States"), as they compare the
# instruction facts.  PA32's optimized lanes (mode lowir_optimized_t) leave
# them uncompared: an optimizer may add a fact it proves, or drop one.
my $LOWIR_COMPARE_SOURCE_FACTS = 1;

sub lowir_metadata_item_is_source_fact
{
	my ($key, $value) = @_;
	return 1 if $key =~ /^(?:effects|unwind|return|inline_hint|no_inline|alias|object_bytes|classes|result_classes|elision)$/;
	return 1 if $key eq 'storage' && $value eq 'readonly';
	return 0;
}

sub lowir_metadata_item_ignored_for_compare
{
	my ($key, $value) = @_;
	# Validate full metadata, but do not make early source-to-LowIR oracles depend
	# on later object/export policy or optional optimizer/provenance annotations.
	return 1 if $key eq 'role' && $value =~ /^(?:allocate_memory|free_memory|terminate|pure_virtual|dynamic_cast|bad_cast|bad_typeid|bad_array_new_length|rtti_class|rtti_si|rtti_vmi|rtti_pointer|rtti_data)$/;
	return 1 if $key =~ /^(?:linkage|binding|replaceable|object|tls_for|keep_alias|prefer_local|trivial_lifecycle|force_inline)$/;
	return 1 if $key =~ /^(?:capture|access|projection)$/;
	return 1 if $key eq 'pass' && $value eq 'decay';
	return 1 if $key eq 'storage' && $value eq 'writable';
	return !$LOWIR_COMPARE_SOURCE_FACTS && lowir_metadata_item_is_source_fact($key, $value);
}

# One bracketed group, its ignored metadata items dropped and, when it is a
# metadata group, the rest in a canonical order: the order of items inside
# a metadata group is presentation.  (A phi's incoming list is bracketed
# too; its order is left alone.)
sub canonicalize_lowir_metadata_group_for_compare
{
	my ($group) = @_;
	my @kept;
	my $metadata = 1;
	for my $item (split(/\s*,\s*/, $group))
	{
		my ($key, $value) = ($item =~ /^([A-Za-z0-9_]+)\s*=\s*([^\],\s]+)$/);
		$metadata = 0 if !defined($key);
		next if defined($key) && lowir_metadata_item_ignored_for_compare($key, $value);
		push @kept, $item;
	}
	@kept = sort @kept if $metadata;
	return scalar(@kept) == 0 ? '' : ' [' . join(', ', @kept) . ']';
}

# Every bracketed group of a text canonicalized; adjacent metadata groups of
# one entity (`[unwind=no] [binding=weak]`) read as one group.
sub canonicalize_lowir_metadata_groups_for_compare
{
	my ($text) = @_;
	$text =~ s/\][ \t]+\[(?=[A-Za-z0-9_]+\s*=)/, /g;
	$text =~ s/\s+\[([^\]]+)\]/canonicalize_lowir_metadata_group_for_compare($1)/ge;
	return $text;
}

sub canonicalize_lowir_function_locals_for_compare
{
	my ($entry) = @_;
	return $entry if $entry !~ /^(?:declare\s+)?function /;

	my (%values, %slots, %blocks, %declared_slots);
	my @declared_slot_order;
	my ($next_value, $next_slot, $next_block) = (0, 0, 0);
	my @lines = split(/\n/, $entry, -1);
	if (scalar(@lines) != 0)
	{
		while ($lines[0] =~ /%([A-Za-z0-9_]+)\s*:/g)
		{
			$values{$1} = '<value' . $next_value . '>';
			++$next_value;
		}
	}
	for my $line (@lines)
	{
		if ($line =~ /^\s*slot \$([A-Za-z0-9_]+)\s*:/)
		{
			$declared_slots{$1} = 1;
			push @declared_slot_order, $1;
		}
		if ($line =~ /^\s*block \^([A-Za-z0-9_]+):/)
		{
			$blocks{$1} = '<block' . $next_block . '>';
			++$next_block;
		}
		if ($line =~ /^\s*%([A-Za-z0-9_]+)\s*=/ && !exists($values{$1}))
		{
			$values{$1} = '<value' . $next_value . '>';
			++$next_value;
		}
	}
	for my $line (@lines)
	{
		next if $line =~ /^\s*slot \$[A-Za-z0-9_]+\s*:/;
		while ($line =~ /\$([A-Za-z0-9_]+)\b/g)
		{
			next if !exists($declared_slots{$1}) || exists($slots{$1});
			$slots{$1} = '<slot' . $next_slot . '>';
			++$next_slot;
		}
	}
	for my $name (@declared_slot_order)
	{
		next if exists($slots{$name});
		$slots{$name} = '<slot' . $next_slot . '>';
		++$next_slot;
	}

	$entry =~ s/%([A-Za-z0-9_]+)\b/
		exists($values{$1}) ? '%' . $values{$1} : '%' . $1/gex;
	$entry =~ s/\$([A-Za-z0-9_]+)\b/
		exists($slots{$1}) ? '$' . $slots{$1} : '$' . $1/gex;
	$entry =~ s/\^([A-Za-z0-9_]+)\b/
		exists($blocks{$1}) ? '^' . $blocks{$1} : '^' . $1/gex;
	# One pass over the lines: slot declarations in sorted order, blank lines
	# dropped, the layout normalized (canonicalize_lowir_layout_for_compare)
	# and literals read by value (canonicalize_lowir_literal_line_for_compare).
	@lines = split(/\n/, $entry, -1);
	my @slot_lines = sort(grep { /^\s*slot \$/ } @lines);
	my $slot_line = 0;
	my @out;
	for my $line (@lines)
	{
		if ($line =~ /^\s+(\S.*)$/)
		{
			my $body = $1;
			if (index($body, 'slot $') == 0)
			{
				$body = $slot_lines[$slot_line++] =~ s/^\s+//r;
				push @out, "  $body";
			}
			elsif (index($body, 'block ') == 0)
			{
				push @out, "  $body";
			}
			else
			{
				push @out, canonicalize_lowir_literal_line_for_compare("    $body");
			}
		}
		elsif ($line !~ /^\s*$/)
		{
			push @out, $line;
		}
	}
	return join("\n", @out);
}

# Indentation and blank lines are presentation: inside an entry, blank lines
# are dropped, `slot`, `block` and data-item lines sit at two spaces and
# instructions at four, whatever the output used.
sub canonicalize_lowir_layout_for_compare
{
	my ($entry) = @_;
	my $is_function = $entry =~ /^(?:declare\s+)?function /;
	my @out;
	for my $line (split(/\n/, $entry, -1))
	{
		next if $line =~ /^\s*$/;
		if ($line =~ /^\s+(\S.*)$/)
		{
			my $body = $1;
			$line = ($is_function && $body !~ /^(?:slot |block )/) ? "    $body" : "  $body";
		}
		push @out, canonicalize_lowir_literal_line_for_compare($line);
	}
	return join("\n", @out);
}

sub canonicalize_lowir_local_names_for_compare
{
	my ($data) = @_;
	my @entries = split_lowir_top_level_entries($data);
	@entries = map {
		/^(?:declare\s+)?function /
			? canonicalize_lowir_function_locals_for_compare($_)
			: canonicalize_lowir_layout_for_compare($_)
	} @entries;
	return join("\n\n", @entries) . (scalar(@entries) ? "\n" : '');
}

# ---- literals by value, commutative operands in either order ----
#
# The relaxed comparison absorbs two spellings a student would not expect
# to matter (pa8/lowir.md, "What The Comparison Absorbs"): a literal is
# its value, not its spelling, and the operand order of a commutative
# operation does not matter.  Everything below implements those two
# sentences and nothing more.

sub lowir_integer_type_shape
{
	my ($type) = @_;
	return () if !defined($type);
	return ('signed', $1) if $type =~ /^i(8|16|32|64|128)$/;
	return ('unsigned', $1) if $type =~ /^u(8|16|32|64|128)$/;
	return ('unsigned', 64) if $type eq 'ptr';
	return ();
}

sub lowir_floating_mantissa_bits
{
	my ($type, $suffix) = @_;
	return 24 if defined($type) && $type eq 'f32';
	return 53 if defined($type) && $type eq 'f64';
	return 64 if defined($type) && $type eq 'f80';
	return 24 if $suffix =~ /^[fF]$/;
	return 64 if $suffix =~ /^[lL]$/;
	return 53;
}

sub lowir_bigint_bit_length
{
	my ($value) = @_;
	return 0 if $value->is_zero();
	return length($value->as_bin()) - 2;
}

# Decimal text of an integer literal's value.  When the operand type is an
# integer type the value is folded to its width and signedness, so that
# `i32 -1` and `i32 4294967295` read the same.  Returns undef for a spelling
# the LowIR reader would not accept (such as `09`).
sub canonical_lowir_integer_literal
{
	my ($sign, $digits, $type) = @_;
	my ($signedness, $width) = lowir_integer_type_shape($type);
	$width = undef if defined($width) && $width < 8;
	if ($digits =~ /^(?:0|[1-9][0-9]{0,17})$/)
	{
		my $value = ($sign eq '-' ? -1 : 1) * $digits;
		return '0' if $value == 0;
		return "$value" if !defined($width);
		if ($signedness eq 'signed')
		{
			return "$value" if $width >= 64;
			return "$value" if $value >= -(2 ** ($width - 1)) && $value < 2 ** ($width - 1);
		}
		else
		{
			return "$value" if $value >= 0 && ($width >= 64 || $value < 2 ** $width);
		}
	}
	# Only spellings the fast path above does not settle need big integers;
	# the module costs 50 ms to load, so it is loaded on first use.
	require Math::BigInt;
	my $value;
	if ($digits =~ /^0[xX][0-9A-Fa-f]+$/)
	{
		$value = Math::BigInt->from_hex($digits);
	}
	elsif ($digits =~ /^0[0-7]+$/)
	{
		$value = Math::BigInt->from_oct($digits);
	}
	elsif ($digits =~ /^(?:0|[1-9][0-9]*)$/)
	{
		$value = Math::BigInt->new($digits);
	}
	else
	{
		return undef;
	}
	$value->bneg() if $sign eq '-';
	if (defined($width))
	{
		my $modulus = Math::BigInt->new(1)->blsft($width);
		$value->bmod($modulus);
		if ($signedness eq 'signed')
		{
			my $half = Math::BigInt->new(1)->blsft($width - 1);
			$value->bsub($modulus) if $value->bcmp($half) >= 0;
		}
	}
	return $value->bstr();
}

# Hexadecimal floating text (`0x1.8p+1`) of a decimal floating literal
# rounded to a mantissa of `$bits` bits, so that every spelling of one
# value of the operand's type reads the same.  Returns undef for text that
# is not a floating literal.
sub canonical_lowir_floating_literal
{
	my ($sign, $text, $bits) = @_;
	my $lower = lc($text);
	return $sign . 'inf' if $lower eq 'inf' || $lower eq 'infinity';
	return $sign . $lower if $lower eq 'nan' || $lower eq 'snan';
	return undef if $text !~ /^([0-9]*)(?:\.([0-9]*))?(?:[eE]([+-]?[0-9]+))?$/;
	my ($integer, $fraction, $exponent) = ($1, defined($2) ? $2 : '', defined($3) ? $3 : 0);
	return undef if $integer eq '' && $fraction eq '';
	my $digits = $integer . $fraction;
	$digits =~ s/^0+//;
	# A zero keeps the sign it was written with (Perl's parse would drop it).
	return $sign . '0x0p+0' if $digits eq '';
	# Perl parses a decimal to the nearest double and prints it exactly as
	# hexadecimal floating text; that settles f64, and f32 after one more
	# rounding.  Only f80 needs the exact arithmetic below.
	return sprintf('%a', 0 + "$sign$text") if $bits == 53;
	return sprintf('%a', unpack('f', pack('f', 0 + "$sign$text"))) if $bits == 24;
	require Math::BigInt;
	my $scale = length($fraction) - $exponent;
	my $numerator = Math::BigInt->new($digits);
	my $denominator = Math::BigInt->new(1);
	if ($scale > 0)
	{
		$denominator = Math::BigInt->new(10)->bpow($scale);
	}
	elsif ($scale < 0)
	{
		$numerator->bmul(Math::BigInt->new(10)->bpow(-$scale));
	}
	my $shift = $bits - (lowir_bigint_bit_length($numerator) - lowir_bigint_bit_length($denominator));
	my ($quotient, $remainder, $divisor);
	while (1)
	{
		my $scaled = $numerator->copy();
		$divisor = $denominator->copy();
		if ($shift >= 0) { $scaled->blsft($shift); } else { $divisor->blsft(-$shift); }
		($quotient, $remainder) = $scaled->bdiv($divisor);
		my $length = lowir_bigint_bit_length($quotient);
		last if $length == $bits;
		$shift += $bits - $length;
	}
	my $twice = $remainder->copy()->blsft(1);
	my $order = $twice->bcmp($divisor);
	if ($order > 0 || ($order == 0 && $quotient->is_odd()))
	{
		$quotient->binc();
		if (lowir_bigint_bit_length($quotient) > $bits)
		{
			$quotient->brsft(1);
			--$shift;
		}
	}
	my $binary_exponent = ($bits - 1) - $shift;
	my $hex_digits = int(($bits - 1 + 3) / 4);
	# Below the type's smallest normal exponent the quantum is fixed and the
	# mantissa loses bits: round again on that quantum (a subnormal value).
	my $minimum_exponent = $bits == 24 ? -126 : $bits == 53 ? -1022 : -16382;
	my $leading = '1';
	if ($binary_exponent < $minimum_exponent)
	{
		$shift = ($bits - 1) - $minimum_exponent;
		my $scaled = $numerator->copy();
		$divisor = $denominator->copy();
		if ($shift >= 0) { $scaled->blsft($shift); } else { $divisor->blsft(-$shift); }
		($quotient, $remainder) = $scaled->bdiv($divisor);
		my $twice_again = $remainder->copy()->blsft(1);
		my $order_again = $twice_again->bcmp($divisor);
		$quotient->binc() if $order_again > 0 || ($order_again == 0 && $quotient->is_odd());
		$binary_exponent = $minimum_exponent;
		if (lowir_bigint_bit_length($quotient) < $bits)
		{
			return $sign . '0x0p+0' if $quotient->is_zero();
			$leading = '0';
		}
	}
	my $mantissa = $leading eq '1'
		? $quotient->copy()->bsub(Math::BigInt->new(1)->blsft($bits - 1))
		: $quotient->copy();
	$mantissa->blsft($hex_digits * 4 - ($bits - 1));
	my $hex = substr($mantissa->as_hex(), 2);
	$hex = ('0' x ($hex_digits - length($hex))) . $hex;
	$hex =~ s/0+$//;
	return $sign . '0x' . $leading . ($hex eq '' ? '' : ".$hex") . 'p' .
		($binary_exponent >= 0 ? "+$binary_exponent" : "$binary_exponent");
}

sub canonical_lowir_literal
{
	my ($sign, $text, $type) = @_;
	my $original = $sign . $text;
	my $lower = lc($text);
	return '0' if $lower eq 'nullptr';
	if ($lower =~ /^(?:inf|infinity|nan|snan)$/)
	{
		my $canonical = canonical_lowir_floating_literal($sign, $text, 53);
		return defined($canonical) ? $canonical : $original;
	}
	my $suffix = '';
	if ($text =~ /^0[xX]/)
	{
		($text, $suffix) = ($1, $2) if $text =~ /^(0[xX][0-9A-Fa-f]+)([uUlL]+)$/;
	}
	elsif ($text =~ /^(.*[0-9.])([uUlLfF]+)$/)
	{
		($text, $suffix) = ($1, $2);
	}
	my $floating_type = defined($type) && $type =~ /^f(?:32|64|80)$/;
	my $floating_spelling = $text !~ /^0[xX]/ && ($text =~ /[.eE]/ || $suffix =~ /[fF]/);
	if ($floating_type || $floating_spelling)
	{
		return $original if scalar(lowir_integer_type_shape($type));
		# An integer spelling in a floating position (`f64 3`, `f64 0x3`) is
		# that integer's value as a floating literal.
		if ($text =~ /^0[xX][0-9A-Fa-f]+$/)
		{
			require Math::BigInt;
			$text = Math::BigInt->from_hex($text)->bstr();
		}
		my $canonical = canonical_lowir_floating_literal(
			$sign, $text, lowir_floating_mantissa_bits($type, $suffix));
		return defined($canonical) ? $canonical : $original;
	}
	my $canonical = canonical_lowir_integer_literal($sign, $text, $type);
	return defined($canonical) ? $canonical : $original;
}

# The operand type an instruction or data line gives its literals, when the
# line names one; undef for positions typed elsewhere (call arguments,
# `index` offsets, atomic orderings, spans and addends).
my $LOWIR_TYPE = qr/(?:[A-Za-z0-9_]+|obj<\d+x\d+>)/;
my $LOWIR_TYPED_INSTRUCTION = qr/^\s*(?:%[A-Za-z0-9_<>]+\s*=\s*)?(?:const|copy|phi|store(?:\s+volatile)?(?:\s+typed)?|return|unary\s+\w+|binary\s+\w+(?:\s+nowrap)?(?:\s+exact)?|cmp\s+\w+|select|atomic_\w+|va_arg)\s+($LOWIR_TYPE)(?:\s|$)/;
my $LOWIR_TYPED_CONVERT = qr/^\s*%[A-Za-z0-9_<>]+\s*=\s*convert\s+\w+\s+$LOWIR_TYPE\s+($LOWIR_TYPE)\s/;
my $LOWIR_TYPED_GLOBAL = qr/^global @[A-Za-z0-9_<>]+(?:\s+readonly)?\s*:\s*($LOWIR_TYPE)/;
my $LOWIR_TYPED_DATA_ITEM = qr/^\s+($LOWIR_TYPE)\s+-?\S+\s*$/;
# A literal candidate: a digit, a point or a named literal right after a
# separator.  Lines without one (most loads, stores of temporaries, calls,
# jumps) skip the literal pass entirely.
my $LOWIR_LITERAL_CANDIDATE = qr/[\s,(\[:]-?(?:[0-9.]|nullptr|inf|nan|snan)/i;
my $LOWIR_LITERAL_TOKEN = qr/(?<=[\s,(\[:])(-?)((?:0[xX][0-9A-Fa-f]+|(?:[0-9]+\.[0-9]*|\.[0-9]+|[0-9]+)(?:[eE][+-]?[0-9]+)?|nullptr|inf|infinity|nan|snan)[uUlLfF]*)(?=[\s,:)\]]|$)/i;

sub lowir_literal_context_type
{
	my ($line) = @_;
	return $1 if $line =~ $LOWIR_TYPED_INSTRUCTION;
	return $1 if $line =~ $LOWIR_TYPED_CONVERT;
	return $1 if $line =~ $LOWIR_TYPED_GLOBAL;
	return $1 if $line =~ $LOWIR_TYPED_DATA_ITEM;
	return undef;
}

sub canonicalize_lowir_literal_line_for_compare
{
	my ($line) = @_;
	return $line if $line !~ $LOWIR_LITERAL_CANDIDATE;
	my $type = lowir_literal_context_type($line);
	$line =~ s{$LOWIR_LITERAL_TOKEN}{canonical_lowir_literal($1, $2, $type)}ge;
	return $line;
}

sub lowir_operand_is_literal
{
	my ($operand) = @_;
	return $operand !~ /^[%\$\@^]/;
}

sub lowir_commutative_operand_order
{
	my ($left, $right) = @_;
	my $left_key = (lowir_operand_is_literal($left) ? '1' : '0') . $left;
	my $right_key = (lowir_operand_is_literal($right) ? '1' : '0') . $right;
	return $left_key le $right_key ? ($left, $right) : ($right, $left);
}

# `eq`, `ne`, `add`, `mul`, `and`, `or` and `xor` read the same with their
# operands exchanged: temporaries before literals, then by text.
my $LOWIR_COMMUTATIVE = qr/^([ \t]*%[A-Za-z0-9_<>]+[ \t]*=[ \t]*(?:binary[ \t]+(?:add|mul|and|or|xor)(?:[ \t]+nowrap)?|cmp[ \t]+(?:eq|ne))[ \t]+$LOWIR_TYPE[ \t]+)([^\s,]+),[ \t]*([^\s,]+)((?:[ \t]+\S[^\n]*)?)$/m;

sub canonicalize_lowir_commutative_operands_for_compare
{
	my ($data) = @_;
	$data =~ s{$LOWIR_COMMUTATIVE}{$1 . join(', ', lowir_commutative_operand_order($2, $3)) . $4}ge;
	return $data;
}

sub canonicalize_lowir_for_compare
{
	my ($data, $function_symbols) = @_;
	return undef if !defined($data);
	$data = canonicalize_lowir_metadata_groups_for_compare($data);
	# Object aliases are backend symbol-surface metadata, like object=... metadata.
	# Validate them above, but omit them from relaxed source-to-LowIR comparison.
	$data =~ s/^alias object [^\n]*\n?//gm;
	if (!defined($function_symbols))
	{
		my %local_function_symbols;
		my $next_function_symbol = 0;
		while ($data =~ /^(?:declare\s+)?function @([A-Za-z0-9_]+)\b/gm)
		{
			my $name = $1;
			next if exists($local_function_symbols{$name});
			$local_function_symbols{$name} = '<fn' . $next_function_symbol . '>';
			++$next_function_symbol;
		}
		$function_symbols = \%local_function_symbols;
	}
	$data =~ s/@([A-Za-z0-9_]+)\b/exists($function_symbols->{$1}) ? '@' . $function_symbols->{$1} : '@' . $1/ge;
	# Local names, layout and literal spellings are canonicalized entry by
	# entry in one pass; commutative operand order needs the local names first.
	$data = canonicalize_lowir_local_names_for_compare($data);
	$data = canonicalize_lowir_commutative_operands_for_compare($data);
	$data =~ s/[ \t]+$//mg;
	return $data;
}

sub lowir_function_symbol_records
{
	my ($data) = @_;
	my %records;
	my @ordered;
	my $type_pattern = lowir_type_pattern();

	for my $line (split(/\n/, $data))
	{
		my ($name, $params, $return_type, $metadata_suffix);
		if ($line =~ /^declare function @([A-Za-z0-9_]+)\((.*?)\) -> ($type_pattern)((?:\s+\[[^\]]+\])*)$/)
		{
			($name, $params, $return_type, $metadata_suffix) = ($1, $2, $3, $4);
		}
		elsif ($line =~ /^function @([A-Za-z0-9_]+)\((.*?)\) -> ($type_pattern)((?:\s+\[[^\]]+\])*) \{$/)
		{
			($name, $params, $return_type, $metadata_suffix) = ($1, $2, $3, $4);
		}
		else
		{
			next;
		}

		if (!exists($records{$name}))
		{
			$records{$name} = {
				name => $name,
				order => scalar(@ordered),
				signature => lowir_normalize_function_signature($params, $return_type),
				keys => {},
			};
			push @ordered, $name;
		}

		my ($ok, $metadata_or_error) =
			parse_lowir_function_metadata_suffix($metadata_suffix);
		next if !$ok;
		$records{$name}{keys}{"role:$metadata_or_error->{role}"} = 1
			if exists($metadata_or_error->{role}) && $metadata_or_error->{role} ne '';
		$records{$name}{keys}{"object:$metadata_or_error->{object}"} = 1
			if exists($metadata_or_error->{object}) && $metadata_or_error->{object} ne '';
	}

	return (\%records, \@ordered);
}

sub lowir_normalize_function_signature
{
	my ($params, $return_type) = @_;
	$params = lowir_trim($params);
	$return_type = lowir_trim($return_type);
	# Parameter names are presentation (the comparison renames them); the
	# signature that pairs two functions keeps only the types and metadata.
	$params =~ s/%[A-Za-z0-9_]+\s*:\s*//g;
	$params = canonicalize_lowir_metadata_groups_for_compare($params);
	$params =~ s/\s+/ /g;
	$return_type =~ s/\s+/ /g;
	return '(' . $params . ') -> ' . $return_type;
}

sub assign_lowir_function_symbol_pair
{
	my ($ref_map, $my_map, $ref_name, $my_name, $next_ref) = @_;
	return $$next_ref if exists($ref_map->{$ref_name}) || exists($my_map->{$my_name});
	my $placeholder = '<fn' . $$next_ref . '>';
	++$$next_ref;
	$ref_map->{$ref_name} = $placeholder;
	$my_map->{$my_name} = $placeholder;
	return $$next_ref;
}

# name -> entry text for every global and function definition; the entries
# whose bodies can refer to other symbols.
sub lowir_top_level_entry_map
{
	my ($data) = @_;
	my %map;
	for my $entry (split_lowir_top_level_entries($data))
	{
		$map{$1} = $entry if $entry =~ /^(?:function|global) \@([A-Za-z0-9_]+)\b/;
	}
	return \%map;
}

# The distinct function symbols an entry refers to, in text order, its own
# name excluded.
sub lowir_entry_function_references
{
	my ($entry, $records) = @_;
	return () if !defined($entry);
	my $body = $entry;
	$body =~ s/^((?:declare\s+)?(?:function|global) )\@[A-Za-z0-9_]+/$1/;
	my (%seen, @names);
	while ($body =~ /\@([A-Za-z0-9_]+)\b/g)
	{
		next if !exists($records->{$1}) || $seen{$1}++;
		push @names, $1;
	}
	return @names;
}

sub lowir_placeholder_index
{
	my ($placeholder) = @_;
	return $placeholder =~ /(\d+)>$/ ? $1 : 0;
}

sub lowir_function_shape_text
{
	my ($entry) = @_;
	my @lines = split(/\n/, $entry, -1);
	# Drop the function-level [..] metadata (object=, binding=, role=, ...) from
	# the header so the shape ignores volatile mangle/linkage details.
	$lines[0] =~ s/((?:\s+\[[^\]]+\])+)\s*\{\s*$/ {/ if @lines;
	my $text = join("\n", @lines);
	# Apply the same ignored-metadata policy before structural pairing that the
	# final relaxed comparison applies after pairing. Legacy source-origin facts
	# must not make two otherwise unique function shapes look different.
	$text = canonicalize_lowir_metadata_groups_for_compare($text);
	# Nothing the final comparison absorbs may tell two shapes apart either:
	# local names, slot order, layout, literal spelling and commutative
	# operand order are canonicalized before the shape is compared.
	$text = canonicalize_lowir_commutative_operands_for_compare(
		canonicalize_lowir_function_locals_for_compare($text));
	# Mask every symbol reference (the function's own name, callees, globals) so
	# two structurally identical functions match regardless of their mangled names.
	$text =~ s/\@[^\s(),]+/\@<sym>/g;
	return $text;
}

sub paired_lowir_function_symbol_maps
{
	my ($ref_data, $my_data, $ref_globals, $my_globals) = @_;
	my ($ref_records, $ref_order) = lowir_function_symbol_records($ref_data);
	my ($my_records, $my_order) = lowir_function_symbol_records($my_data);
	my (%ref_map, %my_map);
	my $next = 0;

	my %ref_by_key;
	for my $name (@$ref_order)
	{
		for my $key (keys(%{$ref_records->{$name}{keys}}))
		{
			push @{$ref_by_key{$key}}, $name;
		}
	}
	my %my_by_key;
	for my $name (@$my_order)
	{
		for my $key (keys(%{$my_records->{$name}{keys}}))
		{
			push @{$my_by_key{$key}}, $name;
		}
	}
	for my $key (sort keys(%ref_by_key))
	{
		next if !exists($my_by_key{$key});
		next if scalar(@{$ref_by_key{$key}}) != 1 || scalar(@{$my_by_key{$key}}) != 1;
		$next = assign_lowir_function_symbol_pair(\%ref_map,
		                                          \%my_map,
		                                          $ref_by_key{$key}[0],
		                                          $my_by_key{$key}[0],
		                                          \$next);
	}

	for my $name (sort keys(%$ref_records))
	{
		next if !exists($my_records->{$name});
		next if $ref_records->{$name}{signature} ne $my_records->{$name}{signature};
		$next = assign_lowir_function_symbol_pair(\%ref_map,
		                                          \%my_map,
		                                          $name,
		                                          $name,
		                                          \$next);
	}

	# Structural pass: pair any still-unpaired *defined* functions whose
	# normalized shape (signature + body with symbol names masked) is unique on
	# both sides. This lets the relaxed compare line functions up by structure
	# rather than falling back to emission order, so a function whose mangled
	# name differs (or is absent) no longer forces an order-sensitive mispairing.
	# Requiring the shape to be unique on both sides keeps this sound: the
	# correspondence is forced, and a genuinely different body has a different
	# shape and stays unpaired, so a real codegen diff is never hidden.
	# One split per side serves both the shape pass (function definitions)
	# and the reference pass below (globals too); function names never
	# collide with global names, so one map holds both.
	my $ref_entries = lowir_top_level_entry_map($ref_data);
	my $my_entries = lowir_top_level_entry_map($my_data);
	my (%ref_by_shape, %my_by_shape);
	for my $name (@$ref_order)
	{
		next if exists($ref_map{$name}) || !exists($ref_entries->{$name});
		push @{$ref_by_shape{lowir_function_shape_text($ref_entries->{$name})}}, $name;
	}
	for my $name (@$my_order)
	{
		next if exists($my_map{$name}) || !exists($my_entries->{$name});
		push @{$my_by_shape{lowir_function_shape_text($my_entries->{$name})}}, $name;
	}
	for my $shape (sort keys(%ref_by_shape))
	{
		next if !exists($my_by_shape{$shape});
		next if scalar(@{$ref_by_shape{$shape}}) != 1 || scalar(@{$my_by_shape{$shape}}) != 1;
		$next = assign_lowir_function_symbol_pair(\%ref_map,
		                                          \%my_map,
		                                          $ref_by_shape{$shape}[0],
		                                          $my_by_shape{$shape}[0],
		                                          \$next);
	}

	# Reference pass: pair what is still unpaired in the order the already
	# paired program first refers to it.  The paired globals and functions
	# are walked in placeholder order (the same order on both sides), each
	# body's k-th distinct unpaired reference on one side is paired with the
	# k-th on the other, and every new pair joins the walk.  Two helpers with
	# one shape are thereby told apart by who refers to them, not by where
	# they were emitted, so emission order and extra declarations do not move
	# the pairing.
	my @queue;
	for my $maps ([$ref_globals, $my_globals], [\%ref_map, \%my_map])
	{
		my ($ref_side, $my_side) = @$maps;
		next if !defined($ref_side) || !defined($my_side);
		my %ref_by_placeholder = reverse(%$ref_side);
		my %my_by_placeholder = reverse(%$my_side);
		for my $placeholder (sort { lowir_placeholder_index($a) <=> lowir_placeholder_index($b) }
		                     keys(%ref_by_placeholder))
		{
			next if !exists($my_by_placeholder{$placeholder});
			push @queue, [$ref_by_placeholder{$placeholder}, $my_by_placeholder{$placeholder}];
		}
	}
	while (@queue)
	{
		my ($ref_anchor, $my_anchor) = @{shift(@queue)};
		my @ref_refs = lowir_entry_function_references($ref_entries->{$ref_anchor}, $ref_records);
		my @my_refs = lowir_entry_function_references($my_entries->{$my_anchor}, $my_records);
		my $count = scalar(@ref_refs) < scalar(@my_refs) ? scalar(@ref_refs) : scalar(@my_refs);
		for (my $i = 0; $i < $count; ++$i)
		{
			next if exists($ref_map{$ref_refs[$i]}) || exists($my_map{$my_refs[$i]});
			$next = assign_lowir_function_symbol_pair(\%ref_map,
			                                          \%my_map,
			                                          $ref_refs[$i],
			                                          $my_refs[$i],
			                                          \$next);
			push @queue, [$ref_refs[$i], $my_refs[$i]];
		}
	}

	# Positional tail for what nothing paired refers to: definitions with
	# definitions and declarations with declarations, in emission order.
	my @remaining_ref = grep { !exists($ref_map{$_}) } @$ref_order;
	my @remaining_my = grep { !exists($my_map{$_}) } @$my_order;
	for my $defined (1, 0)
	{
		my @ref_kind = grep { (exists($ref_entries->{$_}) ? 1 : 0) == $defined } @remaining_ref;
		my @my_kind = grep { (exists($my_entries->{$_}) ? 1 : 0) == $defined } @remaining_my;
		my $pairs = scalar(@ref_kind) < scalar(@my_kind) ? scalar(@ref_kind) : scalar(@my_kind);
		for (my $i = 0; $i < $pairs; ++$i)
		{
			$next = assign_lowir_function_symbol_pair(\%ref_map,
			                                          \%my_map,
			                                          $ref_kind[$i],
			                                          $my_kind[$i],
			                                          \$next);
		}
	}
	for my $name (@remaining_ref)
	{
		next if exists($ref_map{$name});
		$ref_map{$name} = '<fn' . $next . '>';
		++$next;
	}
	for my $name (@remaining_my)
	{
		next if exists($my_map{$name});
		$my_map{$name} = '<fn' . $next . '>';
		++$next;
	}

	return (\%ref_map, \%my_map);
}

sub lowir_global_symbol_records
{
	my ($data) = @_;
	my %records;
	my @ordered;
	my $type_pattern = lowir_type_pattern();
	for my $entry (split_lowir_top_level_entries($data))
	{
		my ($name, $metadata_suffix, $shape);
		if ($entry =~ /^declare global @([A-Za-z0-9_]+)(?:\s*:\s*$type_pattern)?((?:\s+\[[^\]]+\])*)$/)
		{
			($name, $metadata_suffix) = ($1, $2);
			$shape = $entry;
		}
		elsif ($entry =~ /^global @([A-Za-z0-9_]+)(?:\s+readonly)?(?:\s*:\s*$type_pattern)?((?:\s+\[[^\]]+\])*)\s*=/)
		{
			($name, $metadata_suffix) = ($1, $2);
			$shape = $entry;
		}
		else
		{
			next;
		}
		$shape =~ s/^((?:declare )?global \@[^\s:=]+)(?:\s+readonly)?(?:\s*:\s*$type_pattern)?(?:\s+\[[^\]]+\])*/$1/;
		$shape =~ s/\@[^\s(),]+/\@<sym>/g;
		my ($ok, $metadata_or_error) =
			parse_lowir_symbol_metadata_suffix($metadata_suffix);
		my %keys;
		if ($ok)
		{
			$keys{"role:$metadata_or_error->{role}"} = 1
				if exists($metadata_or_error->{role}) &&
					$metadata_or_error->{role} ne '';
			$keys{"object:$metadata_or_error->{object}"} = 1
				if exists($metadata_or_error->{object}) &&
					$metadata_or_error->{object} ne '';
		}
		$records{$name} = {
			name => $name,
			order => scalar(@ordered),
			shape => $shape,
			keys => \%keys,
		};
		push @ordered, $name;
	}
	return (\%records, \@ordered);
}

sub assign_lowir_global_symbol_pair
{
	my ($ref_map, $my_map, $ref_name, $my_name, $next_ref) = @_;
	return if exists($ref_map->{$ref_name}) || exists($my_map->{$my_name});
	my $placeholder = '<global' . $$next_ref . '>';
	++$$next_ref;
	$ref_map->{$ref_name} = $placeholder;
	$my_map->{$my_name} = $placeholder;
}

sub paired_lowir_global_symbol_maps
{
	my ($ref_data, $my_data) = @_;
	my ($ref_records, $ref_order) = lowir_global_symbol_records($ref_data);
	my ($my_records, $my_order) = lowir_global_symbol_records($my_data);
	my (%ref_map, %my_map);
	my $next = 0;

	my (%ref_by_key, %my_by_key);
	for my $name (@$ref_order)
	{
		for my $key (keys(%{$ref_records->{$name}{keys}}))
		{
			push @{$ref_by_key{$key}}, $name;
		}
	}
	for my $name (@$my_order)
	{
		for my $key (keys(%{$my_records->{$name}{keys}}))
		{
			push @{$my_by_key{$key}}, $name;
		}
	}
	for my $key (sort keys(%ref_by_key))
	{
		next if !exists($my_by_key{$key});
		next if scalar(@{$ref_by_key{$key}}) != 1 ||
			scalar(@{$my_by_key{$key}}) != 1;
		assign_lowir_global_symbol_pair(\%ref_map, \%my_map,
			$ref_by_key{$key}[0], $my_by_key{$key}[0], \$next);
	}
	for my $name (sort keys(%$ref_records))
	{
		next if !exists($my_records->{$name});
		assign_lowir_global_symbol_pair(
			\%ref_map, \%my_map, $name, $name, \$next);
	}

	my (%ref_by_shape, %my_by_shape);
	for my $name (@$ref_order)
	{
		next if exists($ref_map{$name});
		push @{$ref_by_shape{$ref_records->{$name}{shape}}}, $name;
	}
	for my $name (@$my_order)
	{
		next if exists($my_map{$name});
		push @{$my_by_shape{$my_records->{$name}{shape}}}, $name;
	}
	for my $shape (sort keys(%ref_by_shape))
	{
		next if !exists($my_by_shape{$shape});
		next if scalar(@{$ref_by_shape{$shape}}) != 1 ||
			scalar(@{$my_by_shape{$shape}}) != 1;
		assign_lowir_global_symbol_pair(\%ref_map, \%my_map,
			$ref_by_shape{$shape}[0], $my_by_shape{$shape}[0], \$next);
	}

	my @remaining_ref = grep { !exists($ref_map{$_}) } @$ref_order;
	my @remaining_my = grep { !exists($my_map{$_}) } @$my_order;
	my $pairs = scalar(@remaining_ref) < scalar(@remaining_my) ?
		scalar(@remaining_ref) : scalar(@remaining_my);
	for (my $i = 0; $i < $pairs; ++$i)
	{
		assign_lowir_global_symbol_pair(\%ref_map, \%my_map,
			$remaining_ref[$i], $remaining_my[$i], \$next);
	}
	for my $name (@remaining_ref)
	{
		next if exists($ref_map{$name});
		$ref_map{$name} = '<global' . $next++ . '>';
	}
	for my $name (@remaining_my)
	{
		next if exists($my_map{$name});
		$my_map{$name} = '<global' . $next++ . '>';
	}
	return (\%ref_map, \%my_map);
}

sub lowir_relaxed_top_level_rank
{
	my ($entry) = @_;
	return 0 if $entry =~ /^declare global /;
	return 1 if $entry =~ /^declare function /;
	return 2 if $entry =~ /^global /;
	return 3 if $entry =~ /^function /;
	return 4 if $entry =~ /^alias object /;
	return 5;
}

sub split_lowir_top_level_entries
{
	my ($data) = @_;
	my @lines = split(/\n/, $data);
	my @entries;
	my @current;
	my $mode = '';
	for (my $i = 0; $i < scalar(@lines); ++$i)
	{
		my $line = $lines[$i];
		next if $line =~ /^\s*$/ && scalar(@current) == 0;
		if ($mode eq '')
		{
			if ($line =~ /^(?:declare global|declare function) /)
			{
				push @entries, $line;
				next;
			}
			if ($line =~ /^alias object /)
			{
				push @entries, $line;
				next;
			}
			if ($line =~ /^global .*=\s*\{$/)
			{
				@current = ($line);
				$mode = 'global';
				next;
			}
			if ($line =~ /^global /)
			{
				push @entries, $line;
				next;
			}
			if ($line =~ /^function /)
			{
				@current = ($line);
				$mode = 'function';
				next;
			}
			push @entries, $line;
			next;
		}

		push @current, $line;
		if ($line eq '}')
		{
			push @entries, join("\n", @current);
			@current = ();
			$mode = '';
		}
	}
	push @entries, join("\n", @current) if scalar(@current) != 0;
	return @entries;
}

sub canonicalize_lowir_top_level_order_for_compare
{
	my ($data) = @_;
	my @entries = split_lowir_top_level_entries($data);
	@entries = sort {
		my $rank_a = lowir_relaxed_top_level_rank($a);
		my $rank_b = lowir_relaxed_top_level_rank($b);
		$rank_a <=> $rank_b || $a cmp $b
	} @entries;
	return join("\n\n", @entries) . (scalar(@entries) ? "\n" : "");
}

sub canonicalize_lowir_trivial_lifecycle_for_compare
{
	my ($data) = @_;
	my @entries = split_lowir_top_level_entries($data);
	my %trivial;
	for my $entry (@entries)
	{
		if ($entry =~ /^(?:declare\s+)?function \@([A-Za-z0-9_]+)\([^\n]*\)\s+->\s+[^\n]*\[[^\]\n]*\btrivial_lifecycle=yes\b[^\]\n]*\]/)
		{
			$trivial{$1} = 1;
		}
	}
	return $data if scalar(keys(%trivial)) == 0;

	# A semantically trivial lifecycle call has no observable effect.  Normalize
	# direct void calls away so an early source fixture may retain the explicit
	# call/body while a later object-capable lowering omits it.
	for my $entry (@entries)
	{
		for my $name (keys(%trivial))
		{
			$entry =~ s/^[ \t]*call[ \t]+void[ \t]+\@\Q$name\E\([^\n]*\)(?:[ \t]+!dbg\([^\n]*\))?[ \t]*\n?//mg;
		}
	}

	my %referenced;
	for my $entry (@entries)
	{
		next if $entry =~ /^alias object /;
		my $without_header = $entry;
		$without_header =~ s/^(?:declare\s+)?function \@([A-Za-z0-9_]+)[^\n]*\n?//;
		while ($without_header =~ /\@([A-Za-z0-9_]+)\b/g)
		{
			$referenced{$1} = 1 if exists($trivial{$1});
		}
	}
	@entries = grep {
		!(/^((?:declare\s+)?function) \@([A-Za-z0-9_]+)\b/ &&
		  exists($trivial{$2}) && !exists($referenced{$2}))
	} @entries;
	return join("\n\n", @entries) . (scalar(@entries) ? "\n" : "");
}

sub canonicalize_lowir_unreachable_role_for_compare
{
	my ($data) = @_;
	my @entries = split_lowir_top_level_entries($data);
	my %unreachable;
	for my $entry (@entries)
	{
		if ($entry =~ /^(?:declare\s+)?function \@([A-Za-z0-9_]+)\([^\n]*\)\s+->\s+[^\n]*\[[^\]\n]*\brole=unreachable\b[^\]\n]*\]/)
		{
			$unreachable{$1} = 1;
		}
	}
	return $data if scalar(keys(%unreachable)) == 0;

	my @retained;
	for my $entry (@entries)
	{
		if ($entry =~ /^(?:declare\s+)?function \@([A-Za-z0-9_]+)\b/ &&
			exists($unreachable{$1}))
		{
			next;
		}
		my @lines = split(/\n/, $entry, -1);
		my @normalized;
		my $after_unreachable = 0;
		for my $line (@lines)
		{
			if ($after_unreachable)
			{
				if ($line =~ /^\s+block \^/ || $line eq '}')
				{
					$after_unreachable = 0;
					push @normalized, $line;
				}
				next;
			}
			my $matched = 0;
			for my $name (keys(%unreachable))
			{
				if ($line =~ /^(\s*)call\s+void\s+\@\Q$name\E\(\)\s*(?:(!dbg\([^\n]*\)))?\s*$/)
				{
					push @normalized, $1 . 'unreachable' .
						(defined($2) ? " $2" : '');
					$after_unreachable = 1;
					$matched = 1;
					last;
				}
			}
			push @normalized, $line if !$matched;
		}
		push @retained, join("\n", @normalized);
	}
	return join("\n\n", @retained) . (scalar(@retained) ? "\n" : "");
}

sub canonicalize_lowir_decay_for_compare
{
	my ($data) = @_;
	my @entries = split_lowir_top_level_entries($data);
	for my $entry (@entries)
	{
		next if $entry !~ /^(?:declare\s+)?function /;
		my @lines = split(/\n/, $entry, -1);
		my %aliases;
		my @retained;
		for my $line (@lines)
		{
			if ($line =~ /^\s*%([A-Za-z0-9_]+)\s*=\s*unary\s+decay\s+ptr\s+(\S+)(?:\s+!dbg\([^\n]*\))?\s*$/)
			{
				$aliases{$1} = $2;
				next;
			}
			push @retained, $line;
		}
		for(my $iteration = 0;
		    $iteration < scalar(keys(%aliases)); ++$iteration)
		{
			my $changed = 0;
			for my $name (keys(%aliases))
			{
				my $operand = $aliases{$name};
				if ($operand =~ /^%([A-Za-z0-9_]+)$/ &&
				    exists($aliases{$1}))
				{
					$aliases{$name} = $aliases{$1};
					$changed = 1;
				}
			}
			last if !$changed;
		}
		for my $line (@retained)
		{
			for my $name (keys(%aliases))
			{
				$line =~ s/%\Q$name\E\b/$aliases{$name}/ge;
			}
		}
		$entry = join("\n", @retained);
	}
	return join("\n\n", @entries) . (scalar(@entries) ? "\n" : "");
}

sub remove_lowir_metadata_item
{
	my ($entry, $key, $value) = @_;
	$entry =~ s/(\[[^\]\n]*)\b\Q$key\E\s*=\s*\Q$value\E\s*,\s*/$1/g;
	$entry =~ s/(\[[^\]\n]*),\s*\Q$key\E\s*=\s*\Q$value\E\b/$1/g;
	$entry =~ s/\s*\[\s*\Q$key\E\s*=\s*\Q$value\E\s*\]//g;
	return $entry;
}

sub canonicalize_lowir_generated_root_annotations_for_compare
{
	my ($ref_data, $my_data, $ref_symbols, $my_symbols) = @_;
	my %required_roots;
	for my $entry (split_lowir_top_level_entries($ref_data))
	{
		next if $entry !~ /^(?:declare\s+)?function \@([A-Za-z0-9_]+)[^\n]*\[[^\]\n]*\bobject_root=yes\b[^\]\n]*\]/;
		my $mapped = exists($ref_symbols->{$1}) ? $ref_symbols->{$1} : $1;
		$required_roots{$mapped} = 1;
	}

	my @entries = split_lowir_top_level_entries($my_data);
	for my $entry (@entries)
	{
		next if $entry !~ /^(?:declare\s+)?function \@([A-Za-z0-9_]+)\b/;
		my $mapped = exists($my_symbols->{$1}) ? $my_symbols->{$1} : $1;
		next if exists($required_roots{$mapped});
		$entry = remove_lowir_metadata_item($entry, 'object_root', 'yes');
	}
	return join("\n\n", @entries) . (scalar(@entries) ? "\n" : "");
}

sub canonicalize_lowir_pair_for_compare
{
	my ($ref_data, $my_data) = @_;
	$ref_data =~ s/\bpass\s*=\s*reference\b/pass=by_address/g;
	$ref_data = canonicalize_lowir_tls_accesses_for_compare($ref_data);
	$my_data = canonicalize_lowir_tls_accesses_for_compare($my_data);
	($ref_data, $my_data) = canonicalize_lowir_pair_nothrow_resume_for_compare(
		$ref_data, $my_data);
	$ref_data = canonicalize_lowir_reference_liveness_for_compare(
		$ref_data, $my_data);
	$ref_data = canonicalize_lowir_trivial_lifecycle_for_compare($ref_data);
	$ref_data = canonicalize_lowir_unreachable_role_for_compare($ref_data);
	$ref_data = canonicalize_lowir_decay_for_compare($ref_data);
	# Globals first: their initializers anchor the reference pass that pairs
	# the functions nothing else distinguishes.
	my ($ref_globals, $my_globals) =
		paired_lowir_global_symbol_maps($ref_data, $my_data);
	my ($ref_functions, $my_functions) =
		paired_lowir_function_symbol_maps($ref_data, $my_data, $ref_globals, $my_globals);
	my %ref_symbols = (%$ref_functions, %$ref_globals);
	my %my_symbols = (%$my_functions, %$my_globals);
	$my_data = canonicalize_lowir_generated_root_annotations_for_compare(
		$ref_data, $my_data, \%ref_symbols, \%my_symbols);
	my $ref_compare_data =
		canonicalize_lowir_top_level_order_for_compare(
			canonicalize_lowir_for_compare($ref_data, \%ref_symbols));
	my $my_compare_data =
		canonicalize_lowir_top_level_order_for_compare(
			canonicalize_lowir_for_compare($my_data, \%my_symbols));
	return ($ref_compare_data, $my_compare_data);
}

sub canonicalize_lowir_tls_accesses_for_compare
{
	my ($data) = @_;
	my (%tls_for, %tls_globals);
	for my $entry (split_lowir_top_level_entries($data))
	{
		next if $entry !~ /^(?:declare\s+)?function \@([A-Za-z0-9_]+)[^\n]*\[[^\]\n]*\btls_for=\@([A-Za-z0-9_]+)\b[^\]\n]*\]/;
		$tls_for{$1} = $2;
		$tls_globals{$2} = 1;
	}
	for my $wrapper (keys(%tls_for))
	{
		my $global = $tls_for{$wrapper};
		$data =~ s/(^\s*%[A-Za-z0-9_]+\s*=\s*)call\s+ptr\s+\@\Q$wrapper\E\(\)/$1addr \@$global/gm;
	}
	for my $global (keys(%tls_globals))
	{
		my $address = '%__compare_tls_address_' . $global;
		$data =~ s/^(\s*)(%[A-Za-z0-9_]+\s*=\s*load\s+(?:volatile\s+)?(?:typed\s+)?([^\s]+)\s+)\@\Q$global\E\s*$/
			$1 . $address . ' = addr @' . $global . "\n" .
			$1 . $2 . $address/gem;
	}
	return $data;
}

sub lowir_function_entry_compare_key
{
	my ($entry) = @_;
	my $object = lowir_function_entry_object($entry);
	return 'object:' . $object if defined($object);
	my $name = lowir_function_entry_name($entry);
	return defined($name) ? 'name:' . $name : undef;
}

sub canonicalize_lowir_nothrow_resume_entries_for_compare
{
	my ($data, $nothrow) = @_;
	my @entries = split_lowir_top_level_entries($data);
	for my $entry (@entries)
	{
		my $key = lowir_function_entry_compare_key($entry);
		next if !defined($key) || !exists($nothrow->{$key});
		$entry =~ s/^[ \t]*resume[ \t]*\n?//mg;
	}
	return join("\n\n", @entries) . (scalar(@entries) ? "\n" : "");
}

sub canonicalize_lowir_pair_nothrow_resume_for_compare
{
	my ($ref_data, $my_data) = @_;
	my %nothrow;
	for my $data ($ref_data, $my_data)
	{
		for my $entry (split_lowir_top_level_entries($data))
		{
			next if $entry !~ /^function [^\n]*\[[^\]\n]*\bunwind=no\b[^\]\n]*\]/;
			my $key = lowir_function_entry_compare_key($entry);
			$nothrow{$key} = 1 if defined($key);
		}
	}
	return (
		canonicalize_lowir_nothrow_resume_entries_for_compare(
			$ref_data, \%nothrow),
		canonicalize_lowir_nothrow_resume_entries_for_compare(
			$my_data, \%nothrow));
}

sub lowir_function_entry_name
{
	my ($entry) = @_;
	return $1 if $entry =~ /^(?:declare\s+)?function \@([A-Za-z0-9_]+)\b/;
	return undef;
}

sub lowir_function_entry_object
{
	my ($entry) = @_;
	return $1 if $entry =~ /^(?:declare\s+)?function \@[^\n]*\[[^\]\n]*\bobject=([^,\]\s]+)[^\]\n]*\]/;
	return undef;
}

sub lowir_function_referenced_outside_entry
{
	my ($name, $entries, $entry_index) = @_;
	for (my $i = 0; $i < @$entries; ++$i)
	{
		next if $i == $entry_index || $entries->[$i] =~ /^alias object /;
		return 1 if $entries->[$i] =~ /\@\Q$name\E\b/;
	}
	return 0;
}

sub canonicalize_lowir_reference_liveness_for_compare
{
	my ($ref_data, $my_data) = @_;
	my (%my_names, %my_objects);
	for my $entry (split_lowir_top_level_entries($my_data))
	{
		my $name = lowir_function_entry_name($entry);
		next if !defined($name);
		$my_names{$name} = 1;
		my $object = lowir_function_entry_object($entry);
		$my_objects{$object} = 1 if defined($object);
	}

	my @entries = split_lowir_top_level_entries($ref_data);
	my @kept;
	for (my $i = 0; $i < @entries; ++$i)
	{
		my $entry = $entries[$i];
		my $name = lowir_function_entry_name($entry);
		if (!defined($name))
		{
			push @kept, $entry;
			next;
		}
		my $object = lowir_function_entry_object($entry);
		my $present = exists($my_names{$name}) ||
			(defined($object) && exists($my_objects{$object}));
		if (!$present &&
			!lowir_function_referenced_outside_entry($name, \@entries, $i))
		{
			my $declaration = $entry =~ /^declare\s+function /;
			my $discardable_definition =
				$entry =~ /^function / &&
				$entry =~ /\[[^\]\n]*\bbinding=(?:weak|internal)\b[^\]\n]*\]/ &&
				$entry !~ /\[[^\]\n]*\bobject_root=yes\b[^\]\n]*\]/ &&
				$entry !~ /\[[^\]\n]*\brole=(?:entry|init|fini)\b[^\]\n]*\]/;
			next if $declaration || $discardable_definition;
		}
		push @kept, $entry;
	}
	return join("\n\n", @kept) . (scalar(@kept) ? "\n" : "");
}

sub lowir_compare_entry_identity
{
	my ($entry) = @_;
	return "function:$1"
		if $entry =~ /^(?:declare\s+)?function \@([^\s(]+)/;
	return "global:$1"
		if $entry =~ /^(?:declare\s+)?global \@([^\s:=]+)/;
	return "exact:$entry";
}

sub lowir_nonempty_trimmed_lines
{
	my ($entry) = @_;
	my @result;
	for my $line (split(/\n/, $entry))
	{
		$line =~ s/^\s+//;
		$line =~ s/\s+$//;
		push @result, $line if $line ne '';
	}
	return @result;
}

sub lowir_function_signature_line
{
	my ($line) = @_;
	$line =~ s/^declare\s+//;
	$line =~ s/\s*\{\s*$//;
	return $line;
}

sub lowir_projection_local_kind
{
	my ($token) = @_;
	return 'value' if $token =~ /^%</;
	return 'slot' if $token =~ /^\$/;
	return 'block' if $token =~ /^\^/;
	return '';
}

sub lowir_projection_line_matches
{
	my ($ref_line, $my_line, $mapping, $reverse) = @_;
	my @ref_parts = split(/((?:%<value\d+>|\$<slot\d+>|\^<block\d+>))/, $ref_line);
	my @my_parts = split(/((?:%<value\d+>|\$<slot\d+>|\^<block\d+>))/, $my_line);
	return 0 if scalar(@ref_parts) != scalar(@my_parts);
	my (%candidate_mapping, %candidate_reverse);
	for my $key (keys(%$mapping))
	{
		$candidate_mapping{$key} = $mapping->{$key};
		$candidate_reverse{$mapping->{$key}} = $key;
	}
	for (my $i = 0; $i < @ref_parts; ++$i)
	{
		my $ref_kind = lowir_projection_local_kind($ref_parts[$i]);
		my $my_kind = lowir_projection_local_kind($my_parts[$i]);
		if ($ref_kind eq '' || $my_kind eq '')
		{
			return 0 if $ref_kind ne $my_kind ||
				$ref_parts[$i] ne $my_parts[$i];
			next;
		}
		return 0 if $ref_kind ne $my_kind;
		if (exists($candidate_mapping{$ref_parts[$i]}))
		{
			return 0 if $candidate_mapping{$ref_parts[$i]} ne $my_parts[$i];
		}
		elsif (exists($candidate_reverse{$my_parts[$i]}))
		{
			return 0;
		}
		else
		{
			$candidate_mapping{$ref_parts[$i]} = $my_parts[$i];
			$candidate_reverse{$my_parts[$i]} = $ref_parts[$i];
		}
	}
	%$mapping = %candidate_mapping;
	%$reverse = %candidate_reverse;
	return 1;
}

# Whether the reference entry's lines appear, in order and with a
# consistent renaming, in the generated entry.  Every generated line the
# match skips must satisfy $inserted_ok (a regex, or a code reference
# applied to the line); without it no line may be skipped, so the entries
# must correspond line for line.
sub lowir_inserted_line_ok
{
	my ($inserted_ok, $line) = @_;
	return 0 if !defined($inserted_ok);
	return $inserted_ok->($line) ? 1 : 0 if ref($inserted_ok) eq 'CODE';
	return $line =~ $inserted_ok ? 1 : 0;
}

sub lowir_reference_entry_is_projection
{
	my ($ref_entry, $my_entry, $inserted_ok) = @_;
	return $ref_entry eq $my_entry if $ref_entry !~ /^(?:declare\s+)?function /;
	my @ref_lines = lowir_nonempty_trimmed_lines($ref_entry);
	my @my_lines = lowir_nonempty_trimmed_lines($my_entry);
	return 0 if scalar(@ref_lines) == 0 || scalar(@my_lines) == 0;
	my $ref_declaration = $ref_lines[0] =~ /^declare\s+function /;
	my $my_declaration = $my_lines[0] =~ /^declare\s+function /;
	my (%mapping, %reverse);
	return 0 if !lowir_projection_line_matches(
		lowir_function_signature_line($ref_lines[0]),
		lowir_function_signature_line($my_lines[0]),
		\%mapping, \%reverse);
	return 1 if $ref_declaration;
	return 0 if $my_declaration;

	my $cursor = 1;
	for (my $i = 1; $i < @ref_lines; ++$i)
	{
		while ($cursor < @my_lines)
		{
			my (%candidate_mapping, %candidate_reverse);
			%candidate_mapping = %mapping;
			%candidate_reverse = %reverse;
			if (lowir_projection_line_matches(
				$ref_lines[$i], $my_lines[$cursor],
				\%candidate_mapping, \%candidate_reverse))
			{
				%mapping = %candidate_mapping;
				%reverse = %candidate_reverse;
				last;
			}
			return 0 if !lowir_inserted_line_ok($inserted_ok, $my_lines[$cursor]);
			++$cursor;
		}
		return 0 if $cursor == @my_lines;
		++$cursor;
	}
	for (; $cursor < @my_lines; ++$cursor)
	{
		return 0 if !lowir_inserted_line_ok($inserted_ok, $my_lines[$cursor]);
	}
	return 1;
}

# A finalizer's destructor actions: each void call whose one argument is a
# global's address or a constant offset into it (an array element), as
# [global, offset, callee]. A dynamic offset, as a destruction loop computes,
# is recorded as '*'.
sub lowir_finalizer_actions
{
	my ($entry) = @_;
	my (%address, %constant, @actions);
	for my $line (split(/\n/, $entry))
	{
		if ($line =~ /^\s*(%<value\d+>)\s*=\s*addr\s+(\@<global\d+>)\s*$/)
		{
			$address{$1} = [$2, 0];
		}
		elsif ($line =~ /^\s*(%<value\d+>)\s*=\s*binary\s+(mul|add)\s+\S+\s+(-?\d+),\s*(-?\d+)\s*$/)
		{
			$constant{$1} = $2 eq 'mul' ? $3 * $4 : $3 + $4;
		}
		elsif ($line =~ /^\s*(%<value\d+>)\s*=\s*index\s+\S+(?:\s+\[[^\]]*\])?\s+(%<value\d+>),\s*(\S+)\s*$/ &&
			exists($address{$2}))
		{
			my ($value, $base, $operand) = ($1, $2, $3);
			my ($global, $offset) = @{$address{$base}};
			my $step = exists($constant{$operand}) ? $constant{$operand} : $operand;
			$address{$value} = [$global,
				$offset ne '*' && $step =~ /^-?\d+$/ ? $offset + $step : '*'];
		}
		elsif ($line =~ /^\s*call\s+void\s+(\@<fn\d+>)\((%<value\d+>)\)\s*$/ &&
			exists($address{$2}))
		{
			push @actions, [$address{$2}[0], $address{$2}[1], $1];
		}
	}
	return @actions;
}

# A static finalizer may instead be a registered destructor function: the
# candidate must destroy the same objects with the same destructors, the
# same elements of each in the same order (a destruction loop stands for
# any count), and keep the groups in the reference's destruction order.
sub lowir_static_finalizer_action_is_projection
{
	my ($ref_entry, $my_entry) = @_;
	return 0 if $ref_entry !~ /^function [^\n]*\[[^\]\n]*\brole=fini\b[^\]\n]*\]/;
	return 0 if $my_entry !~ /^function /;
	my (%expected, %actual, @expected_order, @actual_order);
	for my $action (lowir_finalizer_actions($ref_entry))
	{
		my $key = "$action->[0] $action->[2]";
		push @expected_order, $key if !exists($expected{$key});
		push @{$expected{$key}}, $action->[1];
	}
	return 0 if scalar(@expected_order) == 0;
	for my $action (lowir_finalizer_actions($my_entry))
	{
		my $key = "$action->[0] $action->[2]";
		push @actual_order, $key if !exists($actual{$key});
		push @{$actual{$key}}, $action->[1];
	}
	for my $key (@expected_order)
	{
		return 0 if !exists($actual{$key});
		next if grep { $_ eq '*' } (@{$actual{$key}}, @{$expected{$key}});
		return 0 if join(',', @{$actual{$key}}) ne join(',', @{$expected{$key}});
	}
	my @order = grep { exists($expected{$_}) } @actual_order;
	return 0 if join(';', @order) ne join(';', @expected_order);
	return $my_entry =~ /return void/;
}

# When the canonical texts differ, the generated output still matches the
# reference in exactly these cases, each on pa8/lowir.md's list of what the
# comparison absorbs:
#
# - a declaration the reference makes is a definition in the output (the
#   entity was emitted; its body is not compared), and a declaration the
#   output makes beyond the reference's is ignored;
# - where static destruction happens: the reference's `fini` function may
#   instead be a function registered to run at exit.  Then the fini entry
#   is matched by its destructor actions against the registered function
#   (lowir_static_finalizer_action_is_projection), the function that
#   registers it differs from the reference's only by the inserted
#   registration calls (a call of a function the output declares beyond
#   the reference, passing the registered function), and nothing else.
#
# Every other entry must equal the reference's line for line.  An earlier
# form of this fallback accepted any output in which the reference's lines
# appeared in order, which passed extra instructions and extra definitions
# while rejecting a leaner output; the comparison is symmetric again.
sub lowir_reference_matches_after_canonicalization
{
	my ($ref_data, $my_data) = @_;
	my @ref_entries = split_lowir_top_level_entries($ref_data);
	my @my_entries = split_lowir_top_level_entries($my_data);
	my %ref_identity = map { lowir_compare_entry_identity($_) => 1 } @ref_entries;
	my %my_entries;
	for my $entry (@my_entries)
	{
		push @{$my_entries{lowir_compare_entry_identity($entry)}}, $entry;
	}

	# The registered-finalizer form: the reference's one fini function
	# against a generated definition that is not one of the reference's
	# entries and whose destructor actions are the fini's.
	my @ref_fini = grep { /^function / && /\[[^\]]*\brole=fini\b/ } @ref_entries;
	my %ref_text = map { $_ => 1 } @ref_entries;
	my ($registered, $registered_name);
	if (@ref_fini == 1)
	{
		($registered) = grep {
			/^function / && !$ref_text{$_} &&
			lowir_static_finalizer_action_is_projection($ref_fini[0], $_)
		} @my_entries;
		($registered_name) = $registered =~ /^function\s+(\@[^\s(]+)/ if defined($registered);
	}
	my %extra_declaration = map { /^declare\s+function\s+\@([^\s(]+)/ ? ($1 => 1) : () }
		grep { /^declare\s+function / && !$ref_identity{lowir_compare_entry_identity($_)} } @my_entries;
	my $inserted_ok = sub {
		my ($line) = @_;
		return 0 if !defined($registered_name);
		return $line =~ /^\s*(?:%\S+\s*=\s*)?call\s+\S+\s+\@([^\s(]+)\(\Q$registered_name\E\)\s*$/ &&
			exists($extra_declaration{$1});
	};

	for my $ref_entry (@ref_entries)
	{
		my $identity = lowir_compare_entry_identity($ref_entry);
		my $matched = 0;
		if (defined($registered) && $ref_entry eq $ref_fini[0])
		{
			for my $candidate_identity (keys(%my_entries))
			{
				my $list = $my_entries{$candidate_identity};
				for (my $i = 0; $i < @$list; ++$i)
				{
					next if $list->[$i] ne $registered;
					splice(@$list, $i, 1);
					$matched = 1;
					last;
				}
				last if $matched;
			}
		}
		elsif (exists($my_entries{$identity}))
		{
			my $list = $my_entries{$identity};
			for (my $i = 0; $i < @$list; ++$i)
			{
				next if $ref_entry ne $list->[$i] &&
					!lowir_reference_entry_is_projection($ref_entry, $list->[$i], $inserted_ok);
				splice(@$list, $i, 1);
				$matched = 1;
				last;
			}
		}
		return 0 if !$matched;
	}
	# What the output has beyond the reference may only be declarations.
	for my $leftover (map { @{$_} } values(%my_entries))
	{
		return 0 if $leftover !~ /^declare\s+/;
	}
	return 1;
}

sub lowir_compare_artifact_paths
{
	my ($my) = @_;
	return (
		"$my.lowir.ref.compare",
		"$my.lowir.my.compare",
		"$my.lowir.compare.diff",
	);
}

sub clear_lowir_compare_failure_artifacts
{
	my ($my) = @_;
	my ($ref_compare, $my_compare, $diff_path) =
		lowir_compare_artifact_paths($my);
	unlink($ref_compare);
	unlink($my_compare);
	unlink($diff_path);
}

sub write_lowir_compare_failure_artifacts
{
	my ($my, $ref_compare_data, $my_compare_data) = @_;
	my ($ref_compare, $my_compare, $diff_path) =
		lowir_compare_artifact_paths($my);
	putrawdata($ref_compare, $ref_compare_data);
	putrawdata($my_compare, $my_compare_data);

	my $diff_data = '';
	if (open(my $diff_fh, '-|', 'diff', '-u', $ref_compare, $my_compare))
	{
		local $/;
		$diff_data = <$diff_fh>;
		close($diff_fh);
	}
	putrawdata($diff_path, defined($diff_data) ? $diff_data : '');
	return ($ref_compare, $my_compare, $diff_path);
}

sub lowir_compare_failure_hint
{
	my ($ref_compare, $my_compare, $diff_path) = @_;
	return "To inspect the relaxed LowIR comparison:\n\n    \$ diff " .
		rooted_path($ref_compare) . " " . rooted_path($my_compare) .
		"\n    \$ cat " . rooted_path($diff_path) . "\n\n" .
		"These files contain the canonicalized LowIR used for relaxed comparison.\n\n";
}

sub compare_lowir_text
{
	my ($ref_suffix, $my_suffix, $testbase) = @_;
	my $ref = "$testbase.$ref_suffix";
	my $my = "$testbase.$my_suffix";
	my $source_file = "$testbase.t";
	clear_lowir_compare_failure_artifacts($my);
	my $ref_status = getdata("$ref.exit_status");
	my $my_status = getdata("$my.exit_status");
	my @missing_status = ();
	push @missing_status, "$ref.exit_status" if !defined($ref_status);
	push @missing_status, "$my.exit_status" if !defined($my_status);
	return (0, "ERROR: missing exit status output (" . join(', ', @missing_status) . ")")
		if scalar(@missing_status) != 0;
	return (0, status_mismatch_message("tool", $ref_status, $my_status))
		if $ref_status ne $my_status;
	return (1, undef) if $ref_status ne 'EXIT_SUCCESS';

	my $expect_path = expectation_sidecar($testbase);
	if (defined($expect_path))
	{
		my $my_data = getdata($my);
		return (0, "ERROR: missing output file ($my)") if !defined($my_data);
		my ($my_valid, $my_error) = validate_lowir_text(
			$my_data, $source_file, { strict_presentation_order => 0 });
		return (0, "ERROR: generated LowIR failed sanity validation: $my_error") if !$my_valid;
		my ($lowers, $lower_message, $lower_hint) = check_lowir_lowers($my, $source_file);
		return ($lowers, $lower_message, $lower_hint) if !$lowers;
		my ($behaves, $behave_message, $behave_hint) = check_source_behavior($my, $source_file);
		return ($behaves, $behave_message, $behave_hint) if !$behaves;
		my ($runs, $run_message, $run_hint) = check_lowir_behavior($my, $source_file);
		return ($runs, $run_message, $run_hint) if !$runs;
		return evaluate_expectations($expect_path, $my);
	}

	my $ref_data = getdata($ref);
	my $my_data = getdata($my);
	my @missing_output = ();
	push @missing_output, $ref if !defined($ref_data);
	push @missing_output, $my if !defined($my_data);
	return (0, "ERROR: missing output file (" . join(', ', @missing_output) . ")")
		if scalar(@missing_output) != 0;

	{
		my ($lowers, $lower_message, $lower_hint) = check_lowir_lowers($my, $source_file);
		return ($lowers, $lower_message, $lower_hint) if !$lowers;
		my ($behaves, $behave_message, $behave_hint) = check_source_behavior($my, $source_file);
		return ($behaves, $behave_message, $behave_hint) if !$behaves;
		my ($runs, $run_message, $run_hint) = check_lowir_behavior($my, $source_file);
		return ($runs, $run_message, $run_hint) if !$runs;
	}

	my $direct_compare = env_flag_enabled('CPPGM_LOWIR_DIRECT_TEXT_COMPARE');
	my ($ref_valid, $ref_error) = validate_lowir_text(
		$ref_data,
		$source_file,
		{ strict_presentation_order => 1,
		  allow_legacy_trivial_lifecycle => 1,
		  allow_legacy_unreachable_role => 1,
		  allow_legacy_decay_surface => 1,
		  allow_legacy_reference_pass => 1,
		  allow_legacy_capture_access => 1,
		  allow_legacy_source_origin_projection => 1 });
	return (0, "ERROR: invalid reference LowIR: $ref_error") if !$ref_valid;
	my ($my_valid, $my_error) = validate_lowir_text(
		$my_data,
		$source_file,
		{ strict_presentation_order => $direct_compare });
	return (0, "ERROR: generated LowIR failed sanity validation: $my_error") if !$my_valid;

	# Most course outputs are byte-identical to their reference; identical
	# texts have identical canonical forms, so the canonicalization is skipped.
	return (1, undef, undef) if $ref_data eq $my_data;
	return (0, "ERROR: generated LowIR does not match reference with direct text compare")
		if $direct_compare;

	my ($ref_compare_data, $my_compare_data) =
		canonicalize_lowir_pair_for_compare($ref_data, $my_data);

	return (1, undef, undef) if $ref_compare_data eq $my_compare_data;
	return (1, undef, undef)
		if lowir_reference_matches_after_canonicalization(
			$ref_compare_data, $my_compare_data);

	my ($ref_compare, $my_compare, $diff_path) =
		write_lowir_compare_failure_artifacts($my,
		                                      $ref_compare_data,
		                                      $my_compare_data);
	return (0,
	        "ERROR: generated LowIR does not match reference after relaxed metadata, local-name, and order canonicalization; canonical diff written to $diff_path",
	        lowir_compare_failure_hint($ref_compare, $my_compare, $diff_path));
}

# The debug-information lanes of `cppgm++ --emit-lowir` check that source
# locations reach the emitted LowIR.  Names of temporaries, slots and blocks
# are presentation there as in every relaxed comparison, but the
# `!dbg(file, line, column)` carrier is what these lanes test: the output
# must carry exactly the reference's debug locations, each as often as the
# reference does, and the relaxed comparison then holds each to its place.
sub lowir_debug_locations
{
	my ($data) = @_;
	my %count;
	while ($data =~ /!dbg\(\s*([^,()\s]+)\s*,\s*([^,()\s]+)\s*,\s*([^,()\s]+)\s*\)/g)
	{
		++$count{"$1, $2, $3"};
	}
	return \%count;
}

sub describe_lowir_debug_locations
{
	my ($count, @locations) = @_;
	return join(', ', map {
		"!dbg($_)" . ($count->{$_} > 1 ? " x$count->{$_}" : '')
	} @locations);
}

sub compare_lowir_debug_locations
{
	my ($ref_suffix, $my_suffix, $testbase) = @_;
	my $ref = "$testbase.$ref_suffix";
	my $my = "$testbase.$my_suffix";
	my $ref_status = getdata("$ref.exit_status");
	my $my_status = getdata("$my.exit_status");
	my $ref_data = getdata($ref);
	my $my_data = getdata($my);
	# Missing outputs and exit-status mismatches are the relaxed comparison's
	# to report.
	if (defined($ref_status) && defined($my_status) &&
	    $ref_status eq 'EXIT_SUCCESS' && $my_status eq 'EXIT_SUCCESS' &&
	    defined($ref_data) && defined($my_data))
	{
		my $expected = lowir_debug_locations($ref_data);
		my $actual = lowir_debug_locations($my_data);
		my (%missing, %unexpected);
		for my $location (keys %{$expected})
		{
			my $extra = $expected->{$location} - ($actual->{$location} // 0);
			$missing{$location} = $extra if $extra > 0;
		}
		for my $location (keys %{$actual})
		{
			my $extra = $actual->{$location} - ($expected->{$location} // 0);
			$unexpected{$location} = $extra if $extra > 0;
		}
		if (%missing || %unexpected)
		{
			my @parts;
			push @parts, 'missing ' .
				describe_lowir_debug_locations(\%missing, sort keys %missing)
				if %missing;
			push @parts, 'unexpected ' .
				describe_lowir_debug_locations(\%unexpected, sort keys %unexpected)
				if %unexpected;
			return (0, 'ERROR: generated LowIR debug locations do not match reference (' .
				join('; ', @parts) . ')', undef);
		}
	}
	return compare_lowir_text($ref_suffix, $my_suffix, $testbase);
}

sub canonical_exit_status
{
	my ($status) = @_;
	return undef if !defined($status);
	return 'EXIT_SUCCESS' if $status eq '0' || $status eq 'EXIT_SUCCESS';
	return 'EXIT_TIMEOUT' if $status eq '124' || $status eq 'EXIT_TIMEOUT';
	return 'EXIT_OOM' if $status eq '125' || $status eq 'EXIT_OOM';
	return 'EXIT_NOT_IMPLEMENTED' if $status eq '86' || $status eq 'EXIT_NOT_IMPLEMENTED';
	return 'EXIT_FAILURE' if $status eq '1' || $status eq 'EXIT_FAILURE';
	return $status;
}

sub format_exit_status
{
	my ($status) = @_;
	return '<missing>' if !defined($status);
	my $canonical = canonical_exit_status($status);
	return $status if !defined($canonical) || $canonical eq $status;
	return "$canonical ($status)";
}

sub status_mismatch_message
{
	my ($label, $expected_raw, $got_raw) = @_;
	my $expected = canonical_exit_status($expected_raw);
	my $got = canonical_exit_status($got_raw);
	my $expected_text = format_exit_status($expected_raw);
	my $got_text = format_exit_status($got_raw);
	if (defined($got) && $got eq 'EXIT_TIMEOUT' &&
	    (!defined($expected) || $expected ne 'EXIT_TIMEOUT'))
	{
		return "ERROR: $label timed out (expected $expected_text, got $got_text)";
	}
	if (defined($expected) && $expected eq 'EXIT_TIMEOUT' &&
	    (!defined($got) || $got ne 'EXIT_TIMEOUT'))
	{
		return "ERROR: $label did not time out as expected (expected $expected_text, got $got_text)";
	}
	if (defined($got) && $got eq 'EXIT_OOM' &&
	    (!defined($expected) || $expected ne 'EXIT_OOM'))
	{
		return "ERROR: $label ran out of memory (expected $expected_text, got $got_text)";
	}
	if (defined($expected) && $expected eq 'EXIT_OOM' &&
	    (!defined($got) || $got ne 'EXIT_OOM'))
	{
		return "ERROR: $label did not run out of memory as expected (expected $expected_text, got $got_text)";
	}
	return "ERROR: $label exit status mismatch (expected $expected_text, got $got_text)";
}

sub compare_program_outputs
{
	my ($ref_prefix, $my_prefix, $label, $allow_missing_stdout) = @_;
	$label = 'generated' if !defined($label);
	my $ref_impl_raw = getdata("$ref_prefix.impl.exit_status");
	my $my_impl_raw = getdata("$my_prefix.impl.exit_status");
	my $ref_impl = canonical_exit_status($ref_impl_raw);
	my $my_impl = canonical_exit_status($my_impl_raw);
	return (0, status_mismatch_message("implementation", $ref_impl_raw, $my_impl_raw))
		if !defined($ref_impl) || !defined($my_impl) || $ref_impl ne $my_impl;
	return (1, undef) if $ref_impl ne 'EXIT_SUCCESS';
	my $ref_program_status = getdata("$ref_prefix.program.exit_status");
	my $my_program_status = getdata("$my_prefix.program.exit_status");
	my $ref_program_stdout = getdata("$ref_prefix.program.stdout");
	my $my_program_stdout = getdata("$my_prefix.program.stdout");
	if ($allow_missing_stdout)
	{
		$ref_program_stdout = '' if !defined($ref_program_stdout);
		$my_program_stdout = '' if !defined($my_program_stdout);
	}
	my @missing_program = ();
	push @missing_program, "$ref_prefix.program.exit_status" if !defined($ref_program_status);
	push @missing_program, "$my_prefix.program.exit_status" if !defined($my_program_status);
	return (0, "ERROR: missing program output (" . join(', ', @missing_program) . ")")
		if scalar(@missing_program) != 0;
	my $ref_program = canonical_exit_status($ref_program_status);
	my $my_program = canonical_exit_status($my_program_status);
	return (0, status_mismatch_message("$label program", $ref_program_status, $my_program_status))
		if !defined($ref_program) || !defined($my_program) || $ref_program ne $my_program;
	@missing_program = ();
	push @missing_program, "$ref_prefix.program.stdout" if !defined($ref_program_stdout);
	push @missing_program, "$my_prefix.program.stdout" if !defined($my_program_stdout);
	return (0, "ERROR: missing program output (" . join(', ', @missing_program) . ")")
		if scalar(@missing_program) != 0;
	return (1, undef)
		if ($ref_program_stdout eq $my_program_stdout);
	return (0, "ERROR: $label program stdout does not match reference");
}

sub host_platform_tag
{
	chomp(my $os = `uname -s 2>/dev/null`);
	$os = lc($os);
	return 'macos' if $os eq 'darwin';
	return 'linux' if $os eq 'linux';
	return $os;
}

sub compare_optional_inspect
{
	my ($ref_prefix, $my_prefix, $host_tag) = @_;
	my @ref_candidates = ();
	push @ref_candidates, "$ref_prefix.inspect.$host_tag" if defined($host_tag) && $host_tag ne '';
	push @ref_candidates, "$ref_prefix.inspect";
	my $ref_file;
	for my $candidate (@ref_candidates)
	{
		if (-f $candidate)
		{
			$ref_file = $candidate;
			last;
		}
	}
	return (1, undef, undef) if !defined($ref_file);

	my $my_file = "$my_prefix.inspect";
	my $ref_data = getdata($ref_file);
	my $my_data = getdata($my_file);
	return (0, "ERROR: missing inspect output", "To compare inspect output:\n\n    \$ diff " .
		rooted_path($ref_file) . " " . rooted_path($my_file) . "\n\n")
		if !defined($ref_data) || !defined($my_data);
	return ($ref_data eq $my_data ? (1, undef, undef)
		: (0, "ERROR: inspect output does not match reference",
		   "To compare inspect output:\n\n    \$ diff " .
		   rooted_path($ref_file) . " " . rooted_path($my_file) . "\n\n"));
}

if (scalar(@ARGV) == 2 && $ARGV[0] eq 'canonicalize-machine-ir')
{
	my $path = $ARGV[1];
	my $data;
	if ($path eq '-')
	{
		local $/;
		$data = <STDIN>;
		$data = '' if !defined($data);
	}
	else
	{
		$data = getrawdata($path);
		die "Unable to read $path\n" if !defined($data);
	}
	print canonicalize_machine_ir($data);
	exit 0;
}

if (scalar(@ARGV) == 2 && $ARGV[0] eq 'normalize-machine-ir')
{
	my $path = $ARGV[1];
	my $data;
	if ($path eq '-')
	{
		local $/;
		$data = <STDIN>;
		$data = '' if !defined($data);
	}
	else
	{
		$data = getrawdata($path);
		die "Unable to read $path\n" if !defined($data);
	}
	print normalize_machine_ir($data);
	exit 0;
}

if (scalar(@ARGV) != 4)
{
	die "Usage: compare_results_common.pl <mode> <ref_suffix> <my_suffix> <testlocation>";
}

my $mode = shift(@ARGV);
$LOWIR_COMPARE_SOURCE_FACTS = $mode eq 'lowir_optimized_t' ? 0 : 1;
my $ref_suffix = $ARGV[0];
my $my_suffix = $ARGV[1];
my $tests = $ARGV[2];
my $verbose = $ENV{VERBOSE} || $ENV{CPGM_TEST_VERBOSE};
my $requested_keep_going = $ENV{KEEP_GOING};
my $auto_check_keep_going = env_flag_enabled('CPPGM_CHECK_AUTO_KEEP_GOING');
my $check_mode = env_flag_enabled('CPPGM_CHECK_MODE');
my $cwd = getcwd();
my $assignment = basename($cwd);
my $repo_root = dirname($cwd);
my $host_tag = host_platform_tag();

my %patterns = (
	text_t => qr/\.t$/,
	lowir_t => qr/\.t$/,
	lowir_optimized_t => qr/\.t$/,
	lowir_roundtrip_t => qr/\.t$/,
	lowir_debuginfo_t => qr/\.t$/,
	text_t1 => qr/\.t\.1$/,
	program_t => qr/\.t$/,
	mir_t => qr/\.t$/,
	mir_course_t => qr/\.t$/,
	mir_behavior_t => qr/\.t$/,
	mir_canonical_t => qr/\.t$/,
	mir_structural_t => qr/\.t$/,
	map_struct_t => qr/\.t$/,
	map_exact_t => qr/\.t$/,
	link_program_t => qr/\.t$/,
);
die "Unsupported compare_results mode $mode" if !exists($patterns{$mode});

my @tests = collect_tests($tests, $patterns{$mode});
my $suite_total = scalar(@tests);
my $keep_going = $requested_keep_going || ($auto_check_keep_going && $suite_total > 1);
my $npass = 0;
my $failed = 0;

sub compare_label
{
	return "$assignment check" if $check_mode;
	return "$assignment $tests";
}

sub rooted_path
{
	my ($path) = @_;
	return $path =~ m{^/} ? $path : "$cwd/$path";
}

sub fail_prefix
{
	return compare_label() . ": FAIL after $npass/$suite_total passed\n";
}

sub rerun_hint
{
	return "To rerun this assignment with per-test output from the repo root:\n\n    \$ cd $repo_root && VERBOSE=1 make $assignment\n\n";
}

sub checked_output_hint
{
	my ($ref_path, $my_path) = @_;
	return "To inspect this checked output:\n\n    \$ cat " .
		rooted_path("$ref_path.exit_status") . " " .
		rooted_path("$my_path.exit_status") . "\n    \$ diff " .
		rooted_path($ref_path) . " " . rooted_path($my_path) . "\n\n";
}

sub program_output_hint
{
	my ($ref_prefix, $my_prefix) = @_;
	return "To inspect this program result:\n\n    \$ cat " .
		rooted_path("$ref_prefix.impl.exit_status") . " " .
		rooted_path("$my_prefix.impl.exit_status") . "\n    \$ cat " .
		rooted_path("$ref_prefix.program.exit_status") . " " .
		rooted_path("$my_prefix.program.exit_status") . "\n    \$ diff " .
		rooted_path("$ref_prefix.program.stdout") . " " .
		rooted_path("$my_prefix.program.stdout") . "\n\n";
}

for my $test (@tests)
{
	print "\n$test: " if $verbose;
	my $display_test = $keep_going ? "$assignment/$test" : $test;
	my $testbase = $test;
	$testbase =~ s/\.t$//;
	$testbase =~ s/\.t\.1$//;

	my ($ok, $message, $hint);
	eval {
		if ($mode eq 'text_t')
		{
			($ok, $message) = compare_text(qr/\.t$/, $ref_suffix, $my_suffix, $testbase, 0);
			$hint = checked_output_hint("$testbase.$ref_suffix", "$testbase.$my_suffix");
		}
		elsif ($mode eq 'lowir_roundtrip_t')
		{
			($ok, $message) = compare_lowir_roundtrip($ref_suffix, $my_suffix, $testbase);
			$hint = checked_output_hint("$testbase.$ref_suffix", "$testbase.$my_suffix");
		}
		elsif ($mode eq 'lowir_t' || $mode eq 'lowir_optimized_t')
		{
			($ok, $message, $hint) =
				compare_lowir_text($ref_suffix, $my_suffix, $testbase);
			$hint = checked_output_hint("$testbase.$ref_suffix", "$testbase.$my_suffix")
				if !defined($hint);
		}
		elsif ($mode eq 'lowir_debuginfo_t')
		{
			($ok, $message, $hint) =
				compare_lowir_debug_locations($ref_suffix, $my_suffix, $testbase);
			$hint = checked_output_hint("$testbase.$ref_suffix", "$testbase.$my_suffix")
				if !defined($hint);
		}
		elsif ($mode eq 'text_t1')
		{
			($ok, $message) = compare_text(qr/\.t\.1$/, $ref_suffix, $my_suffix, $testbase, 1);
			$hint = checked_output_hint("$testbase.$ref_suffix", "$testbase.$my_suffix");
		}
		elsif ($mode eq 'program_t')
		{
			($ok, $message) = compare_program_outputs(
				"$testbase.$ref_suffix", "$testbase.$my_suffix", 'generated');
			$hint = program_output_hint("$testbase.$ref_suffix", "$testbase.$my_suffix");
		}

		elsif ($mode eq 'mir_t' || $mode eq 'mir_course_t')
		{
			my $ref = "$testbase.$ref_suffix";
			my $my = "$testbase.$my_suffix";
			my $ref_impl_raw = getdata("$ref.impl.exit_status");
			my $my_impl_raw = getdata("$my.impl.exit_status");
			my $ref_impl = canonical_exit_status($ref_impl_raw);
			my $my_impl = canonical_exit_status($my_impl_raw);
			($ok, $message) = (0, status_mismatch_message("implementation", $ref_impl_raw, $my_impl_raw))
				if !defined($ref_impl) || !defined($my_impl) || $ref_impl ne $my_impl;
			if (!defined($ok))
			{
				if ($ref_impl ne 'EXIT_SUCCESS')
				{
					($ok, $message) = (1, undef);
				}
				elsif (defined(expectation_sidecar($testbase)))
				{
					my ($expect_ok, $expect_message, $expect_hint) =
						evaluate_expectations(expectation_sidecar($testbase), "$my.mir");
					if (!$expect_ok)
					{
						($ok, $message, $hint) = ($expect_ok, $expect_message, $expect_hint);
					}
					else
					{
						my ($prog_ok, $prog_message) = compare_program_outputs($ref, $my, 'generated');
						($ok, $message) = ($prog_ok, $prog_message);
						$hint = program_output_hint($ref, $my);
					}
				}
				elsif ($mode eq 'mir_course_t' && !-f "$my.mir")
				{
					($ok, $message) = (0, "ERROR: missing machine IR dump (.mir)");
				}
				elsif ($mode eq 'mir_t' && (-f "$ref.mir") &&
				       (normalize_machine_ir(getdata("$ref.mir")) ne normalize_machine_ir(getdata("$my.mir"))))
				{
					($ok, $message) = (0, "ERROR: machine IR dumps do not match (.mir)");
					$hint = "To compare machine IR dumps:\n\n    \$ diff " .
						rooted_path("$ref.mir") . " " . rooted_path("$my.mir") . "\n\n";
				}
				else
				{
					my ($prog_ok, $prog_message) = compare_program_outputs($ref, $my, 'generated');
					($ok, $message) = ($prog_ok, $prog_message);
					$hint = program_output_hint($ref, $my);
				}
			}
		}
		elsif ($mode eq 'mir_behavior_t')
		{
			my $ref = "$testbase.$ref_suffix";
			my $my = "$testbase.$my_suffix";
			my $ref_impl_raw = getdata("$ref.impl.exit_status");
			my $my_impl_raw = getdata("$my.impl.exit_status");
			my $ref_impl = canonical_exit_status($ref_impl_raw);
			my $my_impl = canonical_exit_status($my_impl_raw);
			($ok, $message) = (0, status_mismatch_message("implementation", $ref_impl_raw, $my_impl_raw))
				if !defined($ref_impl) || !defined($my_impl) || $ref_impl ne $my_impl;
			if (!defined($ok))
			{
				if ($ref_impl ne 'EXIT_SUCCESS')
				{
					($ok, $message) = (1, undef);
				}
				elsif (!-f "$ref.mir")
				{
					($ok, $message) = (0,
						"ERROR: missing informational reference machine IR dump (.ref.mir)");
				}
				elsif (!-f "$my.mir")
				{
					($ok, $message) = (0,
						"ERROR: missing generated machine IR dump (.my.mir)");
				}
				else
				{
					my ($expect_ok, $expect_message, $expect_hint) = (1, undef, undef);
					($expect_ok, $expect_message, $expect_hint) =
						evaluate_expectations(expectation_sidecar($testbase), "$my.mir")
						if defined(expectation_sidecar($testbase));
					if (!$expect_ok)
					{
						($ok, $message, $hint) = ($expect_ok, $expect_message, $expect_hint);
					}
					else
					{
						my ($prog_ok, $prog_message) = compare_program_outputs($ref, $my, 'generated');
						($ok, $message) = ($prog_ok, $prog_message);
						$hint = program_output_hint($ref, $my);
					}
				}
			}
		}
		elsif ($mode eq 'mir_canonical_t')
		{
			my $ref = "$testbase.$ref_suffix";
			my $my = "$testbase.$my_suffix";
			my $ref_impl_raw = getdata("$ref.impl.exit_status");
			my $my_impl_raw = getdata("$my.impl.exit_status");
			my $ref_impl = canonical_exit_status($ref_impl_raw);
			my $my_impl = canonical_exit_status($my_impl_raw);
			($ok, $message) = (0, status_mismatch_message("implementation", $ref_impl_raw, $my_impl_raw))
				if !defined($ref_impl) || !defined($my_impl) || $ref_impl ne $my_impl;
			if (!defined($ok))
			{
				if ($ref_impl ne 'EXIT_SUCCESS')
				{
					($ok, $message) = (1, undef);
				}
				else
				{
					my ($mir_ok, $mir_message, $mir_hint) = compare_canonical_machine_ir($ref, $my);
					if (!$mir_ok)
					{
						($ok, $message, $hint) = ($mir_ok, $mir_message, $mir_hint);
					}
					else
					{
						my ($prog_ok, $prog_message) = compare_program_outputs($ref, $my, 'generated');
						($ok, $message) = ($prog_ok, $prog_message);
						$hint = program_output_hint($ref, $my);
					}
				}
			}
		}
		elsif ($mode eq 'mir_structural_t')
		{
			my $ref = "$testbase.$ref_suffix";
			my $my = "$testbase.$my_suffix";
			my $ref_impl_raw = getdata("$ref.impl.exit_status");
			my $my_impl_raw = getdata("$my.impl.exit_status");
			my $ref_impl = canonical_exit_status($ref_impl_raw);
			my $my_impl = canonical_exit_status($my_impl_raw);
			($ok, $message) = (0, status_mismatch_message("implementation", $ref_impl_raw, $my_impl_raw))
				if !defined($ref_impl) || !defined($my_impl) || $ref_impl ne $my_impl;
			if (!defined($ok))
			{
				if ($ref_impl ne 'EXIT_SUCCESS')
				{
					($ok, $message) = (1, undef);
				}
				else
				{
					my ($mir_ok, $mir_message, $mir_hint) = defined(expectation_sidecar($testbase)) ?
						evaluate_expectations(expectation_sidecar($testbase), "$my.mir") :
						compare_structural_machine_ir($ref, $my);
					if (!$mir_ok)
					{
						($ok, $message, $hint) = ($mir_ok, $mir_message, $mir_hint);
					}
					else
					{
						my ($prog_ok, $prog_message) = compare_program_outputs($ref, $my, 'generated');
						($ok, $message) = ($prog_ok, $prog_message);
						$hint = program_output_hint($ref, $my);
					}
				}
			}
		}
		elsif ($mode eq 'map_struct_t')
		{
			my $ref = "$testbase.$ref_suffix";
			my $my = "$testbase.$my_suffix";
			my $ref_impl_raw = getdata("$ref.impl.exit_status");
			my $my_impl_raw = getdata("$my.impl.exit_status");
			my $ref_impl = canonical_exit_status($ref_impl_raw);
			my $my_impl = canonical_exit_status($my_impl_raw);
			($ok, $message) = (0, status_mismatch_message("implementation", $ref_impl_raw, $my_impl_raw))
				if !defined($ref_impl) || !defined($my_impl) || $ref_impl ne $my_impl;
			if (!defined($ok))
			{
				if ($ref_impl ne 'EXIT_SUCCESS')
				{
					($ok, $message) = (1, undef);
				}
				elsif ((-f "$ref.map") && !compare_link_maps_structural("$ref.map", "$my.map"))
				{
					($ok, $message) = (0, "ERROR: link maps do not match structurally");
					$hint = "To compare link maps:\n\n    \$ diff " .
						rooted_path("$ref.map") . " " . rooted_path("$my.map") . "\n\n";
				}
				else
				{
					my ($prog_ok, $prog_message) = compare_program_outputs($ref, $my, 'linked');
					($ok, $message) = ($prog_ok, $prog_message);
					$hint = program_output_hint($ref, $my);
				}
			}
		}
		elsif ($mode eq 'map_exact_t')
		{
			my $ref = "$testbase.$ref_suffix";
			my $my = "$testbase.$my_suffix";
			my $ref_impl_raw = getdata("$ref.impl.exit_status");
			my $my_impl_raw = getdata("$my.impl.exit_status");
			my $ref_impl = canonical_exit_status($ref_impl_raw);
			my $my_impl = canonical_exit_status($my_impl_raw);
			($ok, $message) = (0, status_mismatch_message("implementation", $ref_impl_raw, $my_impl_raw))
				if !defined($ref_impl) || !defined($my_impl) || $ref_impl ne $my_impl;
			if (!defined($ok))
			{
				if ($ref_impl ne 'EXIT_SUCCESS')
				{
					($ok, $message) = (1, undef);
				}
				elsif ((-f "$ref.map") && (getdata("$ref.map") ne getdata("$my.map")))
				{
					($ok, $message) = (0, "ERROR: link maps do not match");
					$hint = "To compare link maps:\n\n    \$ diff " .
						rooted_path("$ref.map") . " " . rooted_path("$my.map") . "\n\n";
				}
				else
				{
					my ($prog_ok, $prog_message) = compare_program_outputs($ref, $my, 'linked');
					($ok, $message) = ($prog_ok, $prog_message);
					$hint = program_output_hint($ref, $my);
				}
			}
		}
		elsif ($mode eq 'link_program_t')
		{
			my ($prog_ok, $prog_message) = compare_program_outputs(
				"$testbase.$ref_suffix",
				"$testbase.$my_suffix",
				'linked',
				1);
			($ok, $message) = ($prog_ok, $prog_message);
			$hint = program_output_hint("$testbase.$ref_suffix", "$testbase.$my_suffix");
			if ($ok)
			{
				my ($inspect_ok, $inspect_message, $inspect_hint) =
					compare_optional_inspect("$testbase.$ref_suffix",
					                         "$testbase.$my_suffix",
					                         $host_tag);
				($ok, $message) = ($inspect_ok, $inspect_message);
				$hint = $inspect_hint if defined($inspect_hint);
			}
		}
		else
		{
			die "Unsupported compare_results mode $mode";
		}
	};
	if ($@)
	{
		$ok = 0;
		$message = "ERROR: " . $@;
		$message =~ s/\n+$//;
	}

	if ($ok)
	{
		++$npass;
		print "PASS\n\n" if $verbose;
		next;
	}

	if ($keep_going)
	{
		print "$display_test: $message\n";
		print $hint if $check_mode && $auto_check_keep_going && defined($hint);
		$failed = 1;
		next;
	}

	print fail_prefix();
	print "$test: $message\n\n";
	print rerun_hint();
	print $hint if defined($hint);
	print "TEST FAIL\n";
	exit(1);
}

if ($keep_going && $check_mode)
{
	print lowir_behavior_summary();
	print compare_label() . ": " . ($failed ? "FAIL" : "PASS") . " ($npass/$suite_total)\n";
}
elsif (!$keep_going)
{
	print lowir_behavior_summary();
	print compare_label() . ": PASS ($npass/$suite_total)\n";
}
append_keep_going_summary($repo_root, $cwd, $npass, $suite_total, $failed)
	if $keep_going && !$check_mode;
exit($failed && (!$keep_going || $auto_check_keep_going) ? 1 : 0);
