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

# The declaration whose metadata names the runtime identity `object`, in
# whatever order its metadata is written.
sub declaration_with_object
{
	my ($lowir, $object) = @_;
	for my $line (split(/\n/, $lowir))
	{
		next if $line !~ /^declare function \@/;
		return $line if $line =~ /\[[^\]]*\bobject=\Q$object\E(?:,|\])/;
	}
	return undef;
}

if (scalar(@ARGV) != 2)
{
	die "Usage: check_pa10_lowir_controls.pl <cppgm++> <test-or-directory>\n";
}

my ($app, $root) = @ARGV;
my @tests = collect_tests(
	$root, qr/(?:pointer-difference-strength-reduction|readonly-scalar-storage|volatile-access-markers|builtin-memory-alias-boundaries|ordinary-pointer-decay).*\.cpp$/);
die "No PA10 focused LowIR controls found under $root\n" if !@tests;

for my $test (@tests)
{
	my $directory = tempdir('pa10-lowir-control-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my $output = "$directory/test.lowir";
	my $status = run_command_capture(
		cmd => [$app, '--emit-lowir', '-O0', '-o', $output, $test],
		stdout => "$directory/compile.stdout",
		stderr => "$directory/compile.stderr",
		timeout => 60,
	);
	die "$test: source-to-LowIR compile failed\n" .
		read_file("$directory/compile.stderr") if $status != 0;

	my $lowir = read_file($output);
	if ($test =~ /pointer-difference-strength-reduction/) {
		# The difference of two pointers into one array is a multiple of
		# the element size (5.7p6), stated as `exact`: an exact right shift
		# by 3 and an exact division by 8 are the same division.
		my $power_two = function_body(
			$test, $lowir, 'power_two_difference');
		die "$test: power-of-two pointer difference did not divide the " .
			"byte difference exactly by the element size (shr exact by 3 " .
			"or div exact by 8)\n"
			if $power_two !~ /^\s+%\w+ = binary (?:shr exact i64 %\w+, 3|div exact i64 %\w+, 8)$/m;

		my $general = function_body($test, $lowir, 'general_difference');
		die "$test: non-power-of-two pointer difference did not divide " .
			"the byte difference exactly by its element size\n"
			if $general !~ /^\s+%\w+ = binary div exact i64 %\w+, 3$/m;
		next;
	}
	if ($test =~ /readonly-scalar-storage/) {
		my ($readonly) = $lowir =~
			/^(global \@[^\n]+ : u32 \[[^\n]*\] = 9)$/m;
		die "$test: scalar const definition with value 9 is missing\n"
			if !defined($readonly);
		die "$test: eligible scalar const is not marked readonly\n"
			if $readonly !~ /\bstorage=readonly\b/;
		my ($volatile, $volatile_name) = $lowir =~
			/^(global \@(\S+) : u32 \[[^\n]*\] = 7)$/m;
		die "$test: volatile scalar definition is missing\n"
			if !defined($volatile);
		die "$test: volatile scalar was incorrectly marked readonly\n"
			if $volatile =~ /\bstorage=readonly\b/;
		die "$test: volatile scalar read lost its access marker\n"
			if $lowir !~ /^\s+%\w+ = load volatile u32 \@\Q$volatile_name\E$/m;
		my ($thread) = $lowir =~
			/^(global \@[^\n]+ : u32 \[[^\n]*\] = 5)$/m;
		die "$test: thread-local scalar definition is missing\n"
			if !defined($thread);
		die "$test: thread-local scalar lost its storage class or became readonly\n"
			if $thread !~ /\bstorage=thread_local\b/ ||
			   $thread =~ /\bstorage=readonly\b/;
		my ($object) = $lowir =~
			/^(global \@[^\n]+ \[[^\n]*\] = \{\n\s+u32 3\n\})/m;
		die "$test: const class object definition is missing\n"
			if !defined($object);
		die "$test: statically initialized const class object is not marked readonly\n"
			if $object !~ /\bstorage=readonly\b/;
		next;
	}
	if ($test =~ /volatile-access-markers/) {
		my $local = function_body($test, $lowir, 'local_access');
		die "$test: volatile local stores or load lost their marker\n"
			if scalar(() = $local =~ /^\s+store volatile i32 /mg) < 2 ||
			   $local !~ /^\s+%\w+ = load volatile i32 /m;
		my $pointer = function_body($test, $lowir, 'pointer_access');
		die "$test: volatile pointer dereference lost its load marker\n"
			if $pointer !~ /^\s+%\w+ = load volatile i32 %\w+$/m;
		my $member = function_body($test, $lowir, 'member_access');
		die "$test: volatile member read or write lost its marker\n"
			if $member !~ /^\s+%\w+ = load volatile i32 %\w+$/m ||
			   $member !~ /^\s+store volatile i32 [^,]+, %\w+$/m;
		die "$test: ordinary neighboring member store became volatile\n"
			if $member !~ /^\s+store typed i32 [^,]+, %\w+$/m;
		next;
	}
	if ($test =~ /builtin-memory-alias-boundaries/) {
		# The memory builtins are declared with their runtime identities
		# (pa8/lowir.md, "Facts The C++ Lowering States"): three ordinary
		# pointer and size parameters, non-throwing, and `alias=noalias` on
		# memcpy's pointers alone.
		my $memcpy = declaration_with_object($lowir, 'cppgm_builtin_memcpy');
		die "$test: generated LowIR has no declaration with object=cppgm_builtin_memcpy\n"
			if !defined($memcpy);
		die "$test: memcpy declaration is not (ptr [alias=noalias], ptr [alias=noalias], i64) -> ptr\n"
			if $memcpy !~ /\(%\S+ : ptr \[[^\]]*\balias=noalias\b[^\]]*\], %\S+ : ptr \[[^\]]*\balias=noalias\b[^\]]*\], %\S+ : i64\) -> ptr\b/;
		die "$test: memcpy declaration does not state unwind=no\n"
			if $memcpy !~ /\bunwind=no\b/;
		my $memmove = declaration_with_object($lowir, 'cppgm_builtin_memmove');
		die "$test: generated LowIR has no declaration with object=cppgm_builtin_memmove\n"
			if !defined($memmove);
		die "$test: memmove declaration is not (ptr, ptr, i64) -> ptr\n"
			if $memmove !~ /\(%\S+ : ptr(?: \[[^\]]*\])?, %\S+ : ptr(?: \[[^\]]*\])?, %\S+ : i64\) -> ptr\b/;
		die "$test: memmove declaration does not state unwind=no\n"
			if $memmove !~ /\bunwind=no\b/;
		die "$test: generated LowIR states a parameter key the format does not define (capture= or access=)\n"
			if $lowir =~ /\b(?:capture|access)=/;
		die "$test: potentially overlapping memmove was marked noalias\n"
			if $memmove =~ /\balias=noalias\b/;
		my $copy = function_body($test, $lowir, 'copy_bytes');
		die "$test: source call did not use the ordinary memcpy boundary\n"
			if $copy !~
				/^\s+%\w+ = call ptr \@\S+\(%\w+, %\w+, %\w+\)$/m;
		my $move = function_body($test, $lowir, 'move_bytes');
		die "$test: source call did not use the ordinary memmove boundary\n"
			if $move !~
				/^\s+%\w+ = call ptr \@\S+\(%\w+, %\w+, %\w+\)$/m;
		next;
	}
	if ($test =~ /ordinary-pointer-decay/) {
		die "$test: generated LowIR states a decay operation the format does not define\n"
			if $lowir =~ /\bunary\s+decay\b|\bpass=decay\b/;
		my $array = function_body($test, $lowir, 'array_decay');
		die "$test: array decay did not produce an ordinary pointer address\n"
			if $array !~ /^\s+%\w+ = addr \@\w+$/m;
		die "$test: subscript after array decay did not use pointer indexing\n"
			if $array !~ /^\s+%\w+ = index i32(?: \[projection=array_element\])? %\w+, 2$/m;
		my $function = function_body($test, $lowir, 'function_decay');
		die "$test: function decay did not produce an ordinary function address\n"
			if $function !~ /^\s+%\w+ = addr \@\w+$/m;
		die "$test: function pointer produced by decay was not callable\n"
			if $function !~ /^\s+%\w+ = call i32 %\w+\(4\) as \(/m;
		next;
	}
	die "$test: no PA10 focused predicate selected\n";
}

print "PA10 focused LowIR controls: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
