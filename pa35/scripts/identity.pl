#!/usr/bin/env perl
# identity.pl OLD NEW [--out DIR] [--jobs N] [--no-work]
#
# For changes that must not change generated code (compiler efficiency
# work): builds every cppgm++ object of this repository's dev/ source with
# OLD and with NEW (build_with) and compares each object byte for byte,
# compiles the frozen workload at -O0..-O3 with both and compares, and
# reports each compiler's own work (callgrind instructions) on the workload
# at -O0 and -O3. Exits nonzero if any output differs.
#
# OLD and NEW are compiler binaries, normally g++-built seeds before and
# after the change (freeze.pl seed-L files, or copies of dev/cppgm++).
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Compare qw(compare);
use File::Find;
use File::Path qw(make_path remove_tree);
use Getopt::Long;
use PA35;

my %o = (out => "$PA35::PA35/obj/identity", jobs => PA35::default_jobs());
GetOptions(\%o, 'out=s', 'jobs=i', 'no-work') && @ARGV == 2
	or die "usage: identity.pl OLD NEW [--out DIR] [--jobs N] [--no-work]\n";
my %c = (old => abs_path($ARGV[0]), new => abs_path($ARGV[1]));
-x $c{$_} or die "not executable: $_\n" for qw(old new);
PA35::verify_workload();
remove_tree($o{out});
make_path($o{out});
my $out = abs_path($o{out});

for my $k (qw(old new)) {
	print STDERR "building dev/ with $k ...\n";
	PA35::build_with(compiler => $c{$k}, out => "$out/$k", jobs => $o{jobs});
}
my ($n, @differ) = (0);
find({ no_chdir => 1, wanted => sub {
	return unless /\.o$/;
	(my $rel = $File::Find::name) =~ s{^\Q$out/old/objs/\E}{};
	$n++;
	push @differ, $rel if compare($File::Find::name, "$out/new/objs/$rel") != 0;
} }, "$out/old/objs");
print "dev objects compared $n, differing " . scalar(@differ) . "\n";
print "  DIFFERS $_\n" for @differ;
my $frozen_diff = 0;
for my $level (qw(O0 O1 O2 O3)) {
	for my $k (qw(old new)) {
		PA35::run_or_die($c{$k}, PA35::workload_args("-$level", "$out/workload-$k-$level.o"));
	}
	my $same = compare("$out/workload-old-$level.o", "$out/workload-new-$level.o") == 0;
	$frozen_diff++ unless $same;
	print "workload -$level " . ($same ? 'same' : 'DIFFERS') . "\n";
}
unless ($o{'no-work'}) {
	my @cmds;
	for my $level (qw(O0 O3)) {
		for my $k (qw(old new)) {
			my @cmd = ('valgrind', '--tool=callgrind', "--callgrind-out-file=$out/work-$k-$level.cg", $c{$k},
				PA35::workload_args("-$level", "$out/work-$k-$level.o"));
			push @cmds, join(' ', map { "'$_'" } @cmd) . " > '$out/work-$k-$level.log' 2>&1";
		}
	}
	PA35::run_parallel(scalar @cmds, @cmds) and die "callgrind failed; see $out/work-*.log\n";
	for my $level (qw(O0 O3)) {
		my %ir;
		for my $k (qw(old new)) {
			my $log = PA35::slurp("$out/work-$k-$level.log");
			($ir{$k}) = $log =~ /refs:\s+([\d,]+)/;
			$ir{$k} =~ tr/,//d;
		}
		printf "compiler work at -%s: %s -> %s (%+.2f%%)\n", $level, PA35::fmt_m($ir{old}), PA35::fmt_m($ir{new}),
			100 * ($ir{new} - $ir{old}) / $ir{old};
	}
}
exit((@differ || $frozen_diff) ? 1 : 0);
