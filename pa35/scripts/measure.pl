#!/usr/bin/env perl
# measure.pl --seed A --self B --out DIR [--levels O0,O3] [--sim]
#            [--counter callgrind|perf|auto] [--labels seed,self] [--top N]
#
# Runs both compilers on the frozen workload at each level (all runs in
# parallel), checks that their objects are byte-identical, and writes to DIR:
#   summary.txt / summary.tsv   instructions (own binary and total), calls,
#                               ratios; with --sim the cycle proxy and
#                               cache/branch events
#   <level>-mix.txt             dynamic instruction mix of each binary's own code
#   <level>-calls.txt           callees by executed-call excess (B - A)
#   <level>-functions.txt       functions by own-instruction excess (B - A)
#
# --counter callgrind (default) gives exact counts and the attribution files.
# --counter perf uses user-mode hardware counters (mean of three runs): near
# native speed, total instructions only, no attribution. auto picks perf when
# the machine has counters. Results are cached by binary hash, toolchain
# (libstdc++, libc and valgrind versions), level and counter in
# pa35/obj/cache, so an unchanged binary is never measured twice; --no-cache
# measures again and replaces the cached result. Each run is stopped after
# --timeout seconds (default 3600).
#
# A and B are normally seed-L and self-L from freeze.pl, but any two
# compilers that produce identical objects work, for example two builds of
# one source by an old and a new compiler (constant-work comparison).
use strict;
use warnings;
use FindBin;
use lib $FindBin::Bin;
use Cwd qw(abs_path);
use File::Compare qw(compare);
use File::Copy qw(copy);
use File::Path qw(make_path);
use Getopt::Long;
use PA35;

my %a = (levels => 'O0,O3', top => 40, counter => 'callgrind', labels => 'seed,self', timeout => 3600);
GetOptions(\%a, 'seed=s', 'self=s', 'out=s', 'levels=s', 'sim', 'top=i', 'counter=s', 'labels=s', 'timeout=i', 'no-cache')
	&& $a{seed} && $a{self} && $a{out} && $a{counter} =~ /^(callgrind|perf|auto)$/
	or die "usage: measure.pl --seed A --self B --out DIR [--levels O0,O3] [--sim] [--counter callgrind|perf|auto] [--no-cache]\n";
$a{counter} = PA35::perf_available() ? 'perf' : 'callgrind' if $a{counter} eq 'auto';
die "--sim needs --counter callgrind\n" if $a{sim} && $a{counter} ne 'callgrind';
PA35::verify_workload();
my %bin = (seed => abs_path($a{seed}), self => abs_path($a{self}));
-x $bin{$_} or die "not executable: $a{$_}\n" for qw(seed self);
make_path($a{out});
my $out = abs_path($a{out});
my @levels = split /,/, $a{levels};
my $kind = $a{counter} eq 'perf' ? 'perf' : $a{sim} ? 'sim' : 'cg';
my ($LA, $LB) = split /,/, $a{labels};

# One run per (level, side), skipped when the cache already holds it.
my (%cache, @commands);
for my $level (@levels) {
	for my $side (qw(seed self)) {
		my $dir = PA35::cache_dir($bin{$side}, $level, $kind);
		$cache{$level}{$side} = $dir;
		unlink "$dir/done" if $a{'no-cache'};
		next if -f "$dir/done";
		# Run a copy kept in the cache entry, so the profile names a path
		# that stays valid wherever the binary is later found.
		copy($bin{$side}, "$dir/bin") or die "copy: $!\n";
		chmod 0755, "$dir/bin";
		my $result = $kind eq 'perf' ? "$dir/perf.txt" : "$dir/run.cg";
		my @cmd = $kind eq 'perf'
			? PA35::perf_command(out => $result, argv => ["$dir/bin", PA35::workload_args("-$level", "$dir/out.o")])
			: PA35::callgrind_command(out => $result, sim => $a{sim},
				argv => ["$dir/bin", PA35::workload_args("-$level", "$dir/out.o")]);
		push @commands, "timeout $a{timeout} " . join(' ', map { "'$_'" } @cmd) . " > '$dir/run.log' 2>&1 && touch '$dir/done'";
	}
}
my $failures = @commands ? PA35::run_parallel(scalar @commands, @commands) : 0;
die "$failures measurement run(s) failed; see run.log under pa35/obj/cache\n" if $failures;

