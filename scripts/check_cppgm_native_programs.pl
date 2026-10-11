#!/usr/bin/perl
# Run self-checking C++ fixtures through the native driver at every PA33 level.
# Each program must exit successfully and write no stdout, both when built
# directly, through object link paths, and through the supplied LowIR backend.
use strict;
use warnings;
use File::Temp qw(tempdir);
use FindBin;
use lib $FindBin::Bin;
use CppgmBatchWorker qw(collect_tests resolve_host_command run_command_capture);

die "Usage: check_cppgm_native_programs.pl <cppgm++> <test-or-directory> [lowir2native]\n"
	if @ARGV != 2 && @ARGV != 3;
my ($app, $root, $backend) = @ARGV;
my @tests = collect_tests($root, qr/\.t$/);
die "No native driver tests found under $root\n" if !@tests;
my @host = resolve_host_command($ENV{CPPGM_HOST_CXX} // $ENV{CXX} // '',
	'g++', 'clang++', 'c++');
die "Unable to resolve host C++ link driver\n" if !@host;
my $checks = 0;
my @routes = qw(direct object native);
push @routes, 'lowir' if defined $backend;
for my $test (@tests)
{
	my $directory = tempdir('cppgm-native-program-XXXXXX', TMPDIR => 1, CLEANUP => 1);
	my $stdout = "$directory/stdout";
	my $stderr = "$directory/stderr";
	for my $level (qw(-O1 -O2 -O3))
	{
		for my $route (@routes)
		{
			my $program = "$directory/$route.program";
			my $object = "$directory/$route" . ($route eq 'object' ? '.obj' : '.o');
			my $lowir = "$directory/$route.lowir";
			my @linker = $route eq 'native' ? @host : ($app, $level);
			unlink($program, $object, $lowir);
			my @commands = $route eq 'lowir'
				? ([$app, '--emit-lowir', $level, '-o', $lowir, $test],
				   [$backend, $level, '-o', $program, $lowir])
				: $route eq 'direct'
				? ([$app, $level, '-o', $program, $test])
				: ([$app, $level, '-c', '-o', $object, $test],
				   [@linker, '-o', $program, $object]);
			for my $command (@commands)
			{
				my $status = run_command_capture(cmd => $command,
					stdout => $stdout, stderr => $stderr, timeout => 60);
				if ($status != 0)
				{
					open(my $log, '<', $stderr) or die "Cannot read $stderr: $!\n";
					local $/;
					die "$test $level $route: compilation/link failed\n" . <$log>;
				}
			}
			die "$test $level $route: no executable produced\n" if !-x $program;
			my $status = run_command_capture(cmd => [$program],
				stdout => $stdout, stderr => $stderr, timeout => 10);
			die "$test $level $route: program failed (status $status)\n" if $status != 0;
			die "$test $level $route: unexpected program stdout\n" if -s $stdout;
			++$checks;
		}
	}
}
print "native driver programs: PASS ($checks/$checks)\n" unless $ENV{CPPGM_REPORT_QUIET};
