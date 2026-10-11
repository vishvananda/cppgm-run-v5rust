#!/usr/bin/perl

use strict;
use warnings;

use File::Basename qw(basename dirname);
use File::Spec;
use Text::ParseWords qw(shellwords);
use Getopt::Long qw(GetOptions);

sub usage
{
	die "Usage: write_unresolved_symbol_report.pl --output <file> [--nm <nm>] [--host-cxx <cmd>] --referencer <file>... --provider <file>...\n";
}

sub normalize_path_list
{
	my (@paths) = @_;
	my %seen;
	return grep { defined($_) && $_ ne '' && -e $_ && !$seen{$_}++ } @paths;
}

sub find_in_path
{
	my ($tool) = @_;
	return undef if !defined($tool) || $tool eq '';
	if ($tool =~ m{/})
	{
		return $tool if -x $tool;
		return undef;
	}

	for my $dir (split(/:/, $ENV{PATH} || ''))
	{
		my $candidate = File::Spec->catfile($dir, $tool);
		return $candidate if -x $candidate;
	}
	return undef;
}

sub resolve_nm
{
	my ($explicit_nm, $host_cxx_cmd) = @_;
	return $explicit_nm if defined($explicit_nm) && $explicit_nm ne '' && -x $explicit_nm;
	if (defined($ENV{CPPGM_HOST_NM}) && $ENV{CPPGM_HOST_NM} ne '')
	{
		my $env_nm = find_in_path($ENV{CPPGM_HOST_NM});
		return $env_nm if defined($env_nm);
	}

	my @compiler_words = shellwords($host_cxx_cmd || '');
	if (scalar(@compiler_words) != 0)
	{
		my $compiler = find_in_path($compiler_words[0]);
		if (defined($compiler))
		{
			my $bindir = dirname($compiler);
			for my $candidate (
				File::Spec->catfile($bindir, 'llvm-nm'),
				File::Spec->catfile($bindir, 'gcc-nm'),
				File::Spec->catfile($bindir, 'nm'))
			{
				return $candidate if -x $candidate;
			}
		}
	}

	for my $tool ('llvm-nm', 'gcc-nm', 'nm')
	{
		my $candidate = find_in_path($tool);
		return $candidate if defined($candidate);
	}

	return undef;
}

sub collect_symbols
{
	my ($nm, $defined_only, @files) = @_;
	return () if scalar(@files) == 0;

	my @candidates = (
		[$nm, '-j', ($defined_only ? '--defined-only' : '-u'), @files],
	);
	if (basename($nm) eq 'nm')
	{
		push @candidates, [$nm, ($defined_only ? '-U' : '-u'), '-j', @files];
	}

	for my $cmd (@candidates)
	{
		my %symbols;
		my $ok = open(my $fh, '-|', @{$cmd});
		next if !$ok;
		while (my $line = <$fh>)
		{
			chomp($line);
			$line =~ s/^\s+//;
			$line =~ s/\s+$//;
			next if $line eq '';
			$symbols{$line} = 1;
		}
		if (close($fh))
		{
			return %symbols;
		}
	}

	die "$nm failed while reading symbols\n";
}

my $output = '';
my $explicit_nm = '';
my $host_cxx = $ENV{CPPGM_HOST_CXX} || $ENV{CXX} || 'c++';
my @referencers;
my @providers;

GetOptions(
	'output=s' => \$output,
	'nm=s' => \$explicit_nm,
	'host-cxx=s' => \$host_cxx,
	'referencer=s@' => \@referencers,
	'provider=s@' => \@providers,
) or usage();

usage() if $output eq '';

@referencers = normalize_path_list(@referencers);
@providers = normalize_path_list(@providers);
usage() if scalar(@referencers) == 0;
usage() if scalar(@providers) == 0;

my $nm = resolve_nm($explicit_nm, $host_cxx);
die "Unable to find llvm-nm/gcc-nm/nm for host linker diagnostics\n" if !defined($nm);

my %undefined = collect_symbols($nm, 0, @referencers);
my %defined = collect_symbols($nm, 1, @providers);

my @missing = sort grep { !exists($defined{$_}) } keys %undefined;

open(my $out, '>', $output) or die "Unable to write $output: $!";
print {$out} "still-missing-symbols: " . scalar(@missing) . "\n";
if (scalar(@missing) != 0)
{
	print {$out} "  $_\n" for @missing;
}
else
{
	print {$out} "  (none)\n";
}
close($out) or die "Unable to close $output: $!";
