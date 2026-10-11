#!/usr/bin/env perl
use strict;
use warnings;

use Cwd qw(abs_path);
use File::Basename qw(dirname);
use File::Copy qw(copy);
use File::Path qw(make_path remove_tree);
use FindBin;

my $repo_root = abs_path("$FindBin::Bin/..")
  or die "unable to resolve repository root\n";
my $ref_dir = "$repo_root/reference-binaries";
my $manifest_path = "$ref_dir/manifest.tsv";

sub usage {
  print STDERR "usage: scripts/ensure_reference_binaries.pl [binary ...]\n";
  exit 2;
}

usage() if @ARGV && grep { $_ eq "-h" || $_ eq "--help" } @ARGV;

sub read_manifest {
  open my $fh, "<", $manifest_path
    or die "missing reference binary manifest: $manifest_path\n";

  my %manifest = (binary => {});
  while (my $line = <$fh>) {
    chomp $line;
    next if $line =~ /^\s*$/ || $line =~ /^\s*#/;
    my @fields = split /\t/, $line;
    my $kind = shift @fields;
    if ($kind eq "binary") {
      die "invalid binary manifest row: $line\n" unless @fields == 2;
      my ($name, $sha256) = @fields;
      die "invalid binary name in manifest: $name\n"
        unless $name =~ /\A[A-Za-z0-9_.+-]+\z/;
      $manifest{binary}{$name} = $sha256;
    } else {
      die "invalid manifest row: $line\n" unless @fields == 1;
      $manifest{$kind} = $fields[0];
    }
  }

  for my $key (qw(version platform bundle_name bundle_url bundle_sha256)) {
    die "manifest missing $key\n" unless defined $manifest{$key};
  }
  die "unsupported reference manifest version: $manifest{version}\n"
    unless $manifest{version} eq "1";
  die "manifest has no binaries\n" unless keys %{$manifest{binary}};
  return \%manifest;
}

sub find_tool {
  my ($tool) = @_;
  for my $dir (split /:/, $ENV{PATH} || "") {
    my $path = "$dir/$tool";
    return $path if -x $path;
  }
  return undef;
}

sub run_checked {
  my (@cmd) = @_;
  system @cmd;
  my $status = $?;
  return if $status == 0;
  die "command failed: @cmd\n";
}

sub sha256_file {
  my ($path) = @_;
  open my $fh, "-|", "sha256sum", $path
    or die "unable to run sha256sum for $path: $!\n";
  my $line = <$fh>;
  close $fh or die "sha256sum failed for $path\n";
  die "sha256sum produced no output for $path\n" unless defined $line;
  $line =~ /^([0-9a-fA-F]{64})\b/
    or die "invalid sha256sum output for $path: $line\n";
  return lc $1;
}

sub binary_is_current {
  my ($name, $sha256) = @_;
  my $path = "$ref_dir/$name";
  return 0 unless -x $path;
  return sha256_file($path) eq lc $sha256;
}

sub download_bundle {
  my ($manifest, $bundle_path) = @_;

  if (defined $ENV{CPPGM_REFERENCE_BUNDLE_FILE} &&
      $ENV{CPPGM_REFERENCE_BUNDLE_FILE} ne "") {
    my $source = $ENV{CPPGM_REFERENCE_BUNDLE_FILE};
    die "CPPGM_REFERENCE_BUNDLE_FILE does not exist: $source\n"
      unless -f $source;
    copy($source, $bundle_path)
      or die "unable to copy $source to $bundle_path: $!\n";
    return;
  }

  my $url = $ENV{CPPGM_REFERENCE_BUNDLE_URL} || $manifest->{bundle_url};
  if (my $curl = find_tool("curl")) {
    run_checked($curl, "-fL", "--retry", "3", "-o", $bundle_path, $url);
    return;
  }
  if (my $wget = find_tool("wget")) {
    run_checked($wget, "-O", $bundle_path, $url);
    return;
  }

  die "reference binaries are missing and neither curl nor wget is available\n" .
      "Install curl or wget, then rerun the same command.\n" .
      "Bundle URL: $url\n";
}

sub install_bundle {
  my ($manifest) = @_;

  die "sha256sum is required to verify reference binaries\n"
    unless find_tool("sha256sum");
  die "tar is required to extract reference binaries\n"
    unless find_tool("tar");

  make_path($ref_dir);
  my $work = "$ref_dir/.download.$$";
  remove_tree($work) if -e $work;
  make_path($work);

  my $bundle_path = "$work/$manifest->{bundle_name}";
  eval {
    download_bundle($manifest, $bundle_path);
    my $bundle_sha = sha256_file($bundle_path);
    die "reference bundle sha256 mismatch\n" .
        "expected: $manifest->{bundle_sha256}\n" .
        "actual:   $bundle_sha\n"
      unless $bundle_sha eq lc $manifest->{bundle_sha256};

    my $extract_dir = "$work/extract";
    make_path($extract_dir);
    run_checked("tar", "-xzf", $bundle_path, "-C", $extract_dir);

    for my $name (sort keys %{$manifest->{binary}}) {
      my $source = "$extract_dir/$name";
      die "reference bundle missing $name\n" unless -f $source;
      chmod 0755, $source;
      my $sha = sha256_file($source);
      die "reference binary $name sha256 mismatch\n" .
          "expected: $manifest->{binary}{$name}\n" .
          "actual:   $sha\n"
        unless $sha eq lc $manifest->{binary}{$name};

      my $tmp = "$ref_dir/$name.tmp.$$";
      copy($source, $tmp)
        or die "unable to install $name: $!\n";
      chmod 0755, $tmp;
      rename $tmp, "$ref_dir/$name"
        or die "unable to replace $ref_dir/$name: $!\n";
    }
  };
  my $err = $@;
  remove_tree($work);
  die $err if $err;
}

my $manifest = read_manifest();
my @requested = @ARGV ? @ARGV : sort keys %{$manifest->{binary}};
for my $name (@requested) {
  die "unknown reference binary: $name\n"
    unless exists $manifest->{binary}{$name};
}

my @missing = grep {
  !binary_is_current($_, $manifest->{binary}{$_})
} @requested;

if (@missing) {
  print STDERR "Fetching cppgm reference binaries...\n";
  install_bundle($manifest);
}

for my $name (@requested) {
  die "failed to install reference binary: $name\n"
    unless binary_is_current($name, $manifest->{binary}{$name});
}

exit 0;
