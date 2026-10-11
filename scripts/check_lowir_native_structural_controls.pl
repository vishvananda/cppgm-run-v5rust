#!/usr/bin/perl

use strict;
use warnings;

use File::Copy qw(copy);
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
	my ($test, $mir, $symbol) = @_;
	return $1 if $mir =~ /(^function \@\Q$symbol\E\b.*?)(?=^function \@|\z)/ms;
	die "$test: machine IR has no $symbol function\n";
}

sub frame_offset
{
	my ($body, $name) = @_;
	return $1 if $body =~ /^\s+temp \%\Q$name\E -> \[rbp([+-]\d+)\]/m;
	return undef;
}

sub named_frame_offset
{
	my ($body, $name) = @_;
	return $1 if $body =~
		/^\s+(?:temp \%|slot \$)\Q$name\E -> \[rbp([+-]\d+)\]/m;
	return undef;
}

sub block_body
{
	my ($body, $label) = @_;
	return $1 if $body =~ /(^\s+block \^\Q$label\E\s*\n.*?)(?=^\s+block \^|\z)/ms;
	return undef;
}

sub constant_register
{
	my ($test, $body, $symbol, @constants) = @_;
	my %registers;
	for my $constant (@constants) {
		my @found = $body =~ /^\s+mov ([a-z0-9]+), \Q$constant\E$/mg;
		die "$test: $symbol lost its incoming constant $constant\n" if !@found;
		$registers{$_} = 1 for @found;
	}
	die "$test: $symbol spread one phi's incoming constants over registers\n"
		if scalar(keys %registers) != 1;
	return (keys %registers)[0];
}

if (scalar(@ARGV) != 3)
{
	die "Usage: check_lowir_native_structural_controls.pl <lowir2native> <cppgm++> <test-or-directory>\n";
}

	my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root,
	qr/(?:acyclic-phi-shared-register|call-result-loop-register|conditional-fallthrough-layout|medium-copy-direct-chunks|selected-parameter-index-home|private-stage-promotion).*\.t$/);
