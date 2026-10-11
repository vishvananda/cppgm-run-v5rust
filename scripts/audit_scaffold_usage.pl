#!/usr/bin/perl

# The reference implementation builds on every shipped scaffold file as it
# is shipped (the files themselves stay byte-identical to the course's:
# scripts/check_scaffold_headers.sh).
#
# The shipped files are dev_support_files in scripts/export_student_repo.sh
# (the course), or the files of scripts/scaffold_headers.sha256 (this
# repository, which has no export script).
#
# Run after a build: the compiler's dependency files (obj/dev/.d/**/*.d by
# default) say which headers each implementation object was compiled from.
# A dependency file whose source no longer exists is a leftover and is
# skipped.
# - Every shipped header is a dependency of at least one implementation
#   object, except the scaffold-only files below.
# - A scaffold-only header is a dependency of no implementation object and is
#   included by at least one student scaffold (dev/*-scaffold.cpp), if the
#   tree has scaffolds.
# - The shipped test runner is the build's test-runner source.
#
# Usage: audit_scaffold_usage.pl [--root DIR] [--shipped LIST] [--deps DIR]

use strict;
use warnings;

use Cwd qw(abs_path);
use File::Find qw(find);
use FindBin;
use Getopt::Long;

my $root = abs_path("$FindBin::Bin/..");
my ($shipped_list, $deps_dir);
GetOptions('root=s' => \$root, 'shipped=s' => \$shipped_list, 'deps=s' => \$deps_dir)
	or die "usage: $0 [--root DIR] [--shipped LIST] [--deps DIR]\n";
$root = abs_path($root);
$deps_dir //= "$root/obj/dev/.d";

# Shipped files the implementation must not use, and why.
my %scaffold_only = (
	'dev/src/support/not_implemented.h' =>
		'the scaffolds\' not-implemented exit status; the exception audit forbids it in the implementation',
);
my $test_runner = 'dev/src/support/testing/test_runner.cpp';

sub read_text
{
	my ($path) = @_;
	open(my $fh, '<', $path) or die "unable to read $path: $!\n";
	local $/;
	my $text = <$fh>;
	close($fh);
	return defined($text) ? $text : '';
}

my @errors;
my @shipped;
if (defined($shipped_list))
{
	@shipped = grep { length } split /\n/, read_text($shipped_list);
}
elsif (-f "$root/scripts/export_student_repo.sh")
{
	@shipped = read_text("$root/scripts/export_student_repo.sh") =~ /^dev_support_files=\(\n(.*?)^\)/ms
		? ($1 =~ /^\s*(dev\/\S+)\s*$/mg) : ();
}
elsif (-f "$root/scripts/scaffold_headers.sha256")
{
	@shipped = read_text("$root/scripts/scaffold_headers.sha256") =~ /^[0-9a-f]{64}  (dev\/\S+)$/mg;
}
push @errors, 'no shipped scaffold files listed' if !@shipped;

# The repository path of a dependency written relative to dev/.
sub repository_path
{
	my ($dep) = @_;
	my $path = $dep =~ m{\A/} ? $dep : abs_path("$root/dev/$dep");
	return undef if !defined($path) || index($path, "$root/") != 0;
	(my $rel = $path) =~ s{\A\Q$root\E/}{};
	return $rel;
}

# Implementation dependencies, from every current object's dependency file
# (the entry objects of the student scaffolds are not built by the
# reference).
my %used_by;
my ($dep_files, $stale) = (0, 0);
if (-d $deps_dir)
{
	find({ no_chdir => 1, wanted => sub {
		return if !-f $_ || !/\.d\z/;
		my $text = read_text($_);
		# The first rule is the object's: its first prerequisite is the source.
		my ($source) = $text =~ /\A[^:\n]+:[\s\\]*(\S+)/;
		if (!defined($source) || !-f ($source =~ m{\A/} ? $source : "$root/dev/$source"))
		{
			++$stale;
			return;
		}
		++$dep_files;
		(my $object = $_) =~ s{\A\Q$deps_dir\E/}{};
		for my $dep ($text =~ /(\S+\.(?:h|cpp))\b/g)
		{
			my $rel = repository_path($dep);
			push @{ $used_by{$rel} }, $object if defined($rel);
		}
	} }, $deps_dir);
}
push @errors, "no dependency files under $deps_dir: build the tools first" if !$dep_files;

my @scaffolds = glob("$root/dev/*-scaffold.cpp");
my $scaffold_text = join("\n", map { read_text($_) } @scaffolds);
my ($checked, $only) = (0, 0);
for my $file (@shipped)
{
	push @errors, "shipped scaffold file $file is missing" if !-f "$root/$file";
	if ($file eq $test_runner)
	{
		my $sets = -f "$root/dev/frontend_source_sets.mk" ? read_text("$root/dev/frontend_source_sets.mk") : '';
		push @errors, "$file is not the build's test-runner source (FRONTEND_TEST_RUNNER_SOURCE_ID)"
			if $sets !~ /^FRONTEND_TEST_RUNNER_SOURCE_ID\s*:=\s*support\/testing\/test_runner\s*$/m;
		next;
	}
	(my $include = $file) =~ s{\Adev/src/}{};
	if ($scaffold_only{$file})
	{
		++$only;
		push @errors, "$file is scaffold-only ($scaffold_only{$file}) but implementation objects use it: " .
			join(', ', @{ $used_by{$file} }) if $used_by{$file};
		push @errors, "scaffold-only $file is included by no student scaffold"
			if @scaffolds && $scaffold_text !~ /#\s*include\s*"\Q$include\E"/;
		next;
	}
	++$checked;
	push @errors, "shipped scaffold header $file is used by no implementation object"
		if $dep_files && !$used_by{$file};
}

if (@errors)
{
	print STDERR "Scaffold usage audit: FAIL (" . scalar(@errors) . " issue(s))\n";
	print STDERR "  $_\n" for @errors;
	exit 1;
}
print "Scaffold usage audit: PASS ($checked shipped headers compiled into the implementation, " .
	"$only scaffold-only, test runner in the build; $dep_files dependency files" .
	($stale ? ", $stale left over from deleted sources skipped" : '') . ")\n";
