#!/usr/bin/perl

use strict;
use warnings;

use File::Copy qw(copy);
use File::Temp qw(tempdir);
use FindBin;
use lib $FindBin::Bin;

use CppgmBatchWorker qw(collect_tests run_command_capture);

# A call a function returns from at once becomes a tail jump (MIR
# `tail_jump`: the epilogue, then a jump to the callee) at -O2 and above,
# in the host exception model only: objects `cppgm++ -c` writes for the
# host toolchain (dev/src/native/select/branch_layout.cpp,
# form_tail_jumps). No dump shows that model's MIR, so the check reads the
# host object's encoding: a tail jump is a `jmp` whose branch relocation
# names the callee, an ordinary call a `call`. The course model
# (lowir2native's executables and MIR dump) keeps every call.

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

sub object_function
{
	my ($test, $code, $symbol) = @_;
	return $1 if $code =~
		/(^[0-9a-f]+ <\@?\Q$symbol\E>:\n.*?)(?=^[0-9a-f]+ <|\z)/ms;
	die "$test: host object has no $symbol function\n";
}

# The direct callees of one branch mnemonic (`call` or `jmp`): the symbol
# of the branch relocation objdump -dr prints under the instruction.
sub branch_targets
{
	my ($code, $mnemonic) = @_;
	my @targets;
	while($code =~ /^\s+[0-9a-f]+:\s+\Q$mnemonic\E\s[^\n]*\n
	                \s+[0-9a-f]+:\s+R_X86_64_PLT32\s+\@?([^\s+-]+)/mgx) {
		push(@targets, $1);
	}
	return @targets;
}

sub run_program
{
	my ($test, $program, $label, $directory) = @_;
	my $status = run_command_capture(
		cmd => [$program],
		stdout => "$directory/$label.run.stdout",
		stderr => "$directory/$label.run.stderr",
		timeout => 30,
	);
	die "$test: $label behavior failed with status $status\n"
		if $status != 0;
}

if(scalar(@ARGV) != 3)
{
	die "Usage: check_lowir_native_tail_jump.pl " .
		"<lowir2native> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/host-tail-jump-transfer\.t$/);
die "No host tail-jump controls found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('native-tail-jump-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	for my $level (qw(O0 O1 O2 O3)) {
		my $mir_path = "$directory/$level.mir";
		my $program = "$directory/$level.program";
		my $status = run_command_capture(
			cmd => [$app, "-$level", '--dump-machine-ir', $mir_path,
				'-o', $program, $test],
			stdout => "$directory/$level.compile.stdout",
			stderr => "$directory/$level.compile.stderr",
			timeout => 30,
		);
		die "$test: native -$level compile failed\n" .
			read_file("$directory/$level.compile.stderr") if $status != 0;
		run_program($test, $program, $level, $directory);
		my $mir = read_file($mir_path);
		die "$test: course-model $level formed a tail jump\n"
			if $mir =~ /^\s+tail_jump\b/m;
		my $body = function_body($test, $mir, 'tail_transfer');
		die "$test: $level lost the ordinary direct call/return path\n"
			if $body !~ /^\s+call\s+\@record\b/m ||
			   $body !~ /^\s+ret\b/m;
	}

	my $driver_input = "$directory/driver.lowir";
	copy($test, $driver_input) or
		die "$test: unable to prepare cppgm++ replay: $!\n";
	for my $level (qw(O0 O1 O2 O3)) {
		my $object = "$directory/host-$level.o";
		my $status = run_command_capture(
			cmd => [$driver, '-c', "-$level", '-o', $object, $driver_input],
			stdout => "$directory/host-$level.compile.stdout",
			stderr => "$directory/host-$level.compile.stderr",
			timeout => 60,
		);
		die "$test: host -$level object compile failed\n" .
			read_file("$directory/host-$level.compile.stderr")
			if $status != 0;
		my $program = "$directory/host-$level.program";
		$status = run_command_capture(
			cmd => [$driver, '-o', $program, $object],
			stdout => "$directory/host-$level.link.stdout",
			stderr => "$directory/host-$level.link.stderr",
			timeout => 60,
		);
		die "$test: host -$level link failed\n" .
			read_file("$directory/host-$level.link.stderr") if $status != 0;
		run_program($test, $program, "host-$level", $directory);
		my $disassembly = "$directory/host-$level.disassembly";
		$status = run_command_capture(
			cmd => ['objdump', '-dr', '-Mintel', '--no-show-raw-insn',
				$object],
			stdout => $disassembly,
			stderr => "$disassembly.stderr",
			timeout => 30,
		);
		die "$test: host -$level disassembly failed\n" if $status != 0;
		my $code = read_file($disassembly);

		my $tail = object_function($test, $code, 'tail_transfer');
		my @calls = branch_targets($tail, 'call');
		my @jumps = branch_targets($tail, 'jmp');
		if($level eq 'O0' || $level eq 'O1') {
			die "$test: host $level unexpectedly formed a tail jump\n"
				if @jumps;
			die "$test: host $level lost the ordinary direct call\n"
				if scalar(@calls) != 1 || $calls[0] ne 'record';
		} else {
			die "$test: host $level did not form exactly one direct tail jump\n"
				if scalar(@jumps) != 1 || $jumps[0] ne 'record' || @calls;
			# The arguments need not be the incoming parameters: their
			# registers are set before the jump.
			my $changed = object_function($test, $code, 'changed_argument');
			my @changed_jumps = branch_targets($changed, 'jmp');
			die "$test: host $level kept the call with changed arguments\n"
				if scalar(@changed_jumps) != 1 ||
				   $changed_jumps[0] ne 'record' ||
				   branch_targets($changed, 'call');
		}
		# The callee receives the address of the caller's slot, so the
		# frame must outlive the call: an ordinary call at every level.
		my $guard = object_function($test, $code, 'local_storage');
		my @guard_calls = branch_targets($guard, 'call');
		die "$test: host $level tail-jumped a callee that can reach the frame\n"
			if branch_targets($guard, 'jmp');
		die "$test: host $level lost the guarded ordinary call in local_storage\n"
			if scalar(@guard_calls) != 1 || $guard_calls[0] ne 'record_slot';
	}

	my $driver_program = "$directory/driver.program";
	my $status = run_command_capture(
		cmd => [$driver, '-O3', '-o', $driver_program, $driver_input],
		stdout => "$directory/driver.compile.stdout",
		stderr => "$directory/driver.compile.stderr",
		timeout => 60,
	);
	die "$test: O3 cppgm++ replay failed\n" .
		read_file("$directory/driver.compile.stderr") if $status != 0;
	run_program($test, $driver_program, 'driver', $directory);
}

print "host tail-jump transfer: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
