#!/usr/bin/perl

# The course's C++ sources follow the root .clang-format: every C++ file
# under dev/ (the implementation, the tools and the student scaffolds) and
# under scripts/, tracked or not yet added (ignored files are skipped).  Test
# fixtures and benchmark inputs are outside the check and are never
# reformatted: their text and line numbers are graded or measured inputs.
#
# clang-format 19 and newer format this tree identically; clang-format 18
# lays out for-loop header continuations, some member-call continuations and
# pointer-to-member declarators differently, so it is refused.  CI installs
# clang-format 21.1.8 from PyPI (`pip install clang-format==21.1.8`).  Set
# CLANG_FORMAT to choose the binary; fix a reported file with
# `clang-format -i FILE`.

use strict;
use warnings;

use Cwd qw(abs_path);
use FindBin;

my $minimum_major = 19;
my $ci_version = '21.1.8';

my $root = abs_path("$FindBin::Bin/..");
chdir $root or die "audit_clang_format: cannot enter $root: $!\n";
my $clang_format = $ENV{CLANG_FORMAT} // 'clang-format';

sub fail_setup
{
	my ($message) = @_;
	print STDERR "audit_clang_format: $message\n";
	print STDERR "audit_clang_format: CI uses clang-format $ci_version "
		. "(pip install clang-format==$ci_version); set CLANG_FORMAT to its path\n";
	exit 1;
}

fail_setup("no .clang-format at the repository root") if !-f '.clang-format';

my $version = '';
{
	no warnings 'exec';
	if (open(my $pipe, '-|', $clang_format, '--version'))
	{
		local $/;
		$version = <$pipe> // '';
		close($pipe);
	}
}
fail_setup("cannot run '$clang_format --version'") if $version !~ /clang-format version (\d+)\.(\d+)\.(\d+)/;
my ($major, $minor, $patch) = ($1, $2, $3);
fail_setup("clang-format $major.$minor.$patch is too old: versions before $minimum_major format this tree differently")
	if $major < $minimum_major;

my @files;
{
	open(my $pipe, '-|', 'git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard', '--', 'dev', 'scripts')
		or die "audit_clang_format: cannot run git ls-files: $!\n";
	local $/ = "\0";
	while (my $path = <$pipe>)
	{
		chomp $path;
		next if $path !~ /\.(?:c|cc|cpp|cxx|h|hh|hpp|hxx)\z/;
		next if -l $path || !-f $path;
		push @files, $path;
	}
	close($pipe) or die "audit_clang_format: git ls-files failed\n";
}
die "audit_clang_format: no C++ sources found under dev/ or scripts/\n" if !@files;

my @unformatted;
for my $path (@files)
{
	my $pid = fork() // die "audit_clang_format: cannot fork: $!\n";
	if ($pid == 0)
	{
		open(STDOUT, '>', '/dev/null');
		open(STDERR, '>', '/dev/null');
		exec($clang_format, '--dry-run', '--Werror', '--ferror-limit=1', $path) or exit 127;
	}
	waitpid($pid, 0);
	push @unformatted, $path if $? != 0;
}

if (@unformatted)
{
	print "Format audit: FAIL (" . scalar(@unformatted) . " of " . scalar(@files)
		. " C++ sources differ from .clang-format under clang-format $major.$minor.$patch)\n";
	print "  $_\n" for @unformatted;
	print "Fix with: $clang_format -i FILE...\n";
	exit 1;
}
print "Format audit: PASS (" . scalar(@files)
	. " C++ sources under dev/ and scripts/ match .clang-format, clang-format $major.$minor.$patch)\n";
exit 0;
