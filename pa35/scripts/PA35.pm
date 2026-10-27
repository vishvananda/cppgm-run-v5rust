package PA35;

# Shared helpers for the PA35 measurement scripts: the frozen workload,
# building a compiler binary with any compiler, callgrind runs and parsing,
# and the instruction-class mix of a binary's own code.

use strict;
use warnings;
use Cwd qw(abs_path getcwd);
use Digest::SHA;
use File::Basename qw(dirname basename);
use File::Path qw(make_path);
use File::Spec;
use POSIX qw(:sys_wait_h);
use Fcntl qw(F_SETFD);
use IO::Handle;

our $PA35 = abs_path(dirname(__FILE__) . '/..');
our $ROOT = abs_path("$PA35/..");

# Output appears as it is printed, not when a pipe's buffer fills or the
# script exits. A long measurement run in the background otherwise looks
# silent for its whole duration, and an agent harness that watches for output
# can take it for a hung command and kill it.
$| = 1;

# Progress lines for long steps. They go to the stderr of the outermost PA35
# script (the terminal or task log that started it), even from a script whose
# output a parent is capturing with run_capture, so `make -C pa35 test` shows
# them too. Captured output is unchanged by them.
our $PROGRESS;
if (($ENV{PA35_PROGRESS_FD} // '') =~ /^\d+$/ && open(my $inherited, '>&=', $ENV{PA35_PROGRESS_FD})) {
	$PROGRESS = $inherited;
} else {
	open($PROGRESS, '>&', \*STDERR) or $PROGRESS = \*STDERR;
}
$PROGRESS->autoflush(1);

sub progress {
	my ($sec, $min, $hour) = localtime;
	printf {$PROGRESS} "[pa35 %02d:%02d:%02d] %s\n", $hour, $min, $sec, join(' ', @_);
}

# Valgrind emulates x87 arithmetic with 64-bit doubles, so the extreme
# long double limits in <limits> overflow under it. Every measurement of
# the workload (seed, self, host, with or without valgrind) uses the same
# representable values so the work stays identical.
our @LDBL_FLAGS = (
	'-U__LDBL_MAX__', '-D__LDBL_MAX__=1.0e300L',
	'-U__LDBL_MIN__', '-D__LDBL_MIN__=1.0e-300L',
	'-U__LDBL_DENORM_MIN__', '-D__LDBL_DENORM_MIN__=1.0e-310L',
	'-U__LDBL_NORM_MAX__', '-D__LDBL_NORM_MAX__=1.0e300L',
);

sub workload_dir { "$PA35/workload" }

# Command-line arguments (after the compiler) that compile the frozen TU at
# $level (-O<n>), or preprocess it when $level is -E.
sub workload_args {
	my ($level, $output) = @_;
	my $dir = workload_dir();
	my @mode = $level eq '-E' ? ('-E') : ($level, '-c');
	return (@mode, "$dir/semantic_overload.cpp", "-I$dir/include", @LDBL_FLAGS, '-o', $output);
}

sub sha256_file {
	my ($path) = @_;
	my $sha = Digest::SHA->new(256);
	$sha->addfile($path, 'b');
	return $sha->hexdigest;
}

# Dies unless the workload matches MANIFEST.sha256 exactly.
sub verify_workload {
	my $dir = workload_dir();
	open(my $fh, '<', "$dir/MANIFEST.sha256") or die "missing $dir/MANIFEST.sha256\n";
	my %want;
	while (my $line = <$fh>) {
		my ($sum, $file) = $line =~ /^([0-9a-f]{64})\s+(\S+)$/ or next;
		$want{$file} = $sum;
	}
	close $fh;
	my @present = ('semantic_overload.cpp', map { "include/" . basename($_) } glob("$dir/include/*"));
	for my $file (@present) {
		die "workload file $file is not in MANIFEST.sha256\n" unless exists $want{$file};
	}
	for my $file (sort keys %want) {
		die "workload file $file is missing\n" unless -f "$dir/$file";
		die "workload file $file changed (sha256)\n" if sha256_file("$dir/$file") ne $want{$file};
	}
	return scalar keys %want;
}

sub slurp {
	my ($path) = @_;
	open(my $fh, '<', $path) or die "cannot read $path: $!\n";
	local $/;
	my $text = <$fh>;
	close $fh;
	return $text;
}

sub spit {
	my ($path, $text) = @_;
	make_path(dirname($path));
	open(my $fh, '>', $path) or die "cannot write $path: $!\n";
	print $fh $text;
	close $fh;
}

# Runs a command list, returns (exit status, combined output).
sub run_capture {
	my (@cmd) = @_;
	my $pid = open(my $fh, '-|') // die "fork: $!\n";
	if ($pid == 0) {
		open(STDERR, '>&', \*STDOUT);
		# Hand the progress channel to child PA35 scripts across the exec.
		if (my $fd = fileno($PROGRESS)) {
			fcntl($PROGRESS, F_SETFD, 0);
			$ENV{PA35_PROGRESS_FD} = $fd;
		}
		exec(@cmd) or die "exec $cmd[0]: $!\n";
	}
	local $/;
	my $out = <$fh> // '';
	close $fh;
	return (exit_status($?), $out);
}

# The exit status of a wait status: the code of a process that exited, 128
# plus the signal of one a signal ended (as the shell reports it), 127 when
# the command could not run.  `$? >> 8` alone reports a killed child as 0.
sub exit_status {
	my ($wait) = @_;
	return 127 if $wait == -1;
	return 128 + ($wait & 127) if $wait & 127;
	return $wait >> 8;
}

sub run_or_die {
	my (@cmd) = @_;
	my ($status, $out) = run_capture(@cmd);
	die "command failed ($status): @cmd\n$out" if $status;
	return $out;
}

# Runs shell command strings with at most $jobs in parallel. Returns the
# number of failures; failing commands' output goes to STDERR.
our $SHOW_FAILED_COMMANDS = 1;

sub run_parallel {
	my ($jobs, @commands) = @_;
	my %running;
	my $failures = 0;
	my $reap = sub {
		my $pid = waitpid(-1, 0);
		return if $pid <= 0;
		my $job = delete $running{$pid};
		if ($?) {
			$failures++;
			print STDERR "FAILED: $job->{cmd}\n" if $SHOW_FAILED_COMMANDS;
			print STDERR slurp($job->{log});
		}
		unlink $job->{log};
	};
	my $n = 0;
	for my $cmd (@commands) {
		$reap->() while keys(%running) >= $jobs;
		my $log = File::Spec->tmpdir . "/pa35-job-$$-" . $n++ . ".log";
		my $pid = fork() // die "fork: $!\n";
		if ($pid == 0) {
			open(STDOUT, '>', $log);
			open(STDERR, '>&', \*STDOUT);
			exec('/bin/sh', '-c', $cmd) or exit 127;
		}
		$running{$pid} = { cmd => $cmd, log => $log };
	}
	$reap->() while keys %running;
	return $failures;
}

# Parallel jobs for builds and test programs: PA35_JOBS when set (a shared
# machine), otherwise the online processors, at most 32.
sub default_jobs {
	return $ENV{PA35_JOBS} if ($ENV{PA35_JOBS} // '') =~ /^[1-9]\d*$/;
	my $n = `getconf _NPROCESSORS_ONLN 2>/dev/null`;
	chomp $n;
	$n = 8 unless $n && $n > 0;
	return $n > 32 ? 32 : $n;
}

# Builds dev/'s cppgm++ from the source tree at $repo with $compiler, using
# the g++ build's own compile lines (same flags and defines, the compiler
# swapped) and its link line (same host link, same malloc). The source tree
# must have been built once with `make build` (generated headers). make_vars
# (VAR=VALUE strings) are passed to that `make build`, for example to take
# the compile lines of other flags.
sub build_with {
	my (%a) = @_;
	my $compiler = abs_path($a{compiler}) // die "no compiler $a{compiler}\n";
	my $repo = abs_path($a{repo} // $ROOT);
	my $out = $a{out};
	make_path("$out/objs");
	$out = abs_path($out);
	my $jobs = $a{jobs} // default_jobs();
	# The build's own command lines, from a dry run of the same `make build`
	# (an OBJ= in the environment reaches it like any other make variable).
	my ($status, $dry) = run_capture('make', '-C', $repo, '-n', '-B', 'build', @{ $a{make_vars} // [] });
	my @lines = split /\n/, $dry;
	# The link is the line that writes cppgm++ (the Makefile links a .tmp it
	# then moves into place) and compiles nothing.
	my ($link) = grep { / -o (?:\S*\/)?cppgm\+\+(?:\.tmp)?(?: |$)/ && !/(?:^| )-c(?: |$)/ } @lines;
	die "no cppgm++ link line in `make -n -B build` output: the self build takes the g++ build's own link line, which must name cppgm++ (or cppgm++.tmp) as its -o output\n"
		unless $link;
	my @linked = $link =~ /(?:^| )(\S+\.o)(?= |$)/g;
	my %linked = map { $_ => 1 } @linked;
	die "the cppgm++ link line names no object files\n" unless @linked;
	# The objects keep their layout below the directory they share, whatever
	# object root the build uses.
	my $root = common_directory(@linked);
	my @jobs;
	for my $line (grep { /(?:^| )-c(?: |$)/ && !/^printf/ } @lines) {
		my ($obj) = $line =~ / -o (\S+\.o)(?: |$)/ or next;
		next unless $linked{$obj};
		my @words = split ' ', $line;
		my @keep;
		for (my $i = 1; $i < @words; $i++) {
			my $w = $words[$i];
			if ($w eq '-MT' || $w eq '-MF' || $w eq '-o') { $i++; next; }
			next if $w eq '-MMD' || $w eq '-MP';
			push @keep, $w;
		}
		my $dest = "$out/objs/" . substr($obj, length($root));
		make_path(dirname($dest));
		push @jobs, "cd '$repo/dev' && '$compiler' @keep -o '$dest'";
	}
	die "found no compile lines for the objects the cppgm++ link names\n" unless @jobs;
	my $failures = run_parallel($jobs, @jobs);
	die "$failures of " . scalar(@jobs) . " compiles failed\n" if $failures;
	(my $relinked = $link) =~ s{(?:^| )\K(\S+\.o)(?= |$)}{ $linked{$1} ? "'$out/objs/" . substr($1, length($root)) . "'" : $1 }ge;
	$relinked =~ s{-o (?:\S*\/)?cppgm\+\+(?:\.tmp)?(?= |$)}{-o '$out/cppgm++'};
	my ($ls, $lout) = run_capture('/bin/sh', '-c', "cd '$repo/dev' && $relinked");
	die "link failed:\n$lout" if $ls;
	return ("$out/cppgm++", scalar @jobs);
}

# The longest directory prefix (with its trailing slash) the paths share.
sub common_directory {
	my (@paths) = @_;
	my @common = split m{/}, $paths[0], -1;
	pop @common;
	for my $path (@paths[1 .. $#paths]) {
		my @parts = split m{/}, $path, -1;
		pop @parts;
		my $n = 0;
		$n++ while $n < @common && $n < @parts && $common[$n] eq $parts[$n];
		splice(@common, $n);
	}
	return @common ? join('/', @common) . '/' : '';
}

# ---------------------------------------------------------------- callgrind

sub callgrind_command {
	my (%a) = @_;
	my @cmd = ('valgrind', '--tool=callgrind', '--dump-instr=yes', "--callgrind-out-file=$a{out}");
	push @cmd, '--cache-sim=yes', '--branch-sim=yes' if $a{sim};
	return (@cmd, @{ $a{argv} });
}

# Parses a callgrind output file. Returns a hash:
#   events  => [names], totals => {event => count},
#   objects => {object path => Ir}, own => Ir of $binary,
#   own_events => {event => count} for $binary,
#   addr => {address => Ir} for $binary (with --dump-instr=yes),
#   addr_fn => {address => function} for $binary (with --dump-instr=yes),
#   fn => {function => self Ir} for $binary,
#   calls => {callee => calls executed} (callees in $binary).
sub parse_callgrind {
	my ($path, $binary) = @_;
	$binary = abs_path($binary) if defined $binary;
	my %r = (events => [], totals => {}, objects => {}, own => 0, own_events => {}, addr => {}, addr_fn => {}, fn => {}, calls => {});
	my (%obname, %fnname);
	my ($ob, $fn, $cob, $cfn) = ('', '', '', '');
	my @positions = ('line');
	my @last;
	my $skip = 0;
	my $instr_index = -1;
	open(my $fh, '<', $path) or die "cannot read $path: $!\n";
	while (my $line = <$fh>) {
		my $c = substr($line, 0, 1);
		if ($c =~ /[0-9+\-*]/) {
			my @p = split ' ', $line;
			for my $i (0 .. $#positions) {
				my $t = $p[$i];
				my $v;
				if ($t eq '*') { $v = $last[$i] // 0; }
				elsif ($t =~ /^[+-]/) { $v = ($last[$i] // 0) + ($t =~ /^([+-])0x([0-9a-f]+)$/i ? ($1 eq '-' ? -hex($2) : hex($2)) : $t); }
				else { $v = $t =~ /^0x/i ? hex($t) : $t; }
				$last[$i] = $v;
			}
			if ($skip) { $skip = 0; next; }
			my @costs = @p[scalar(@positions) .. $#p];
			my $ir = $costs[0] // 0;
			$r{objects}{$ob} += $ir;
			for my $e (0 .. $#costs) { $r{totals}{ $r{events}[$e] } += $costs[$e]; }
			if (defined $binary && $ob eq $binary) {
				$r{own} += $ir;
				$r{fn}{$fn} += $ir;
				$r{addr}{ $last[$instr_index] } += $ir if $instr_index >= 0;
				$r{addr_fn}{ $last[$instr_index] } //= $fn if $instr_index >= 0;
				for my $e (0 .. $#costs) { $r{own_events}{ $r{events}[$e] } += $costs[$e]; }
			}
			next;
		}
		if ($line =~ /^(c?)(ob|fn|fl|fi|fe)=\((\d+)\)(?: (.*))?$/) {
			my ($call, $kind, $id, $name) = ($1, $2, $3, $4);
			if ($kind eq 'ob') {
				$obname{$id} = (abs_path($name) // $name) if defined $name;
				if ($call) { $cob = $obname{$id} // ''; } else { $ob = $obname{$id} // ''; }
			} elsif ($kind eq 'fn') {
				$fnname{$id} = $name if defined $name;
				if ($call) { $cfn = $fnname{$id} // ''; } else { $fn = $fnname{$id} // ''; $cob = $ob; }
			}
			next;
		}
		if ($line =~ /^calls=(\d+)/) {
			$r{calls}{$cfn} += $1 if defined $binary && ($cob || $ob) eq $binary;
			$skip = 1;
			$cob = $ob;
			next;
		}
		if ($line =~ /^positions:\s*(.*)$/) {
			@positions = split ' ', $1;
			($instr_index) = grep { $positions[$_] eq 'instr' } 0 .. $#positions;
			$instr_index //= -1;
			next;
		}
		if ($line =~ /^events:\s*(.*)$/) { $r{events} = [split ' ', $1]; next; }
	}
	close $fh;
	return \%r;
}

# Cycle proxy from callgrind's cache and branch simulation (whole process):
# Ir/3 + 15 Bcm + 20 Bim + 10 I1mr + 10 (D1mr + D1mw). A deterministic stand-in
# for time on machines without hardware counters. It is not time: check
# important claims with paired timing (abba.pl).
sub cycle_proxy {
	my ($t) = @_;
	return undef unless defined $t->{Bcm};
	return ($t->{Ir} // 0) / 3 + 15 * ($t->{Bcm} // 0) + 20 * ($t->{Bim} // 0)
		+ 10 * ($t->{I1mr} // 0) + 10 * (($t->{D1mr} // 0) + ($t->{D1mw} // 0));
}

# ---------------------------------------------------------------- counters

# True when user-mode hardware instruction counting works here.
my $perf_ok;
sub perf_available {
	return $perf_ok if defined $perf_ok;
	my ($st, $out) = run_capture('perf', 'stat', '-x', ',', '-e', 'instructions:u', '--', 'true');
	$perf_ok = (!$st && $out =~ /^\d+,[^,]*,instructions/m) ? 1 : 0;
	return $perf_ok;
}

# Shell command that counts user-mode instructions of @argv with perf
# (mean of $repeat runs) into $out_file.
sub perf_command {
	my (%a) = @_;
	my $r = $a{repeat} // 3;
	return ('perf', 'stat', '-x', ',', '-r', $r, '-e', 'instructions:u', '-o', $a{out}, '--', @{ $a{argv} });
}

sub parse_perf_instructions {
	my ($path) = @_;
	for my $line (split /\n/, slurp($path)) {
		my @f = split /,/, $line;
		return $f[0] if @f >= 3 && $f[2] =~ /^instructions/ && $f[0] =~ /^\d+(\.\d+)?$/;
	}
	die "no instruction count in $path (try --counter callgrind)\n";
}

# What a measurement depends on besides the binary, the level and the
# counter: the hosted headers the frozen workload includes (libstdc++'s
# release and the C library), the tool that counts (valgrind's version),
# and the processor with its cache geometry, which the simulator takes as
# its defaults.  The workload itself is frozen and verified.
my $toolchain_key;
sub toolchain_key {
	return $toolchain_key if defined $toolchain_key;
	my $macros = `printf '#include <cstddef>\\n' | g++ -std=gnu++11 -x c++ -E -dM - 2>/dev/null` // '';
	my ($glibcxx) = $macros =~ /#define __GLIBCXX__ (\d+)/;
	my ($libc) = (`ldd --version 2>/dev/null` // '') =~ /^(.*)$/m;
	my ($valgrind) = (`valgrind --version 2>/dev/null` // '') =~ /^(.*)$/m;
	my ($cpu) = (`grep -m1 'model name' /proc/cpuinfo 2>/dev/null` // '') =~ /:\s*(.*)$/m;
	my $caches = join(' ', map { /^(\S+)\s+(\d+)/ ? "$1=$2" : () }
		grep { /CACHE_(?:SIZE|ASSOC|LINESIZE)/ } split(/\n/, `getconf -a 2>/dev/null` // ''));
	$toolchain_key = substr(Digest::SHA::sha256_hex(join("\n", $glibcxx // '', $libc // '', $valgrind // '', $cpu // '', $caches)), 0, 8);
	return $toolchain_key;
}

# Cache directory for one measurement of a binary, keyed by the binary's
# bytes, the toolchain key, the level and the counter.
sub cache_dir {
	my ($binary, $level, $kind) = @_;
	my $dir = "$PA35/obj/cache/" . substr(sha256_file($binary), 0, 20) . '-' . toolchain_key() . "-$level-$kind";
	make_path($dir);
	return $dir;
}

# ---------------------------------------------------------------- mix

sub disassemble {
	my ($binary) = @_;
	my %dis;
	open(my $fh, '-|', 'objdump', '-d', '--no-show-raw-insn', $binary) or die "objdump: $!\n";
	while (my $line = <$fh>) {
		$dis{ hex($1) } = $2 if $line =~ /^\s*([0-9a-f]+):\s+(.*?)\s*$/;
	}
	close $fh;
	return \%dis;
}

# Instruction class of one objdump (AT&T) line: the classes measure.pl's
# mix tables report.
sub classify {
	my ($t) = @_;
	$t =~ s/\s+#.*$//;
	$t =~ s/^(?:notrack|bnd|lock|rex\.\w*)\s+//;
	my ($mn, $ops) = $t =~ /^(\S+)\s*(.*)$/ or return '?';
	my $frame = $ops =~ /\(%r[bs]p\)/;
	my $mem = $ops =~ /\(/;
	return 'rep movs/stos' if $mn =~ /^rep/;
	return 'call' if $mn =~ /^call/;
	return 'ret' if $mn =~ /^ret/;
	return 'push/pop' if $mn =~ /^(push|pop)/;
	return 'leave' if $mn eq 'leave';
	return 'jmp' if $mn =~ /^jmp/;
	return 'jcc' if $mn =~ /^j/;
	return 'div' if $mn =~ /^i?div[bwlq]?$/;
	return 'sign-ext rax' if $mn =~ /^(cqto|cltq|cltd|cqo|cwtl)$/;
	return 'setcc' if $mn =~ /^set/;
	return 'cmov' if $mn =~ /^cmov/;
	return 'lea' if $mn =~ /^lea/;
	return 'nop' if $mn =~ /^(nop|xchg|data16|cs)/;
	if ($mn =~ /^mov(s|z)[bwl][wlq]?$/ || $mn =~ /^mov(slq|sbq|zbl|zwl|sbl|swl|swq|zbq|zwq|sbw|zbw)$/) {
		return $frame ? 'extend (frame)' : $mem ? 'extend load' : 'extend reg';
	}
	if ($mn =~ /^mov/ && $ops !~ /xmm/) {
		my ($src, $dst) = $ops =~ /^(.*),([^,]*)$/;
		$src //= ''; $dst //= $ops;
		return $frame ? 'store frame' : 'store mem' if $dst =~ /\(/;
		return $frame ? 'load frame' : 'load mem' if $src =~ /\(/;
		return 'mov imm' if $src =~ /^\$/;
		return 'mov reg-reg';
	}
	return 'sse/x87 ' . ($mem ? 'mem' : 'reg') if $ops =~ /xmm/ || $mn =~ /^(f|pxor|pcmp|pmov)/;
	return 'cmp/test' . ($frame ? ' frame' : $mem ? ' mem' : '') if $mn =~ /^(cmp|test)/;
	return 'alu' . ($frame ? ' frame' : $mem ? ' mem' : '');
}

# Dynamic instruction mix of $binary's own code from a parsed callgrind run.
# $dis is the binary's disassembly (disassemble), when the caller has it.
sub instruction_mix {
	my ($parsed, $binary, $dis) = @_;
	$dis //= disassemble($binary);
	my %mix;
	my $unknown = 0;
	while (my ($a, $n) = each %{ $parsed->{addr} }) {
		my $t = $dis->{$a};
		if (!defined $t) { $unknown += $n; next; }
		$mix{ classify($t) } += $n;
	}
	$mix{'(unknown address)'} = $unknown if $unknown;
	return \%mix;
}

# The same mix for each function of $binary's own code: {function => {class => Ir}}.
# Callgrind's recursion levels (name'2, name'3) are merged into the function.
sub owner_mix {
	my ($parsed, $binary, $dis) = @_;
	$dis //= disassemble($binary);
	my %mix;
	while (my ($a, $n) = each %{ $parsed->{addr} }) {
		my $t = $dis->{$a};
		(my $fn = $parsed->{addr_fn}{$a} // '?') =~ s/'\d+$//;
		$mix{$fn}{ defined $t ? classify($t) : '(unknown address)' } += $n;
	}
	return \%mix;
}

sub text_bytes {
	my ($binary) = @_;
	my $out = run_or_die('size', '-A', $binary);
	my ($text) = $out =~ /^\.text\s+(\d+)/m;
	return $text // 0;
}

# ---------------------------------------------------------------- timing

# Path of the runtime helper (runtime.c), built on first use.
sub runtime_helper {
	my $bin = "$PA35/obj/runtime";
	my $src = "$PA35/scripts/runtime.c";
	if (!-x $bin || (stat($src))[9] > (stat($bin))[9]) {
		make_path("$PA35/obj");
		run_or_die('cc', '-O2', '-o', $bin, $src);
	}
	return $bin;
}

# Runs a command under the runtime helper. Options (an optional leading hash
# reference): timeout => wall seconds, memory_kb => address-space limit,
# allow_fail => return instead of dying when the command fails or times out.
# Returns {user, sys, wall, rss_kb, status}; status is the exit code,
# "timeout" or "signal-N".
sub timed_run {
	my $opt = ref $_[0] eq 'HASH' ? shift : {};
	my (@cmd) = @_;
	my @limits;
	push @limits, '-t', int($opt->{timeout} + 0.5) if $opt->{timeout};
	push @limits, '-m', int($opt->{memory_kb}) if $opt->{memory_kb};
	my $helper = runtime_helper();
	my $pid = open(my $fh, '-|') // die "fork: $!\n";
	if ($pid == 0) {
		# The helper reports on stderr: send stderr into the pipe, stdout away.
		exec('/bin/sh', '-c', 'exec "$@" 2>&1 1>/dev/null', 'sh', $helper, @limits, @cmd) or exit 127;
	}
	local $/;
	my $out = <$fh> // '';
	close $fh;
	my @lines = grep { /^\d+\.\d+ \d+\.\d+ \d+\.\d+ \d+ \S+$/ } split /\n/, $out;
	die "no timing from runtime helper for @cmd\n$out" unless @lines;
	my ($user, $sys, $wall, $rss, $status) = split ' ', $lines[-1];
	die "command failed ($status): @cmd\n$out" if $status ne '0' && !$opt->{allow_fail};
	return { user => $user, sys => $sys, wall => $wall, rss_kb => $rss, status => $status, output => $out };
}

# The growth of a scaling family from its times at the base size and the
# sizes n, 2n, 4n, ... (scaling.pl): the ratio of the last two measured
# sizes' times above the base cost. A time that is not a number is the
# status of a run that exceeded its bound ("timeout") or failed, which is
# unbounded growth ("inf" or "failed") whatever the sizes before it showed;
# a last doubling under $noise seconds is "flat".
sub scaling_growth {
	my ($times, $noise) = @_;
	my @times = @$times;
	return "base $times[0]" if $times[0] !~ /^[\d.]+$/;
	for my $t (@times[1 .. $#times]) {
		return $t eq 'timeout' ? 'inf' : 'failed' if $t !~ /^[\d.]+$/;
	}
	return 'flat' if @times < 3;
	my ($a, $b) = map { $_ - $times[0] } @times[-2, -1];
	return $b < $noise ? 'flat' : $a <= 0 ? 'inf' : sprintf('%.2f', $b / $a);
}

sub median {
	my @v = sort { $a <=> $b } @_;
	return undef unless @v;
	return @v % 2 ? $v[$#v / 2] : ($v[@v / 2 - 1] + $v[@v / 2]) / 2;
}

sub fmt_m { sprintf('%.1fM', $_[0] / 1e6) }

1;
