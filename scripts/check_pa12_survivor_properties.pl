#!/usr/bin/perl

use strict;
use warnings;

use File::Temp qw(tempdir);
use FindBin;
use lib $FindBin::Bin;

use CppgmBatchWorker qw(collect_tests run_command_capture);

sub read_file
{
	my ($path) = @_;
	open(my $fh, '<', $path) or die "Unable to read $path: $!\n";
	local $/;
	my $data = <$fh>;
	close($fh) or die "Unable to close $path: $!\n";
	return defined($data) ? $data : '';
}

sub function_body
{
	my ($test, $lowir, $name) = @_;
	return $1 if $lowir =~
		/(function \@\Q$name\E\b.*?)(?=\nfunction \@|\z)/s;
	die "$test: generated LowIR has no $name definition\n";
}

# The headers of the definitions of the C++ entities whose Itanium names
# match $re: the function's own name, or its `object=` metadata (the
# comparison pairs internal symbols by the latter).
sub definition_headers_of_entities
{
	my ($lowir, $re) = @_;
	my @headers;
	for my $line (split(/\n/, $lowir))
	{
		next if $line !~ /^function \@([A-Za-z0-9_]+)\(/;
		my $symbol = $1;
		my ($object) = $line =~ /\[[^\]]*\bobject=([^,\]\s]+)/;
		push @headers, $line
			if $symbol =~ $re || (defined($object) && $object =~ $re);
	}
	return @headers;
}

