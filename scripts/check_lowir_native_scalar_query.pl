#!/usr/bin/perl

use strict;
use warnings;

use File::Copy qw(copy);
use File::Temp qw(tempdir);
use FindBin;
use lib $FindBin::Bin;

use CppgmBatchWorker qw(collect_tests run_command_capture);

# The wrapper's exact-result call becomes a tail jump (MIR `tail_jump`) at
# -O2 and above in the host exception model only: objects `cppgm++ -c`
# writes for the host toolchain (dev/src/native/select/branch_layout.cpp,
# form_tail_jumps). No dump shows that model's MIR, so the check reads the
# host object's encoding: a tail jump is a `jmp` whose branch relocation
# names the callee, an ordinary call a `call`. The course model
# (lowir2native's executables and MIR dump) keeps every call; its MIR
# shows the register choices around the calls.

sub read_file
{
	my ($path) = @_;
	open(my $fh, '<', $path) or die "Unable to read $path: $!\n";
	local $/;
	my $data = <$fh>;
	close($fh) or die "Unable to close $path: $!\n";
	return defined($data) ? $data : '';
}

sub lowir_functions
{
	my ($test, $text) = @_;
	my %result;
	while($text =~ /(function\s+\@([^\s(]+).*?\n\})
	                 (?=\n(?:[ \t]*\n)*function\s+\@|\s*\z)/sgx) {
		die "$test: duplicate LowIR function symbol $2\n"
			if exists($result{$2});
		$result{$2} = $1;
	}
	return \%result;
}

