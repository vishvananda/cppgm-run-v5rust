#!/usr/bin/perl

# Compile every standard header the compiler's own sources include, one per
# translation unit, and report which ones the compiler under test accepts.
#
# Whole-program builds stop at the first failure, which hides how much of the
# standard library is actually reachable.  One header per translation unit
# turns that into an inventory: a count that moves as fixes land, and a
# grouping by first diagnostic that says how many distinct causes are left.

use strict;
use warnings;

use File::Path qw(make_path);
use File::Temp qw(tempdir);
use Getopt::Long;

my $source_root = "dev/src";
my $standard = "gnu++11";
my $out_path;
my $timeout = 120;

GetOptions(
	"source-root=s" => \$source_root,
	"std=s" => \$standard,
	"out=s" => \$out_path,
	"timeout=i" => \$timeout,
) or die "usage: probe_hosted_headers.pl [options] <compiler>\n";

my $compiler = shift(@ARGV);
die "usage: probe_hosted_headers.pl [options] <compiler>\n" if !defined($compiler);
die "compiler is not executable: $compiler\n" if !-x $compiler;

sub collect_headers
{
	my ($root) = @_;
	my %seen;
	my @sources;
	my @queue = ($root);
	while(my $dir = shift(@queue))
	{
		opendir(my $dh, $dir) or die "cannot read $dir: $!\n";
		for my $entry (sort readdir($dh))
		{
			next if $entry eq "." || $entry eq "..";
			my $path = "$dir/$entry";
			if(-d $path) { push(@queue, $path); next; }
			push(@sources, $path) if $path =~ /\.(cpp|h)$/;
		}
		closedir($dh);
	}
	for my $source (@sources)
	{
		open(my $fh, "<", $source) or next;
		while(my $line = <$fh>)
		{
			$seen{$1} = 1 if $line =~ /^\s*#\s*include\s*<([a-z_]+)>/;
		}
		close($fh);
	}
	return sort keys %seen;
}

# Only the first diagnostic matters: everything after it may be fallout from
# the same cause, and the point of the inventory is to count causes.
sub first_diagnostic
{
	my ($compiler, $standard, $header, $work, $timeout) = @_;
	my $source = "$work/probe.cpp";
	open(my $fh, ">", $source) or die "cannot write $source: $!\n";
	print {$fh} "#include <$header>\nint main() { return 0; }\n";
	close($fh);

	my $log = "$work/probe.log";
	my $pid = fork();
	die "fork failed: $!\n" if !defined($pid);
	if($pid == 0)
	{
		open(STDOUT, ">", $log) or exit(127);
		open(STDERR, ">&", \*STDOUT) or exit(127);
		exec($compiler, "-std=$standard", "-c", "-o", "$work/probe.o", $source);
		exit(127);
	}

	my $status;
	eval {
		local $SIG{ALRM} = sub { die "timeout\n" };
		alarm($timeout);
		waitpid($pid, 0);
		$status = $?;
		alarm(0);
		1;
	} or do {
		alarm(0);
		kill("KILL", $pid);
		waitpid($pid, 0);
		return (1, "TIMEOUT after ${timeout}s");
	};

	my $text = "";
	if(open(my $lh, "<", $log)) { local $/; $text = <$lh> // ""; close($lh); }
	unlink($source, $log, "$work/probe.o");
	return (0, "") if $status == 0;
	my ($first) = split(/\n/, $text);
	$first = "exited $status with no diagnostic" if !defined($first) || $first eq "";
	return (1, $first);
}

# Strip the parts that vary between hosts so the same cause groups together.
sub cause_of
{
	my ($diagnostic) = @_;
	my $cause = $diagnostic;
	$cause =~ s/^ERROR:\s*//;
	$cause =~ s/ at token \d+.*$//;
	$cause =~ s{/[^ ]*/include/c\+\+/v1/}{libc++/}g;
	$cause =~ s{/[^ ]*/include/c\+\+/[0-9.]+/}{libstdc++/}g;
	return $cause;
}

my @headers = collect_headers($source_root);
die "no standard headers found under $source_root\n" if !@headers;

my $work = tempdir(CLEANUP => 1);
my @lines;
my %causes;
my $passed = 0;
for my $header (@headers)
{
	my ($failed, $diagnostic) =
		first_diagnostic($compiler, $standard, $header, $work, $timeout);
	if(!$failed)
	{
		++$passed;
		push(@lines, sprintf("%-16s OK", $header));
		next;
	}
	my $cause = cause_of($diagnostic);
	push(@{$causes{$cause}}, $header);
	push(@lines, sprintf("%-16s %s", $header, $diagnostic));
}

my @report;
push(@report, "compiler: $compiler");
push(@report, "standard: $standard");
push(@report, "headers:  $passed/" . scalar(@headers) . " accepted");
push(@report, "");
push(@report, @lines);
if(%causes)
{
	push(@report, "");
	push(@report, "distinct causes: " . scalar(keys %causes));
	for my $cause (sort { scalar(@{$causes{$b}}) <=> scalar(@{$causes{$a}}) || $a cmp $b }
		keys %causes)
	{
		push(@report, sprintf("  %2d  %s", scalar(@{$causes{$cause}}), $cause));
		push(@report, "      " . join(" ", @{$causes{$cause}}));
	}
}
my $text = join("\n", @report) . "\n";

if(defined($out_path))
{
	open(my $oh, ">", $out_path) or die "cannot write $out_path: $!\n";
	print {$oh} $text;
	close($oh);
}
print $text;
exit($passed == scalar(@headers) ? 0 : 1);