# The symbols of the allocation functions (role=allocate_memory) and the
# position of the first call of one of them in $body, or -1.
sub first_allocation_index
{
	my ($lowir, $body) = @_;
	my @allocators = $lowir =~
		/^declare function \@([A-Za-z0-9_]+)\([^\n]*\brole=allocate_memory\b/mg;
	my $at = -1;
	for my $allocator (@allocators)
	{
		my $index = index($body, "call ptr \@$allocator(");
		$at = $index if $index >= 0 && ($at < 0 || $index < $at);
	}
	return $at;
}

sub compile_source
{
	my ($app, $test, $directory) = @_;
	my $output = "$directory/test.lowir";
	my $status = run_command_capture(
		cmd => [$app, '--emit-lowir', '-O0', '-o', $output, $test],
		stdout => "$directory/source.stdout",
		stderr => "$directory/source.stderr",
		timeout => 60,
	);
	die "$test: source-to-LowIR compile failed\n" .
		read_file("$directory/source.stderr") if $status != 0;
	return ($output, read_file($output));
}

sub compile_and_run
{
	my ($app, $test, $directory, $lowir) = @_;
	my $program = "$directory/program";
	my $status = run_command_capture(
		cmd => [$app, '-O0', '-o', $program, $lowir],
		stdout => "$directory/native.stdout",
		stderr => "$directory/native.stderr",
		timeout => 60,
	);
	die "$test: generated LowIR did not compile\n" .
		read_file("$directory/native.stderr") if $status != 0;
	$status = run_command_capture(
		cmd => [$program],
		stdout => "$directory/program.stdout",
		stderr => "$directory/program.stderr",
		timeout => 30,
	);
	die "$test: generated program failed with status $status\n"
		if $status != 0;
}

if(scalar(@ARGV) != 3)
{
	die "Usage: check_pa12_survivor_properties.pl " .
		"<cppgm++> <native-reference> <test-or-directory>\n";
}

my ($app, $native_reference, $root) = @ARGV;
my @tests = collect_tests($root,
	qr/(?:constructor-alias-boundaries|enclosing-temporary-lifetime|out-of-class-move-assignment-boundary|conditional-copy-elision-permission|parameter-object-extent-boundary|parameter-object-identity|parameter-destruction-full-expression|array-new-size-width|array-new-cookie-layout|array-new-length-check|array-new-braced-list|array-new-class-braced-list|named-return-object-identity|static-reference-lifetime|conditional-transfer-selection|empty-value-initialization)\.cpp$/);
die "No PA12 survivor-property tests found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('pa12-survivor-property-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my ($path, $lowir) = compile_source($app, $test, $directory);
	if($test =~ /(?:static-reference-lifetime|conditional-transfer-selection|empty-value-initialization|array-new-size-width|array-new-cookie-layout|array-new-class-braced-list|named-return-object-identity)/) {
		compile_and_run($native_reference, $test, $directory, $path);
		next;
	}

	if($test =~ /constructor-alias-boundaries/) {
		my @constructors = definition_headers_of_entities($lowir, qr/C1E(?:RKS_|OS_)/);
		die "$test: copy/move constructor definitions are missing\n"
			if scalar(@constructors) != 2;
		for my $header (@constructors) {
			my @noalias = $header =~ /\bptr \[[^\]]*\balias=noalias\b[^\]]*\]/g;
			die "$test: constructor destination/source are not both noalias\n"
				if scalar(@noalias) != 2;
		}
		my ($assignment) = definition_headers_of_entities($lowir, qr/aSERKS_/);
		die "$test: copy-assignment control definition is missing\n"
			if !defined($assignment);
		die "$test: assignment operator incorrectly received constructor noalias facts\n"
			if $assignment =~ /\balias=noalias\b/;
		compile_and_run($native_reference, $test, $directory, $path);
		next;
	}
	if($test =~ /enclosing-temporary-lifetime/) {
		# Temporaries are destroyed in the reverse order of construction
		# (12.2p5): after the operator's call, the inner operand first and
		# the enclosing temporary second; and a failing inner construction
		# destroys the enclosing temporary on its cleanup path.  Objects
		# are known by the slots their address temporaries name, however
		# often an address is taken.
		my $main = function_body($test, $lowir, 'main');
		my %slot_of;
		while ($main =~ /^\s+(%\w+) = addr (\$\w+)$/mg) { $slot_of{$1} = $2; }
		my ($operator, $outer, $inner) = $main =~
			/^\s+%\w+ = call i32 \@(\w+)\((%\w+), (%\w+)\)$/m;
		die "$test: nested temporary/operator relationship is missing\n"
			if !defined($operator) || !defined($slot_of{$outer}) || !defined($slot_of{$inner});
		my $use = index($main, "call i32 \@$operator($outer, $inner)");
		my @destructions = substr($main, $use) =~ /^\s+call void \@(\w+)\((%\w+)\)$/mg;
		my $ordered = scalar(@destructions) >= 4 && $destructions[0] eq $destructions[2] &&
			defined($slot_of{$destructions[1]}) && defined($slot_of{$destructions[3]}) &&
			$slot_of{$destructions[1]} eq $slot_of{$inner} &&
			$slot_of{$destructions[3]} eq $slot_of{$outer};
		die "$test: selected operand and enclosing temporary were not " .
			"destroyed after use in reverse order\n"
			if !$ordered;
		my $destructor = $destructions[0];
		my $cleanup = 0;
		for my $block (split(/(?=^\s+block \^)/m, $main))
		{
			next if $block !~ /^\s+call void \@\Q$destructor\E\((%\w+)\)\n\s+jump \^/m;
			next if !defined($slot_of{$1}) || $slot_of{$1} ne $slot_of{$outer};
			$cleanup = 1;
			last;
		}
		die "$test: inner-construction failure does not clean the enclosing temporary\n"
			if !$cleanup;
		compile_and_run($native_reference, $test, $directory, $path);
		next;
	}
	if($test =~ /out-of-class-move-assignment-boundary/) {
		my ($assignment) = definition_headers_of_entities($lowir, qr/^_ZN7MoveBoxaSEOS_$/);
		die "$test: out-of-class move assignment did not retain its " .
			"declared rvalue-reference ABI identity\n"
			if !defined($assignment);
		compile_and_run($native_reference, $test, $directory, $path);
		next;
	}
	if($test =~ /conditional-copy-elision-permission/) {
		my $main = function_body($test, $lowir, 'main');
		my ($callee, $destination, $source) = $main =~
			/^\s+call void \@(\w+)\((%\w+), (%\w+)\) \[[^\]]*\belision=copy\b[^\]]*\]$/m;
		die "$test: conditional class prvalue did not emit copy-elision permission\n"
			if !defined($source) || $destination eq $source;
		my $marker = index($main, "elision=copy");
		my $before = substr($main, 0, $marker);
		my $after = substr($main, $marker);
		my @blocks = $before =~ /^\s*block \^(\w+):/mg;
		die "$test: permitted transfer is not in a merge block\n" if !@blocks;
		my $merge = $blocks[-1];
		my $incoming = () = $before =~ /^\s+jump \^\Q$merge\E$/mg;
		my $producers = () = $before =~
			/^\s+call void \@\w+\(\Q$source\E(?:,|\))/mg;
		die "$test: conditional operands do not independently construct the source temporary\n"
			if $incoming < 2 || $producers < 2;
		die "$test: O0 permission removed the ordinary source cleanup\n"
			if $after !~ /^\s+call void \@\w+\(\Q$source\E\)$/m;
		compile_and_run($native_reference, $test, $directory, $path);
		next;
	}
	if($test =~ /parameter-object-identity/) {
		for my $name (qw(take_derived take_mixed take_moved)) {
			die "$test: \@$name does not receive its class by address\n"
				if function_body($test, $lowir, $name) !~
					/\A[^\n]*\(%\w+ : ptr \[[^\]]*\bpass=by_address\b/;
		}
		for my $name (qw(make_mixed copy_mixed)) {
			die "$test: \@$name does not return through caller storage\n"
				if function_body($test, $lowir, $name) !~
					/\A[^\n]*\(%\w+ : ptr \[[^\]]*\bpass=indirect_result\b/;
		}
		compile_and_run($native_reference, $test, $directory, $path);
		next;
	}
	if($test =~ /parameter-destruction-full-expression/) {
		my $take = function_body($test, $lowir, 'take');
		my ($parameter) = $take =~ /\A[^\n]*\((%\w+)/;
		die "$test: \@take has no parameter\n" if !defined($parameter);
		die "$test: \@take destroys its by-address parameter\n"
			if $take =~ /^\s+call void \@\w+\(\Q$parameter\E\)$/m;
		compile_and_run($native_reference, $test, $directory, $path);
		next;
	}
	if($test =~ /array-new-length-check/) {
		my ($runtime) = $lowir =~
			/^declare function \@(\w+)\([^\n]*\brole=bad_array_new_length\b/m;
		die "$test: no bad_array_new_length runtime declaration\n"
			if !defined($runtime);
		for my $name (qw(make_signed make_sized make_class)) {
			my $body = function_body($test, $lowir, $name);
			my $check = index($body, "call void \@$runtime()");
			my $allocation = first_allocation_index($lowir, $body);
			die "$test: \@$name does not check its count before allocating\n"
				if $check < 0 || $allocation < 0 || $check > $allocation;
		}
		die "$test: \@make_unsigned checks a count that cannot be invalid\n"
			if function_body($test, $lowir, 'make_unsigned') =~ /\@\Q$runtime\E\b/;
		compile_and_run($native_reference, $test, $directory, $path);
		next;
	}
	if($test =~ /array-new-braced-list/) {
		my ($runtime) = $lowir =~
			/^declare function \@(\w+)\([^\n]*\brole=bad_array_new_length\b/m;
		my $body = function_body($test, $lowir, 'listed');
		my $check = defined($runtime) ? index($body, "call void \@$runtime()") : -1;
		my $allocation = first_allocation_index($lowir, $body);
		die "$test: \@listed does not check its count against the 3 clauses before allocating\n"
			if $body !~ /^\s+%\w+ = (?:binary sub i64 %\w+, 3|cmp \w+ i64 %\w+, 3)$/m || $check < 0 ||
				$allocation < 0 || $check > $allocation;
		compile_and_run($native_reference, $test, $directory, $path);
		next;
	}
	if($test =~ /parameter-object-extent-boundary/) {
		# Every pointer parameter's metadata group, whatever the order of
		# its keys.
		my @groups = $lowir =~ /\bptr\s+\[([^\]]*)\]/g;
		my @indirect = map { /\bobject_bytes=([1-9][0-9]*)/ ? $1 : () }
			grep { /\bpass=indirect_result\b/ } @groups;
		my @by_address = map { /\bobject_bytes=([1-9][0-9]*)/ ? $1 : () }
			grep { /\bpass=by_address\b/ } @groups;
		my @object_parameters = map { /\bobject_bytes=([1-9][0-9]*)/ ? $1 : () }
			grep { !/\bpass=/ } @groups;
		my @indirect_call_extents = $lowir =~
			/^\s+%[A-Za-z0-9_]+\s+=\s+call\s+i64\s+%[A-Za-z0-9_]+\([^\n]*
			 \bptr\s+\[[^\]]*\bobject_bytes=([1-9][0-9]*)/mgx;
		die "$test: O0 LowIR lacks an indirect-result object extent\n"
			if !@indirect;
		die "$test: O0 LowIR lacks a by-address object extent\n"
			if !@by_address;
		die "$test: O0 LowIR lacks a member-object parameter extent\n"
			if !@object_parameters;
		die "$test: an indirect member call lost its object extent signature\n"
			if !@indirect_call_extents;
		my %indirect_sizes = map { $_ => 1 } @indirect;
		my $shared_extent = scalar(grep { $indirect_sizes{$_} }
			(@by_address, @object_parameters));
		die "$test: related class boundaries disagree on their semantic extent\n"
			if !$shared_extent;
		# The C function taking a pointer to a scalar states no extent.
		my ($pointer_value) = $lowir =~ /^(function \@pointer_value\([^\n]*)$/m;
		die "$test: the extern \"C\" pointer_value definition is missing\n"
			if !defined($pointer_value);
		die "$test: pointer_value's scalar pointer parameter states an object extent\n"
			if $pointer_value =~ /\(%\w+ : ptr \[[^\]]*\bobject_bytes=/;
		die "$test: source lowering attached object_bytes to a non-pointer\n"
			if $lowir =~ /\b(?:i1|i8|u8|i16|u16|i32|u32|i64|f32|f64|f80)\s+\[[^\]]*object_bytes=/;
		compile_and_run($native_reference, $test, $directory, $path);
		next;
	}
	die "$test: no PA12 survivor predicate selected\n";
}

print "PA12 survivor properties: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