my (@rows, $text);
my ($tseed, $tself) = (PA35::text_bytes($bin{seed}), PA35::text_bytes($bin{self}));
$text = "$LA $bin{seed}\n$LB $bin{self}\ncounter $a{counter}" . ($a{sim} ? ' (with cache and branch simulation)' : '') . "\n";
$text .= sprintf("text bytes: $LA %d, $LB %d (%.3fx)\n", $tseed, $tself, $tself / ($tseed || 1));
for my $level (@levels) {
	for my $side (qw(seed self)) {
		copy("$cache{$level}{$side}/out.o", "$out/$level-$side.o");
	}
	my $same = compare("$out/$level-seed.o", "$out/$level-self.o") == 0;
	my (%v, %p);
	for my $side (qw(seed self)) {
		if ($kind eq 'perf') {
			$v{$side} = { total => PA35::parse_perf_instructions("$cache{$level}{$side}/perf.txt") };
			next;
		}
		$p{$side} = PA35::parse_callgrind("$cache{$level}{$side}/run.cg", "$cache{$level}{$side}/bin");
		my $t = $p{$side}{totals};
		my $calls = 0;
		$calls += $_ for values %{ $p{$side}{calls} };
		$v{$side} = { own => $p{$side}{own}, total => $t->{Ir}, calls => $calls, proxy => PA35::cycle_proxy($t), t => $t };
	}
	$text .= "\n-$level: objects " . ($same ? 'identical' : 'DIFFER') . "\n";
	for my $k (qw(own total calls proxy)) {
		next unless defined $v{seed}{$k} && $v{seed}{$k};
		my $ratio = $v{self}{$k} / $v{seed}{$k};
		$text .= sprintf("  %-6s $LA %12s  $LB %12s  ratio %.4f\n", $k, PA35::fmt_m($v{seed}{$k}), PA35::fmt_m($v{self}{$k}), $ratio);
		push @rows, join("\t", $level, $k, $v{seed}{$k}, $v{self}{$k}, sprintf('%.6f', $ratio));
	}
	if ($a{sim}) {
		for my $e (qw(I1mr D1mr D1mw Bcm Bim)) {
			$text .= sprintf("  %-6s $LA %12s  $LB %12s\n", $e, PA35::fmt_m($v{seed}{t}{$e} // 0), PA35::fmt_m($v{self}{t}{$e} // 0));
		}
	}
	push @rows, join("\t", $level, 'identical', $same ? 1 : 0, $same ? 1 : 0, 1);
	next if $kind eq 'perf';

	my %mix = map { $_ => PA35::instruction_mix($p{$_}, "$cache{$level}{$_}/bin") } qw(seed self);
	my %classes = map { $_ => 1 } (keys %{ $mix{seed} }, keys %{ $mix{self} });
	my $excess = sub { ($mix{self}{ $_[0] } // 0) - ($mix{seed}{ $_[0] } // 0) };
	my $m = sprintf("%-20s %12s %12s %12s\n", 'class (own code)', $LA, $LB, 'excess');
	for my $c (sort { $excess->($b) <=> $excess->($a) } keys %classes) {
		$m .= sprintf("%-20s %12s %12s %12s\n", $c, PA35::fmt_m($mix{seed}{$c} // 0), PA35::fmt_m($mix{self}{$c} // 0), PA35::fmt_m($excess->($c)));
	}
	PA35::spit("$out/$level-mix.txt", $m);

	for my $k (['calls', 'calls', 'calls'], ['fn', 'functions', 'own Ir']) {
		my ($key, $file, $label) = @$k;
		my %names = map { $_ => 1 } (keys %{ $p{seed}{$key} }, keys %{ $p{self}{$key} });
		my $d = sub { ($p{self}{$key}{ $_[0] } // 0) - ($p{seed}{$key}{ $_[0] } // 0) };
		my @order = sort { $d->($b) <=> $d->($a) } keys %names;
		my @lowest = reverse @order;
		splice(@lowest, 10) if @lowest > 10;
		splice(@order, $a{top}) if @order > $a{top};
		my $r = sprintf("%14s %14s %14s  %s\n", "$LA $label", "$LB $label", 'excess', 'function');
		for my $name (@order, '...', @lowest) {
			if ($name eq '...') { $r .= "...\n(most negative)\n"; next; }
			$r .= sprintf("%14d %14d %14d  %s\n", $p{seed}{$key}{$name} // 0, $p{self}{$key}{$name} // 0, $d->($name), $name);
		}
		PA35::spit("$out/$level-$file.txt", $r);
	}
}
PA35::spit("$out/summary.txt", $text);
PA35::spit("$out/summary.tsv", join("\n", "level\tmetric\tseed\tself\tratio", @rows) . "\n");
print $text;
print "\nwrote $out/summary.txt" . ($kind eq 'perf' ? '' : " and <level>-{mix,calls,functions}.txt") . "\n";
for my $level (@levels) {
	die "objects differ at -$level: the two compilers must produce identical output\n"
		if compare("$out/$level-seed.o", "$out/$level-self.o") != 0;
}