sub mir_functions
{
	my ($test, $text) = @_;
	my %result;
	while($text =~ /(^function\s+\@([^\s]+).*?)(?=^function\s+\@|\z)/msg) {
		die "$test: duplicate MIR function symbol $2\n"
			if exists($result{$2});
		$result{$2} = $1;
	}
	return \%result;
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

sub block_with
{
	my ($body, $pattern, $without) = @_;
	my @blocks = $body =~
		/(^\s+block\s+\^[^\n]+\n.*?)(?=^\s+block\s+\^|\z)/msg;
	my @matches = grep { $_ =~ $pattern &&
	                     (!defined($without) || $_ !~ $without) } @blocks;
	return if @matches != 1;
	return $matches[0];
}

if(scalar(@ARGV) != 3)
{
	die "Usage: check_lowir_native_scalar_query.pl " .
		"<lowir2native> <cppgm++> <test-or-directory>\n";
}

my ($app, $driver, $root) = @ARGV;
my @tests = collect_tests($root, qr/host-scalar-tail-query\.t$/);
die "No scalar tail-query controls found under $root\n" if !@tests;

for my $test (@tests)
{
	my $source = lowir_functions($test, read_file($test));
	my ($helper, $producer, $wrapper, $changed);
	for my $symbol (keys(%$source)) {
		my $body = $source->{$symbol};
		next if scalar(() = $body =~ /^\s*block\s+\^/mg) != 1;
		my ($value, $target) = $body =~
			/^\s+%([A-Za-z0-9_]+)\s*=\s*call i32\s+\@([^\s(]+)\([^\n]*\)\s*$/m;
		next if !defined($value) ||
		        $body !~ /^\s+store i32 %\Q$value\E,\s*%/m ||
		        $body !~ /^\s+return i32 %\Q$value\E\s*$/m;
		die "$test: control has multiple terminal call/store/return helpers\n"
			if defined($helper);
		($helper, $producer) = ($symbol, $target);
	}
	die "$test: control has no structurally identifiable slow helper\n"
		if !defined($helper) || !exists($source->{$producer});

	for my $symbol (keys(%$source)) {
		my $body = $source->{$symbol};
		next if scalar(() = $body =~ /^\s*block\s+\^/mg) != 3;
		my ($value) = $body =~
			/^\s+%([A-Za-z0-9_]+)\s*=\s*call i32\s+\@\Q$helper\E\([^\n]*\)\s*$/m;
		next if !defined($value);
		if($body =~ /^\s+return i32 %\Q$value\E\s*$/m) {
			die "$test: control has multiple exact scalar wrappers\n"
				if defined($wrapper);
			$wrapper = $symbol;
		} else {
			$changed = $symbol;
		}
	}
	die "$test: control needs exact-result and changed-result wrappers\n"
		if !defined($wrapper) || !defined($changed);

	my $directory = tempdir('native-scalar-query-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my %mir;
	for my $level (qw(O0 O1 O2 O3)) {
		my $mir_path = "$directory/$level.mir";
		my $program = "$directory/$level.program";
		my $stderr = "$directory/$level.compile.stderr";
		my $status = run_command_capture(
			cmd => [$app, "-$level", '--dump-machine-ir', $mir_path,
				'-o', $program, $test],
			stdout => "$directory/$level.compile.stdout",
			stderr => $stderr,
			timeout => 30,
		);
		die "$test: native -$level compile failed\n" . read_file($stderr)
			if $status != 0;
		my $text = read_file($mir_path);
		die "$test: course-model $level formed a tail jump\n"
			if $text =~ /^\s+tail_jump\b/m;
		$mir{$level} = mir_functions($test, $text);
		run_program($test, $program, $level, $directory);
		my $body = $mir{$level}{$wrapper} //
			die "$test: $level lost the scalar wrapper\n";
		die "$test: $level lost the ordinary scalar call/return arm\n"
			if $body !~ /^\s+call\s+\@\Q$helper\E\b/m ||
			   $body !~ /^\s+ret\s+/m;
	}

	# From -O1 the wrapper's fast arm addresses through the incoming
	# parameter register, with no copy to another home; O0 copies it.
	my $o3_wrapper = $mir{O3}{$wrapper};
	my ($incoming) = $o3_wrapper =~
		/^\s+param\s+%[A-Za-z0-9_]+\s+->\s+([a-z0-9]+)\s*:\s*ptr\s*$/m;
	die "$test: scalar wrapper has no direct pointer ABI parameter\n"
		if !defined($incoming);
	my $fast_block = block_with($o3_wrapper, qr/^\s+ret\s+/m,
		qr/^\s+call\b/m);
	die "$test: scalar wrapper has no unique call-free return block\n"
		if !defined($fast_block);
	die "$test: O3 fast arm did not keep the incoming parameter carrier\n"
		if $fast_block !~ /\[\Q$incoming\E(?:\+|\])/;
	die "$test: O3 retained an eager copy of the incoming wrapper parameter\n"
		if $o3_wrapper =~ /^\s+mov\s+(?!\Q$incoming\E\b)[a-z0-9]+,\s*\Q$incoming\E\s*$/m;

	my $o0_wrapper = $mir{O0}{$wrapper};
	my ($o0_incoming) = $o0_wrapper =~
		/^\s+param\s+%[A-Za-z0-9_]+\s+->\s+([a-z0-9]+)\s*:\s*ptr\s*$/m;
	die "$test: O0 comparison wrapper has no parameter home copy\n"
		if !defined($o0_incoming) ||
		   $o0_wrapper !~ /^\s+mov\s+(?!\Q$o0_incoming\E\b)[a-z0-9]+,\s*\Q$o0_incoming\E\s*$/m;

	my $guard = $mir{O3}{$changed} //
		die "$test: O3 lost the changed-result guard\n";
	die "$test: changed-result guard lost its ordinary call\n"
		if $guard !~ /^\s+call\s+\@\Q$helper\E\b/m;

	# The helper's call value stays in the ABI return register through
	# the store and the return.
	my $o3_helper = $mir{O3}{$helper} //
		die "$test: O3 lost the terminal slow helper\n";
	my ($return_register) = $o3_helper =~
		/^\s+return i32\s+->\s+([a-z0-9]+)\s*$/m;
	die "$test: slow helper has no scalar ABI return carrier\n"
		if !defined($return_register);
	die "$test: O3 did not retain the call value through store and return\n"
		if $o3_helper !~ /^\s+store\.i32\s+[^,]+,\s*\Q$return_register\E\s*$/m ||
		   $o3_helper !~ /^\s+ret\s+\Q$return_register\E\s*$/m;

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

		my $query = object_function($test, $code, $wrapper);
		my @calls = branch_targets($query, 'call');
		my @jumps = branch_targets($query, 'jmp');
		if($level eq 'O0' || $level eq 'O1') {
			die "$test: host $level unexpectedly formed a scalar tail jump\n"
				if @jumps;
			die "$test: host $level lost the ordinary scalar call\n"
				if scalar(@calls) != 1 || $calls[0] ne $helper;
		} else {
			die "$test: host $level did not form exactly one scalar tail jump\n"
				if scalar(@jumps) != 1 || $jumps[0] ne $helper || @calls;
		}
		# A result changed after the call is not returned at once.
		my $changed_code = object_function($test, $code, $changed);
		my @changed_calls = branch_targets($changed_code, 'call');
		die "$test: host $level transferred a call whose result is changed before return\n"
			if branch_targets($changed_code, 'jmp');
		die "$test: host $level changed-result guard lost its ordinary call\n"
			if scalar(@changed_calls) != 1 || $changed_calls[0] ne $helper;
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

print "scalar tail queries: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