die "No native structural-control tests found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('lowir-native-structural-XXXXXX', TMPDIR => 1,
		CLEANUP => 1);
	my $mir_path = "$directory/test.mir";
	my $program = "$directory/test.program";
	my $level = $test =~
		/private-stage-promotion/
		? '-O3' : $test =~
		/(?:conditional-fallthrough-layout|call-result-loop-register)/
		? '-O2' : '-O1';
	my $status = run_command_capture(
		cmd => [$app, $level, '--dump-machine-ir', $mir_path,
			'-o', $program, $test],
		stdout => "$directory/compile.stdout",
		stderr => "$directory/compile.stderr",
		timeout => 30,
	);
	die "$test: optimized native compile failed\n" .
		read_file("$directory/compile.stderr") if $status != 0;
	my $run_status = run_command_capture(
		cmd => [$program],
		stdout => "$directory/program.stdout",
		stderr => "$directory/program.stderr",
		timeout => 30,
	);
	die "$test: generated program failed with status $run_status\n" if $run_status != 0;

	my $mir = read_file($mir_path);
	if ($test =~ /private-stage-promotion/) {
		my %level_mir;
		for my $request (['O0', '-O0'], ['O1', '-O1'],
			['O2', '-O2'], ['O3', '-O3']) {
			my ($name, $requested_level) = @$request;
			my $level_mir = "$directory/$name.mir";
			my $level_program = "$directory/$name.program";
			$status = run_command_capture(
				cmd => [$app, $requested_level, '--dump-machine-ir',
					$level_mir, '-o', $level_program, $test],
				stdout => "$directory/$name.compile.stdout",
				stderr => "$directory/$name.compile.stderr",
				timeout => 30,
			);
			die "$test: $requested_level native compile failed\n" .
				read_file("$directory/$name.compile.stderr") if $status != 0;
			$run_status = run_command_capture(
				cmd => [$level_program],
				stdout => "$directory/$name.program.stdout",
				stderr => "$directory/$name.program.stderr",
				timeout => 30,
			);
			die "$test: $requested_level generated program failed with status " .
				"$run_status\n" if $run_status != 0;
			$level_mir{$name} = read_file($level_mir);
		}

		# Private scalar stages: v4opt promotes a slot stored once in the
		# entry block, with no address taken and no volatile access, to the
		# value it stores (at every level), so no stage keeps a frame home
		# and the guard tests the loaded carrier itself.  It does not fuse
		# the adjacent scalar copies into one wider transfer.
		for my $name (qw(O1 O2 O3)) {
			my $direct = function_body(
				$test, $level_mir{$name}, 'copy_adjacent_fields');
			die "$test: $name lost the adjacent scalar-copy baseline\n"
				if scalar(() = $direct =~ /^\s+load\.i64\b/mg) < 2 ||
				   scalar(() = $direct =~ /^\s+store\.i64\b/mg) < 2;

			my $staged = function_body(
				$test, $level_mir{$name}, 'copy_through_private_stages');
			die "$test: $name kept a frame home for a private scalar stage\n"
				if defined(named_frame_offset($staged, 'first_stage')) ||
				   defined(named_frame_offset($staged, 'second_stage')) ||
				   $staged =~ /\[rbp[+-]/;
			die "$test: $name lost the staged copy's field transfers\n"
				if scalar(() = $staged =~ /^\s+load\.i64\b/mg) < 2 ||
				   scalar(() = $staged =~ /^\s+store\.i64\b/mg) < 2;

			my $shared = function_body(
				$test, $level_mir{$name}, 'copy_shared_stage_guard');
			die "$test: $name kept a frame home for the shared guard's private stage\n"
				if defined(named_frame_offset($shared, 'second_stage'));

			my $guard = function_body(
				$test, $level_mir{$name}, 'guarded_private_stage');
			my $entry = block_body($guard, 'entry');
			my $consume = block_body($guard, 'consume');
			die "$test: $name lost the guarded-stage relationship\n"
				if !defined($entry) || !defined($consume);
			die "$test: $name kept a frame home for the guarded private stage\n"
				if defined(named_frame_offset($guard, 'stage')) ||
				   $guard =~ /\[rbp[+-]/;
			my ($carrier) = $entry =~
				/^\s+load\.ptr (\w+),[^\n]*\n\s+(?:test\.(?:i64|ptr) \1, \1|cmp\.(?:i64|ptr) \1, 0)(?: [^\n]*)?$/m;
			die "$test: $name did not compare the defining carrier directly\n"
				if !defined($carrier);
			die "$test: $name direct guard test lost its source location\n"
				if $entry !~
					/^\s+(?:test|cmp)\.(?:i64|ptr) \Q$carrier\E, (?:\Q$carrier\E|0) !dbg\(common_path\.cpp, 55, 5\)$/m;
			die "$test: $name consuming arm does not read through the defining carrier\n"
				if $consume !~ /^\s+load\.i64 \w+, \[\Q$carrier\E\](?:\s|$)/m;
		}

		# Controls, at every level: a stage read again volatilely, a
		# volatile stage and an escaping stage keep their frame homes and
		# their stores (the volatile stage also its reads).
		for my $name (qw(O0 O1 O2 O3)) {
			my $shared = function_body(
				$test, $level_mir{$name}, 'copy_shared_stage_guard');
			my $shared_home = named_frame_offset($shared, 'first_stage');
			die "$test: $name promoted a stage that is read again volatilely\n"
				if !defined($shared_home) ||
				   $shared !~ /^\s+store\.i64 \[rbp\Q$shared_home\E\], \w+$/m ||
				   $shared !~ /^\s+load\.i64 \w+, \[rbp\Q$shared_home\E\]$/m;

			for my $symbol (qw(guarded_volatile_stage guarded_escaping_stage)) {
				my $body = function_body($test, $level_mir{$name}, $symbol);
				my $guard_home = named_frame_offset($body, 'stage');
				my $guard_entry = block_body($body, 'entry');
				die "$test: $name promoted or sank an observable or escaping stage in $symbol\n"
					if !defined($guard_home) || !defined($guard_entry) ||
					   $guard_entry !~
						/^\s+store\.ptr \[rbp\Q$guard_home\E\], \w+$/m;
			}
			my $volatile = function_body(
				$test, $level_mir{$name}, 'guarded_volatile_stage');
			my $volatile_home = named_frame_offset($volatile, 'stage');
			my $volatile_entry = block_body($volatile, 'entry');
			my $volatile_consume = block_body($volatile, 'consume');
			die "$test: $name elided a volatile stage read\n"
				if !defined($volatile_consume) ||
				   $volatile_entry !~
					/^\s+store\.ptr \[rbp\Q$volatile_home\E\], \w+\n\s+load\.ptr \w+, \[rbp\Q$volatile_home\E\]$/m ||
				   $volatile_consume !~
					/^\s+load\.ptr \w+, \[rbp\Q$volatile_home\E\]$/m;
		}
		next;
	}
	if ($test =~ /selected-parameter-index-home/) {
		my %level_mir;
		for my $request (['O0', '-O0'], ['O1', '-O1'],
			['O2', '-O2'], ['O3', '-O3']) {
			my ($name, $requested_level) = @$request;
			my $level_mir = "$directory/$name.mir";
			my $level_program = "$directory/$name.program";
			$status = run_command_capture(
				cmd => [$app, $requested_level, '--dump-machine-ir',
					$level_mir, '-o', $level_program, $test],
				stdout => "$directory/$name.compile.stdout",
				stderr => "$directory/$name.compile.stderr",
				timeout => 30,
			);
			die "$test: $requested_level native compile failed\n" .
				read_file("$directory/$name.compile.stderr") if $status != 0;
			$run_status = run_command_capture(
				cmd => [$level_program],
				stdout => "$directory/$name.program.stdout",
				stderr => "$directory/$name.program.stderr",
				timeout => 30,
			);
			die "$test: $requested_level generated program failed with status " .
				"$run_status\n" if $run_status != 0;
			$level_mir{$name} = read_file($level_mir);
		}

		for my $name (qw(O1 O2 O3)) {
			my $body = function_body(
				$test, $level_mir{$name}, 'copy_then_index');
			my ($source_register) =
				$body =~ /^\s+param\s+%source\s+->\s+(\w+)\s+:\s+ptr$/m;
			die "$test: $name lost the source parameter ABI binding\n"
				if !defined($source_register);
			my ($prefix, $index_register) = $body =~
				/\A(.*?)^\s+lea\s+\w+,\s*\[(\w+)\+144\]\s*$/ms;
			die "$test: $name lost the post-copy constant-index address\n"
				if !defined($index_register);
			if ($index_register ne $source_register) {
				my $direct = $prefix =~
					/^\s+mov\s+\Q$index_register\E,\s*\Q$source_register\E\s*$/m;
				my $through_frame = 0;
				while ($prefix =~
					/^\s+store\.ptr\s+(\[rbp[^\]]*\]),\s*\Q$source_register\E\s*$/mg) {
					my $home = $1;
					if ($prefix =~
						/^\s+load\.ptr\s+\Q$index_register\E,\s*\Q$home\E\s*$/m) {
						$through_frame = 1;
						last;
					}
				}
				die "$test: $name used an uninitialized selected parameter home\n"
					if !$direct && !$through_frame;
			}
		}

		my $driver_input = "$directory/test.lowir";
		copy($test, $driver_input) or
			die "$test: unable to prepare driver replay: $!\n";
		for my $name (qw(O1 O2 O3)) {
			my $driver_program = "$directory/driver-$name.program";
			$status = run_command_capture(
				cmd => [$driver, "-$name", '-o', $driver_program,
					$driver_input],
				stdout => "$directory/driver-$name.compile.stdout",
				stderr => "$directory/driver-$name.compile.stderr",
				timeout => 30,
			);
			die "$test: $name driver replay failed\n" .
				read_file("$directory/driver-$name.compile.stderr")
				if $status != 0;
			$run_status = run_command_capture(
				cmd => [$driver_program],
				stdout => "$directory/driver-$name.program.stdout",
				stderr => "$directory/driver-$name.program.stderr",
				timeout => 30,
			);
			die "$test: $name driver output failed with status $run_status\n"
				if $run_status != 0;
		}
		next;
	}
	if ($test =~ /medium-copy-direct-chunks/) {
		my %level_mir;
		for my $request (['O0', '-O0'], ['O1', '-O1'],
			['O2', '-O2'], ['O3', '-O3']) {
			my ($name, $requested_level) = @$request;
			my $level_mir = "$directory/$name.mir";
			my $level_program = "$directory/$name.program";
			my $level_stderr = "$directory/$name.compile.stderr";
			$status = run_command_capture(
				cmd => [$app, $requested_level,
					'--dump-machine-ir', $level_mir,
					'-o', $level_program, $test],
				stdout => "$directory/$name.compile.stdout",
				stderr => $level_stderr,
				timeout => 30,
			);
			die "$test: $requested_level native compile failed\n" .
				read_file($level_stderr) if $status != 0;
			$run_status = run_command_capture(
				cmd => [$level_program],
				stdout => "$directory/$name.program.stdout",
				stderr => "$directory/$name.program.stderr",
				timeout => 30,
			);
			die "$test: $requested_level generated program failed with status " .
				"$run_status\n" if $run_status != 0;
			$level_mir{$name} = read_file($level_mir);
		}

		for my $name (qw(O0 O1 O2 O3)) {
			my $medium = function_body(
				$test, $level_mir{$name}, 'medium_copy');
			my $small = function_body(
				$test, $level_mir{$name}, 'small_copy_control');
			my $large = function_body(
				$test, $level_mir{$name}, 'large_copy_control');
			die "$test: $name medium reducer lost its weak 61-byte copy\n"
				if $medium !~ /^\s+copy_bytes 61x1, /m;
			die "$test: $name small control lost its 32-byte copy\n"
				if $small !~ /^\s+copy_bytes 32x1, /m;
			die "$test: $name large control lost its 129-byte copy\n"
				if $large !~ /^\s+copy_bytes 129x1, /m;
		}

		# The encoding is visible only in the code: v4opt copies through
		# 32 bytes directly at every level and, at O1+, every copy through
		# 128 bytes (select/bulk_policy.h); larger or (at O0) weakly
		# aligned medium copies use rep movsb.
		my $driver_input = "$directory/test.lowir";
		copy($test, $driver_input) or
			die "$test: unable to prepare driver replay: $!\n";
		my %rep_moves;
		my %chunk_stores;
		for my $name (qw(O0 O1 O2 O3)) {
			my $object = "$directory/driver-$name.o";
			$status = run_command_capture(
				cmd => [$driver, '-c', "-$name", '-o', $object,
					$driver_input],
				stdout => "$object.compile.stdout",
				stderr => "$object.compile.stderr",
				timeout => 30,
			);
			die "$test: $name driver replay failed\n" .
				read_file("$object.compile.stderr") if $status != 0;
			for my $symbol (qw(medium_copy small_copy_control
				large_copy_control)) {
				my $disassembly = "$object.$symbol.disassembly";
				$status = run_command_capture(
					cmd => ['objdump', '-d', '-Mintel',
						"--disassemble=$symbol", $object],
					stdout => $disassembly,
					stderr => "$disassembly.stderr",
					timeout => 30,
				);
				die "$test: $name driver $symbol disassembly failed\n"
					if $status != 0;
				my $code = read_file($disassembly);
				die "$test: $name driver has no $symbol code\n"
					if $code !~ /^[0-9a-f]+ <\Q$symbol\E>:$/m;
				$rep_moves{$name}{$symbol} =
					scalar(() = $code =~ /\brep\s+movs/g);
				$chunk_stores{$name}{$symbol} = scalar(() = $code =~
					/\bmov(?:dqu)?\s+(?:XMMWORD|QWORD|DWORD|WORD|BYTE) PTR \[[^\]]+\],(?:xmm\d+|r\d+[dwb]?|[re][a-z]{2}|[a-z]{2})\s*$/mg);
			}
		}
		for my $name (qw(O0 O1 O2 O3)) {
			die "$test: driver $name small control lost its direct copy\n"
				if $rep_moves{$name}{small_copy_control} != 0;
			die "$test: driver $name oversized control lost compact fallback\n"
				if $rep_moves{$name}{large_copy_control} != 1;
		}
		die "$test: driver O0 medium copy lost compact string encoding\n"
			if $rep_moves{O0}{medium_copy} != 1;
		for my $name (qw(O1 O2 O3)) {
			die "$test: driver $name medium copy retained string encoding\n"
				if $rep_moves{$name}{medium_copy} != 0;
			die "$test: driver $name medium copy used no direct chunks\n"
				if $chunk_stores{$name}{medium_copy} < 4;
		}
		next;
	}
	if ($test =~ /call-result-loop-register/) {
		# v4opt's O1+ allocator assigns values by use counts weighted with
		# block execution estimates: the loop's call result keeps a
		# call-safe register and an earlier result, read once after the
		# loop, gives way; its O0 allocator gives the loop's result a frame
		# home.
		my $future = function_body($test, $mir, 'future_claimed_region');
		die "$test: an earlier call result blocked a future cyclic result plan\n"
			if defined(frame_offset($future, 'choice'));
		my $future_later = block_body($future, 'check_one');
		die "$test: future-plan control lost its intervening call\n"
			if !defined($future_later) ||
			   $future_later !~ /^\s+call \@touch\b/m;
		die "$test: choice was not consumed after the intervening call\n"
			if $future_later !~
				/^\s+call \@touch\b.*^\s+cmp\.i64 (?:rbx|r1[2-5]), 1$/ms;

		my $baseline_mir = "$directory/baseline.mir";
		my $baseline_program = "$directory/baseline.program";
		$status = run_command_capture(
			cmd => [$app, '-O0', '--dump-machine-ir', $baseline_mir,
				'-o', $baseline_program, $test],
			stdout => "$directory/baseline-compile.stdout",
			stderr => "$directory/baseline-compile.stderr",
			timeout => 30,
		);
		die "$test: O0 native compile failed\n" .
			read_file("$directory/baseline-compile.stderr") if $status != 0;
		$run_status = run_command_capture(
			cmd => [$baseline_program],
			stdout => "$directory/baseline-program.stdout",
			stderr => "$directory/baseline-program.stderr",
			timeout => 30,
		);
		die "$test: O0 generated program failed with status $run_status\n"
			if $run_status != 0;
		my $baseline_future = function_body(
			$test, read_file($baseline_mir), 'future_claimed_region');
		my $baseline_home = frame_offset($baseline_future, 'choice');
		my $baseline_later = block_body($baseline_future, 'check_one');
		die "$test: O0 control no longer distinguishes future plan reservation\n"
			if !defined($baseline_home) || !defined($baseline_later) ||
			   $baseline_later !~
				/^\s+call \@touch\b.*^\s+cmp\.i64 \[rbp\Q$baseline_home\E\], 1$/ms;
		next;
	}
	if ($test =~ /conditional-fallthrough-layout/) {
		# v4opt drops the unreachable cold block and lays the unconditional
		# successor out next at O1+; its O0 layout keeps the jump.  (Its
		# placement leaves conditional_fallthrough's guard a jump: entry and
		# dispatch already fall through to hot and other.)
		my $trace = function_body($test, $mir, 'pure_trace');
		die "$test: O2 did not place the unconditional trace successor next\n"
			if $trace !~
				/^  block \^entry\n(?:(?:    [^\n]*)?\n)*  block \^done$/m;
		my $trace_entry = block_body($trace, 'entry');
		die "$test: O2 retained the unconditional trace jump\n"
			if !defined($trace_entry) || $trace_entry =~ /^\s+jmp\b/m;

		my $baseline_mir = "$directory/baseline.mir";
		my $baseline_program = "$directory/baseline.program";
		$status = run_command_capture(
			cmd => [$app, '-O0', '--dump-machine-ir', $baseline_mir,
				'-o', $baseline_program, $test],
			stdout => "$directory/baseline-compile.stdout",
			stderr => "$directory/baseline-compile.stderr",
			timeout => 30,
		);
		die "$test: O0 native compile failed\n" .
			read_file("$directory/baseline-compile.stderr") if $status != 0;
		$run_status = run_command_capture(
			cmd => [$baseline_program],
			stdout => "$directory/baseline-program.stdout",
			stderr => "$directory/baseline-program.stderr",
			timeout => 30,
		);
		die "$test: O0 generated program failed with status $run_status\n"
			if $run_status != 0;
		my $baseline_entry = block_body(
			function_body($test, read_file($baseline_mir), 'pure_trace'),
			'entry');
		die "$test: O0 control no longer keeps the unconditional trace jump\n"
			if !defined($baseline_entry) ||
			   $baseline_entry !~ /^\s+jmp \^done$/m;
		next;
	}

	# Acyclic phi chains: v4opt's O1+ allocator prefers a value's phi-web
	# register, so a phi and the phi it merges share one register and no
	# edge transfers the value; its O0 allocator copies on the edge.  A source still live after the merge, and a loop
	# invariant one arm of a repeated merge forwards, keep their own
	# registers at every level.
	my %phi_level_mir = (O1 => $mir);
	for my $request (['O0', '-O0'], ['O2', '-O2'], ['O3', '-O3']) {
		my ($name, $requested_level) = @$request;
		my $level_mir = "$directory/phi-$name.mir";
		my $level_program = "$directory/phi-$name.program";
		$status = run_command_capture(
			cmd => [$app, $requested_level, '--dump-machine-ir',
				$level_mir, '-o', $level_program, $test],
			stdout => "$directory/phi-$name.compile.stdout",
			stderr => "$directory/phi-$name.compile.stderr",
			timeout => 30,
		);
		die "$test: $requested_level native compile failed\n" .
			read_file("$directory/phi-$name.compile.stderr") if $status != 0;
		$run_status = run_command_capture(
			cmd => [$level_program],
			stdout => "$directory/phi-$name.program.stdout",
			stderr => "$directory/phi-$name.program.stderr",
			timeout => 30,
		);
		die "$test: $requested_level generated program failed with status " .
			"$run_status\n" if $run_status != 0;
		$phi_level_mir{$name} = read_file($level_mir);
	}

	for my $name (qw(O0 O1 O2 O3)) {
		for my $symbol (qw(single_use_chain last_transfer_chain
			repeated_local_chain)) {
			my $body = function_body($test, $phi_level_mir{$name}, $symbol);
			my $source = constant_register($test, $body, $symbol, 11, 22);
			my $merge = constant_register($test, $body, $symbol, 33);
			die "$test: $name acyclic phi chain in $symbol kept an edge transfer\n"
				if $name ne 'O0' && $source ne $merge;
			die "$test: O0 control no longer transfers on the phi edge in $symbol\n"
				if $name eq 'O0' && $source eq $merge;
		}
		my $transfer = function_body(
			$test, $phi_level_mir{$name}, 'last_transfer_chain');
		die "$test: $name final-transfer source lost its earlier observable use\n"
			if $transfer !~ /^\s+store\.i64 \@transfer_observed,/m;

		my $multi = function_body(
			$test, $phi_level_mir{$name}, 'multi_use_guard');
		die "$test: $name multi-use phi values unsafely share a register\n"
			if constant_register($test, $multi, 'multi_use_guard', 44, 55) eq
			   constant_register($test, $multi, 'multi_use_guard', 66);

		my $repeated = function_body(
			$test, $phi_level_mir{$name}, 'repeated_merge_guard');
		my $zero = block_body($repeated, 'zero_value');
		my ($zero_register) =
			defined($zero) ? ($zero =~ /^\s+mov ([a-z0-9]+), 0$/m) : ();
		die "$test: $name repeated merge lost its zero arm\n"
			if !defined($zero_register);
		die "$test: $name repeated acyclic merge unsafely overwrites its invariant\n"
			if constant_register($test, $repeated, 'repeated_merge_guard',
				7, 9) eq $zero_register;
	}
}

print "native structural controls: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
