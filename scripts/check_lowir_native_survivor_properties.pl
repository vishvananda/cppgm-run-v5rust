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

sub compile_level
{
	my ($app, $test, $directory, $level) = @_;
	my $mir = "$directory/$level.mir";
	my $program = "$directory/$level.program";
	my $status = run_command_capture(
		cmd => [$app, "-$level", '--dump-machine-ir', $mir,
			'-o', $program, $test],
		stdout => "$directory/$level.stdout",
		stderr => "$directory/$level.stderr",
		timeout => 30,
	);
	die "$test: lowir2native -$level failed\n" .
		read_file("$directory/$level.stderr") if $status != 0;
	$status = run_command_capture(
		cmd => [$program],
		stdout => "$directory/$level-program.stdout",
		stderr => "$directory/$level-program.stderr",
		timeout => 30,
	);
	die "$test: generated -$level program failed with status $status\n"
		if $status != 0;
	return read_file($mir);
}

sub function_body
{
	my ($test, $mir, $name) = @_;
	return $1 if $mir =~
		/(^function \@\Q$name\E\b.*?)(?=^function \@|\z)/ms;
	die "$test: machine IR has no $name function\n";
}

sub block_body
{
	my ($test, $function, $name) = @_;
	return $1 if $function =~
		/(^\s+block \^\Q$name\E\s*\n.*?)(?=^\s+block \^|\z)/ms;
	die "$test: function has no $name block\n";
}

sub frame_offset
{
	my ($body, $name) = @_;
	return $1 if $body =~
		/^\s+(?:temp|param-slot) %\Q$name\E -> (\[rbp[^\]]*\])/m;
	return undef;
}

sub preserved_registers
{
	my ($body) = @_;
	return split(/\s+/, $1) if $body =~ /^\s+preserve\s+([^\n]+)$/m;
	return ();
}

sub preserve_count
{
	my ($body) = @_;
	my @registers = preserved_registers($body);
	return scalar(@registers);
}

# Whether $operand, read after the first call to $callee, survives that
# call: it is a preserved register, a frame home stored before the call,
# or a register reloaded after the call from such a home.
sub call_safe_source
{
	my ($function, $callee, $operand) = @_;
	my $call = index($function, "call \@$callee");
	return 0 if $call < 0;
	return 1 if grep { $_ eq $operand } preserved_registers($function);
	my $home = $operand;
	if($home !~ /^\[rbp[^\]]*\]$/)
	{
		return 0 if substr($function, $call) !~
			/^\s+load\.i64 \Q$operand\E, (\[rbp[^\]]*\])$/m;
		$home = $1;
	}
	return substr($function, 0, $call) =~
		/^\s+store\.i64 \Q$home\E, [^\n]+$/m ? 1 : 0;
}

if(scalar(@ARGV) != 2)
{
	die "Usage: check_lowir_native_survivor_properties.pl " .
		"<lowir2native> <test-or-directory>\n";
}

my ($app, $root) = @ARGV;
my @tests = collect_tests($root,
	qr/(?:\d{3}-loop-invariant-call-crossing-placement|\d{3}-deferred-compare-across-call|\d{3}-volatile-access-emission|\d{3}-eh-edge-placement-barrier|\d{3}-loop-and-eh-placement|\d{3}-native-layout-policy-guards|\d{3}-staged-home-selection|\d{3}-rematerialized-storage-addresses)\.t$/);
die "No native survivor-property tests found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('native-survivor-property-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my $o0 = compile_level($app, $test, $directory, 'O0');
	my $o1 = compile_level($app, $test, $directory, 'O1');
	my $optimized = $test =~
		/(?:\d{3}-loop-invariant-call-crossing-placement|\d{3}-eh-edge-placement-barrier)/
		? compile_level($app, $test, $directory, 'O2') : $o1;

	if($test =~ /\d{3}-loop-invariant-call-crossing-placement/) {
		my $positive = function_body($test, $optimized, 'main');
		die "$test: optimized call-crossing value retained its frame home\n"
			if defined(frame_offset($positive, 'stable'));
		my $loop = block_body($test, $positive, 'loop');
		my ($carrier) = $loop =~
			/^\s+mov rdi, ([a-z0-9]+)\s*\n\s+call \@identity\b/m;
		die "$test: call-crossing value received no call-safe capacity\n"
			if !defined($carrier) ||
			   !grep { $_ eq $carrier } preserved_registers($positive);
		die "$test: loop reloaded the call-crossing value from its frame\n"
			if $loop =~ /\[rbp/;
		my $floating = function_body($test, $optimized, 'float_loop');
		die "$test: call-free floating loop consumed integer preserved capacity\n"
			if preserve_count($floating) != 0;
		next;
	}
	if($test =~ /\d{3}-deferred-compare-across-call/) {
		my $probe = function_body($test, $o1, 'probe');
		my $call = index($probe, 'call @sink');
		my $compare = index($probe, 'cmp.i64');
		my @loads = $probe =~ /^\s+load\.i64 /mg;
		die "$test: compare operands were not loaded before the clobbering call\n"
			if scalar(@loads) != 2 || $call < 0;
		die "$test: compare was not deferred until after the call\n"
			if $compare < $call;
		die "$test: deferred compare operands lack call-safe capacity\n"
			if preserve_count($probe) < 2;
		next;
	}
	if($test =~ /\d{3}-volatile-access-emission/) {
		for my $mir ($o0, $o1) {
			my $body = function_body($test, $mir, 'spin_flag');
			die "$test: native lowering removed or merged volatile traffic\n"
				if scalar(() = $body =~ /^\s+store\.i32 /mg) != 2 ||
				   scalar(() = $body =~ /^\s+load\.i32 /mg) != 2;
		}
		next;
	}
	if($test =~ /\d{3}-eh-edge-placement-barrier/) {
		my $positive = function_body($test, $optimized, 'main');
		for my $level ([ 'O0', $o0 ], [ 'optimized', $optimized ]) {
			my ($label, $mir) = @$level;
			my $main = function_body($test, $mir, 'main');
			my ($source) = block_body($test, $main, 'done') =~
				/^\s+cmp\.i64 ([^,\s]+),/m;
			die "$test: $label EH-crossing value has no call-safe capacity\n"
				if !defined($source) ||
				   !call_safe_source($main, 'identity', $source);
		}
		die "$test: host-EH function incorrectly omitted its frame pointer\n"
			if $positive !~ /^\s+frame_pointer keep$/m;
		next;
	}

	if($test =~ /\d{3}-loop-and-eh-placement/) {
		my $positive = function_body($test, $o1, 'walk_unavoidable');
		for my $name ('cursor', 'count', 'limit') {
			die "$test: O1 retained the unavoidable-loop $name frame home\n"
				if defined(frame_offset($positive, $name));
		}
		die "$test: O1 unavoidable loop kept a value in frame memory\n"
			if $positive =~ /\[rbp/;
		my $call_free = function_body($test, $o1, 'call_free_eh_load');
		die "$test: call-free EH load consumed preserved capacity\n"
			if preserve_count($call_free) != 0;
		my $crossing = function_body(
			$test, $o1, 'call_crossing_eh_value');
		my $done = block_body($test, $crossing, 'done');
		my ($source) = $done =~ /^\s+add ([a-z0-9]+), 1$/m;
		($source) = $done =~ /^\s+lea [a-z0-9]+, \[([a-z0-9]+)\+1\]$/m
			if !defined($source);
		die "$test: call-crossing EH value has no call-safe capacity\n"
			if !defined($source) ||
			   !call_safe_source($crossing, 'identity', $source);
		next;
	}
	if($test =~ /\d{3}-native-layout-policy-guards/) {
		for my $name ('identity', 'frameless_leaf', 'frameless_call',
			'frame_operand', 'dynamic_stack', 'floating_scratch',
			'host_eh', 'direct_returns') {
			my $baseline = function_body($test, $o0, $name);
			my $positive = function_body($test, $o1, $name);
			die "$test: O0 $name lost the shared-epilogue baseline\n"
				if $baseline !~ /^\s+epilogues shared$/m;
			die "$test: O1 $name did not select duplicated epilogues\n"
				if $positive !~ /^\s+epilogues duplicated$/m;
		}
		for my $name ('dynamic_stack', 'host_eh') {
			my $positive = function_body($test, $o1, $name);
			die "$test: guarded O1 $name incorrectly omitted its frame pointer\n"
				if $positive !~ /^\s+frame_pointer keep$/m;
		}
		next;
	}
	if($test =~ /\d{3}-staged-home-selection/) {
		my $positive = function_body($test, $o1, 'copy_to_edge_home');
		my ($value_register) = $positive =~
			/^\s+param %value -> ([a-z0-9]+) : i64$/m;
		my $copy_home = frame_offset($positive, 'copied');
		$copy_home = frame_offset($positive, 'value') if !defined($copy_home);
		die "$test: copy reducer lost its parameter or edge home\n"
			if !defined($value_register) || !defined($copy_home);
		die "$test: O1 did not store the identity copy directly to its home\n"
			if $positive !~
			   /^\s+store\.i64 \Q$copy_home\E, \Q$value_register\E$/m;

		my $baseline = function_body($test, $o0, 'call_result_to_edge_home');
		$positive = function_body($test, $o1, 'call_result_to_edge_home');
		my $value_home = frame_offset($positive, 'value');
		die "$test: call-result reducer lost its stable edge home\n"
			if !defined($value_home);
		die "$test: O1 did not store the first scalar call result directly\n"
			if $positive !~
			   /^\s+call \@identity\b[^\n]*\n\s+store\.i64 \Q$value_home\E, rax$/m;
		my $baseline_home = frame_offset($baseline, 'value');
		die "$test: O0 baseline already had the direct call-result store\n"
			if defined($baseline_home) && $baseline =~
			   /^\s+call \@identity\b[^\n]*\n\s+store\.i64 \Q$baseline_home\E, rax$/m;

		my $dead = function_body($test, $o1, 'load_with_dead_address');
		my ($dead_carrier) = $dead =~
			/^\s+load\.ptr ([a-z0-9]+), \@number_pointer\s*\n\s+load\.i64 \1, \[\1\]$/m;
		die "$test: final-use address register was not taken over by load\n"
			if !defined($dead_carrier);
		my $live = function_body($test, $o1, 'load_with_live_address');
		my ($address, $value) = $live =~
			/^\s+load\.ptr ([a-z0-9]+), \@number_pointer\s*\n\s+load\.i64 ([a-z0-9]+), \[\1\]$/m;
		die "$test: live-address control lost its paired loads\n"
			if !defined($address) || !defined($value);
		die "$test: live address was overwritten by its load result\n"
			if $address eq $value;
		next;
	}
	if($test =~ /\d{3}-rematerialized-storage-addresses/) {
		my $frame = function_body($test, $o1, 'frame_storage_after_call');
		die "$test: frame storage was materialized into an address register\n"
			if $frame =~ /^\s+lea [^,]+, \[rbp/m;
		die "$test: frame storage did not remain a direct load/store operand\n"
			if $frame !~ /^\s+store\.i64 \[rbp[^\]]*\], 19$/m ||
			   $frame !~ /^\s+load\.i64 [a-z0-9]+, \[rbp[^\]]*\]$/m;
		my $global = function_body($test, $o1, 'global_storage_after_call');
		die "$test: local global storage did not remain a direct operand\n"
			if $global !~ /^\s+load\.i64 [a-z0-9]+, \@global_value$/m;
		die "$test: local global address was unnecessarily materialized\n"
			if $global =~ /^\s+(?:mov|lea) [a-z0-9]+, \@global_value$/m;
		my $constant = function_body(
			$test, $o1, 'constant_field_after_call');
		die "$test: constant frame field did not fold into direct operands\n"
			if $constant =~ /^\s+lea [^,]+, \[rbp/m ||
			   $constant !~ /^\s+store\.i64 \[rbp[^\]]*\], 23$/m ||
			   $constant !~ /^\s+load\.i64 [a-z0-9]+, \[rbp[^\]]*\]$/m;
		my $variable = function_body(
			$test, $o1, 'variable_index_after_call');
		die "$test: variable frame address was materialized into a register\n"
			if $variable =~ /^\s+lea [^,]+, \[rbp/m;
		my ($index) = $variable =~
			/^\s+store\.i64 \[rbp\+([a-z0-9]+)\*8[^\]]*\], 29$/m;
		die "$test: variable frame index did not fold into indexed operands\n"
			if !defined($index) || $variable !~
			   /^\s+load\.i64 [a-z0-9]+, \[rbp\+\Q$index\E\*8[^\]]*\]$/m;
		die "$test: variable frame index has no call-safe capacity\n"
			if !call_safe_source($variable, 'identity', $index);
		next;
	}
	die "$test: no native survivor-property predicate selected\n";
}

print "native survivor properties: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
