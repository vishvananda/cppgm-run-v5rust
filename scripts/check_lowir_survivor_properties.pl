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

sub run_optimizer
{
	my ($app, $test, $directory, $level, @options) = @_;
	my $label = @options ? "$level-limited" : $level;
	my $output = "$directory/$label.lowir";
	my $stderr = "$directory/$label.stderr";
	my $status = run_command_capture(
		cmd => [$app, "-$level", @options, '-o', $output, $test],
		stdout => "$directory/$label.stdout",
		stderr => $stderr,
		timeout => 30,
	);
	die "$test: lowiropt " . join(' ', "-$level", @options) . " failed\n" .
		read_file($stderr)
		if $status != 0;
	return read_file($output);
}

sub function_body
{
	my ($test, $output, $name) = @_;
	return $1 if $output =~
		/(function \@\Q$name\E\b.*?)(?=\nfunction \@|\z)/s;
	die "$test: optimized output has no $name definition\n";
}

sub block_body
{
	my ($test, $function, $name) = @_;
	return $1 if $function =~
		/(^\s+block \^\Q$name\E:\n.*?)(?=^\s+block \^|\z)/ms;
	die "$test: function has no $name block\n";
}

sub check_phi_predecessors
{
	my ($test, $name, $body) = @_;
	my %predecessors;
	my %blocks;
	while($body =~
		/^\s+block \^([A-Za-z0-9_]+):\n(.*?)(?=^\s+block \^|\z)/msg) {
		my ($label, $instructions) = ($1, $2);
		$blocks{$label} = $instructions;
		while($instructions =~
			/^\s+(?:jump|branch\s+[^,]+,|switch\s+[^,]+,)\s*\^([A-Za-z0-9_]+)/mg) {
			$predecessors{$1}{$label} = 1;
		}
		while($instructions =~ /,\s*\^([A-Za-z0-9_]+)/g) {
			$predecessors{$1}{$label} = 1;
		}
	}
	for my $label (keys(%blocks)) {
		my $instructions = $blocks{$label};
		while($instructions =~ /^\s+%\w+ = phi \w+ (.+)$/mg) {
			my $inputs = $1;
			my %incoming;
			while($inputs =~ /(?:\[|,)\s*\^([A-Za-z0-9_]+):/g) {
				$incoming{$1} = 1;
			}
			for my $source (keys(%incoming)) {
				die "$test: $name phi names non-predecessor $source " .
					"for $label\n"
					if !$predecessors{$label}{$source};
			}
			for my $source (keys(%{$predecessors{$label} || {}})) {
				die "$test: $name phi omits predecessor $source " .
					"for $label\n"
					if !$incoming{$source};
			}
		}
	}
}

if(scalar(@ARGV) != 2)
{
	die "Usage: check_lowir_survivor_properties.pl " .
		"<lowiropt> <test-or-directory>\n";
}

my ($app, $root) = @ARGV;
my @tests = collect_tests($root,
	qr/(?:\d{3}-branch-boolean-conversion|\d{3}-negated-boolean-compare|\d{3}-duplicate-block-loads|\d{3}-staged-copy-forwarding|\d{3}-aggregate-slot-scalar-replacement|\d{3}-inline-growth-budget-boundary|\d{3}-inline-trivial-leaf-budget-exempt|\d{3}-truncate-noreturn-continuation|\d{3}-inline-discardable-size-cap|\d{3}-inline-single-call-caller-budget|\d{3}-inline-hint-late-nonleaf|\d{3}-nonzero-underflow-predicate|\d{3}-adjacent-noalias-scalar-copy|\d{3}-shared-loop-inline-policy|\d{3}-cold-path-discounted-inlining|\d{3}-phi-integrity-survivors|\d{3}-nonzero-underflow-value-proof|\d{3}-loop-carried-store-forwarding|\d{3}-volatile-access-preservation)\.t$/);
die "No LowIR survivor-property tests found under $root\n" if !@tests;

# The budget fixtures' 50 copies of a 32-instruction @piece fit the default
# per-caller budget of the ordinary and late waves together; this budget
# (each wave's, through the documented diagnostic control) runs out after
# 20 copies in each wave.
my @caller_budget = ('--inline-limit', 'caller-budget=640');

for my $test (@tests)
{
	my $directory = tempdir('lowir-survivor-property-XXXXXX',
		TMPDIR => 1, CLEANUP => 1);
	my $o0 = run_optimizer($app, $test, $directory, 'O0');
	my $o1 = run_optimizer($app, $test, $directory, 'O1');
	my $optimized = $test =~ /\d{3}-volatile-access-preservation/
		? run_optimizer($app, $test, $directory, 'O2') : $o1;

	if($test =~ /\d{3}-volatile-access-preservation/) {
		my $baseline = function_body(
			$test, $o0, 'keeps_volatile_slot_accesses');
		my $positive = function_body(
			$test, $optimized, 'keeps_volatile_slot_accesses');
		for my $body ($baseline, $positive) {
			die "$test: volatile slot traffic was removed or merged\n"
				if scalar(() = $body =~ /^\s+store volatile i32 /mg) != 2 ||
				   scalar(() = $body =~ /^\s+%\w+ = load volatile i32 /mg) != 2;
		}
		my $unused = function_body(
			$test, $optimized, 'keeps_unused_volatile_load');
		die "$test: unused volatile load was removed\n"
			if $unused !~ /^\s+%\w+ = load volatile i32 %\w+$/m;
		my $ordinary = function_body(
			$test, $optimized, 'keeps_ordinary_elimination');
		die "$test: ordinary slot traffic did not remain optimizable\n"
			if $ordinary =~ /^\s+(?:store|%\w+ = load) i32 /m ||
			   $ordinary !~ /^\s+return i32 18$/m;
		next;
	}

	if($test =~ /\d{3}-branch-boolean-conversion/) {
		my $baseline = function_body($test, $o0, 'truth_trunc');
		my $positive = function_body($test, $o1, 'truth_trunc');
		die "$test: O0 lost the compare-result truncation baseline\n"
			if $baseline !~ /^\s+%\w+ = convert trunc u8 i64 /m;
		die "$test: O1 retained a compare-result truncation before branch\n"
			if $positive =~ /^\s+%\w+ = convert trunc u8 i64 /m;
		my $widen = function_body($test, $o1, 'truth_widen');
		die "$test: O1 retained a Boolean widening before branch\n"
			if $widen =~ /^\s+%\w+ = convert zext i64 u8 /m;
		my $guard = function_body($test, $o1, 'keeps_value_truncation');
		die "$test: O1 removed a value-changing truncation\n"
			if $guard !~ /^\s+%\w+ = convert trunc u8 i64 /m;
		next;
	}
	if($test =~ /\d{3}-negated-boolean-compare/) {
		my $baseline = function_body($test, $o0, 'negated_direct');
		my $positive = function_body($test, $o1, 'negated_direct');
		my @baseline_cmps = $baseline =~ /^\s+%\w+ = cmp /mg;
		my @positive_cmps = $positive =~ /^\s+%\w+ = cmp /mg;
		die "$test: O0 lost the nested-compare baseline\n"
			if scalar(@baseline_cmps) != 2;
		die "$test: O1 did not collapse the integer negation to one compare\n"
			if scalar(@positive_cmps) != 1 ||
			   $positive !~ /^\s+%\w+ = cmp ne i64 /m;
		my $guard = function_body($test, $o1, 'keeps_float');
		my @float_cmps = $guard =~ /^\s+%\w+ = cmp /mg;
		die "$test: O1 applied the integer-only negation rule to float\n"
			if scalar(@float_cmps) != 2;
		next;
	}
	if($test =~ /\d{3}-duplicate-block-loads/) {
		my $baseline = function_body($test, $o0, 'repeated');
		my $positive = function_body($test, $o1, 'repeated');
		my @baseline_loads = $baseline =~ /^\s+%\w+ = load i64 \@cell$/mg;
		my @positive_loads = $positive =~ /^\s+%\w+ = load i64 \@cell$/mg;
		die "$test: O0 did not preserve three same-block loads\n"
			if scalar(@baseline_loads) != 3;
		die "$test: O1 did not reuse the adjacent duplicate load or " .
			"crossed the call barrier\n"
			if scalar(@positive_loads) != 2;
		my $guard = function_body($test, $o1, 'keeps_store');
		my @guard_loads = $guard =~ /^\s+%\w+ = load i64 %base$/mg;
		# The load after the store may take the stored value, never the
		# load before it.
		die "$test: O1 reused a load across a store\n"
			if $guard !~ /^\s+store i64 9, %base$/m ||
			   (scalar(@guard_loads) != 2 &&
			    $guard !~ /^\s+%\w+ = binary add i64 %\w+, 9$/m);
		next;
	}
	if($test =~ /\d{3}-staged-copy-forwarding/) {
		my $baseline = function_body($test, $o0, 'forwards');
		my $positive = function_body($test, $o1, 'forwards');
		die "$test: O0 lost the staged-object baseline\n"
			if $baseline !~ /^\s+slot \$staging : obj<24x8>$/m ||
			   $baseline !~ /^\s+copyobj 24x8 /m;
		die "$test: O1 retained the eligible staging object or bulk copy\n"
			if $positive =~ /^\s+slot \$staging\b/m ||
			   $positive =~ /^\s+copyobj 24x8 /m;
		# The conditionally stored field is tracked apart from the rest of
		# the staging object: a phi of its value per path (13 from ^set)
		# is what reaches the destination's second field.
		my $conditional = function_body(
			$test, $o1, 'keeps_conditional_store');
		my ($field) = $conditional =~
			/^\s+%(\w+) = phi i64 \[[^\]\n]*\^set: 13[,\]]/m;
		my ($high) = $conditional =~
			/^\s+%(\w+) = index i8 \[projection=field\] %dest, 8$/m;
		die "$test: conditional field lost its independently tracked home\n"
			if $conditional =~ /^\s+slot \$staging\b/m ||
			   $conditional =~ /^\s+copyobj 16x8 /m ||
			   !defined($field) || !defined($high) ||
			   $conditional !~ /^\s+store i64 %\Q$field\E, %\Q$high\E$/m;
		next;
	}
	if($test =~ /\d{3}-aggregate-slot-scalar-replacement/) {
		my $baseline = function_body($test, $o0, 'decomposes');
		my $positive = function_body($test, $o1, 'decomposes');
		die "$test: O0 lost the aggregate-slot baseline\n"
			if $baseline !~ /^\s+slot \$local : obj<24x8>$/m;
		die "$test: O1 retained the eligible aggregate object\n"
			if $positive =~ /^\s+slot \$local : obj/m ||
			   $positive =~ /^\s+copyobj 24x8 /m;
		my $copies = function_body($test, $o1, 'copies_between_locals');
		die "$test: local aggregate copy did not fully collapse\n"
			if $copies =~ /^\s+slot /m || $copies =~ /^\s+copyobj /m;
		my $escape = function_body($test, $o1, 'escaping_stays');
		die "$test: escaping aggregate was scalar-replaced\n"
			if $escape !~ /^\s+slot \$local : obj<16x8>$/m ||
			   $escape !~ /\bcall void \@escape\(/;
		next;
	}
	if($test =~ /\d{3}-inline-growth-budget-boundary/) {
		my $baseline = function_body($test, $o0, 'driver');
		my $positive = function_body($test, run_optimizer(
			$app, $test, $directory, 'O1', @caller_budget), 'driver');
		my @before = $baseline =~ /^\s+call void \@piece\b/mg;
		my @after = $positive =~ /^\s+call void \@piece\b/mg;
		die "$test: reducer did not start above the caller-growth budget\n"
			if scalar(@before) < 2;
		die "$test: bounded inlining made no progress or ignored the budget\n"
			if !@after || scalar(@after) >= scalar(@before);
		next;
	}
	if($test =~ /\d{3}-inline-trivial-leaf-budget-exempt/) {
		my $baseline = function_body($test, $o0, 'driver');
		my $positive = function_body($test, run_optimizer(
			$app, $test, $directory, 'O1', @caller_budget), 'driver');
		die "$test: O0 lost the tiny/small call baseline\n"
			if $baseline !~ /^\s+call void \@tiny\b/m ||
			   $baseline !~ /^\s+call void \@small\b/m;
		die "$test: exhausted-budget trivial leaf was not inlined\n"
			if $positive =~ /^\s+call void \@tiny\b/m;
		die "$test: exemption incorrectly admitted the larger leaf\n"
			if $positive !~ /^\s+call void \@small\b/m;
		next;
	}
	if($test =~ /\d{3}-truncate-noreturn-continuation/) {
		my $baseline = function_body($test, $o0, 'select');
		my $positive = function_body($test, $o1, 'select');
		die "$test: O0 lost the post-noreturn continuation baseline\n"
			if block_body($test, $baseline, 'raise') !~ /^\s+jump \^merge$/m;
		die "$test: O1 retained control flow after the noreturn call\n"
			if block_body($test, $positive, 'raise') =~ /^\s+jump /m;
		my $guard = function_body($test, $o1, 'keeps_reachable_tail');
		die "$test: reachable merge tail was removed\n"
			if $guard !~ /^\s+%\w+ = phi i64 /m;
		check_phi_predecessors($test, 'keeps_reachable_tail', $guard);
		next;
	}
	if($test =~ /\d{3}-inline-discardable-size-cap/) {
		my $main = function_body($test, $o1, 'main');
		die "$test: at-limit discardable body was not transferred\n"
			if $main =~ /\bcall i64 \@at_limit\b/ ||
			   $o1 =~ /^function \@at_limit\b/m;
		die "$test: over-limit discardable body was transferred\n"
			if $main !~ /\bcall i64 \@too_large\b/ ||
			   $o1 !~ /^function \@too_large\b/m;
		next;
	}
	if($test =~ /\d{3}-inline-single-call-caller-budget/) {
		my $baseline = function_body($test, $o0, 'main');
		my $positive = function_body($test, $o1, 'main');
		my @before = $baseline =~ /^\s+call void \@(first|second|third|fourth|fifth)\b/mg;
		my @after = $positive =~ /^\s+call void \@(first|second|third|fourth|fifth)\b/mg;
		die "$test: O0 lost the five-call budget baseline\n"
			if scalar(@before) != 5;
		die "$test: caller budget made no progress or admitted every body\n"
			if !@after || scalar(@after) >= scalar(@before);
		next;
	}
	if($test =~ /\d{3}-inline-hint-late-nonleaf/) {
		my $main = function_body($test, $o1, 'main');
		die "$test: eligible hinted nonleaf was not inlined\n"
			if $main =~ /^\s+call void \@preferred\b/m;
		die "$test: no_inline guard was bypassed\n"
			if $main !~ /^\s+call void \@blocked\b/m;
		die "$test: hinted body cap was bypassed\n"
			if $main !~ /^\s+call void \@over_cap\b/m;
		next;
	}

	if($test =~ /\d{3}-nonzero-underflow-predicate/) {
		my $baseline = function_body(
			$test, $o0, 'fold_after_nonzero_edge');
		my $positive = function_body(
			$test, $o1, 'fold_after_nonzero_edge');
		die "$test: O0 lost the underflow-comparison baseline\n"
			if $baseline !~ /^\s+%\w+ = cmp uge u32 /m;
		die "$test: O1 retained the proved-false underflow comparison\n"
			if $positive =~ /^\s+%\w+ = cmp uge u32 /m;
		for my $name ('retain_after_mutation', 'retain_volatile_reload') {
			my $guard = function_body($test, $o1, $name);
			die "$test: O1 removed the guarded underflow comparison in $name\n"
				if $guard !~ /^\s+%\w+ = cmp uge u32 /m;
		}
		next;
	}
	if($test =~ /\d{3}-nonzero-underflow-value-proof/) {
		my $baseline = function_body(
			$test, $o0, 'fold_dominating_same_value');
		my $positive = function_body(
			$test, $o1, 'fold_dominating_same_value');
		die "$test: O0 lost the dominating-value underflow baseline\n"
			if $baseline !~ /^\s+%\w+ = cmp uge u32 /m;
		die "$test: O1 retained the dominating same-value underflow comparison\n"
			if $positive =~ /^\s+%\w+ = cmp uge u32 /m;
		my $guard = function_body(
			$test, $o1, 'retain_different_nonzero_value');
		die "$test: O1 confused distinct nonzero and decremented values\n"
			if $guard !~ /^\s+%\w+ = cmp uge u32 /m;
		next;
	}
	if($test =~ /\d{3}-loop-carried-store-forwarding/) {
		my $baseline = function_body(
			$test, $o0, 'forward_exact_store');
		my $positive = function_body(
			$test, $o1, 'forward_exact_store');
		die "$test: O0 lost the loop-header load baseline\n"
			if $baseline !~ /block \^header:.*?^\s+%\w+ = load i64 \@cell$/ms;
		die "$test: O1 did not replace the loop-header load with a phi\n"
			if $positive !~ /block \^header:\n\s+%\w+ = phi i64 /m ||
			   $positive =~ /block \^header:.*?^\s+%\w+ = load i64 \@cell$/ms;
		# retain_ordinary_exact_store differs from forward_exact_store only
		# in its inline hint: its backedge store is proved and is not a
		# guard. In the guards no phi takes the value the latch stores
		# (to another global, or before a call that writes @cell), and
		# @cell is still read: anywhere in retain_different_store (its
		# loop never writes @cell), inside the loop in
		# retain_post_store_call (the call writes it every pass).
		for my $name ('retain_different_store', 'retain_post_store_call') {
			my $guard = function_body($test, $o1, $name);
			my ($stored) = $guard =~
				/^\s+store i64 %(\w+), \@(?:cell|other)$/m;
			die "$test: $name lost its backedge store\n"
				if !defined($stored);
			my $reads = $name eq 'retain_post_store_call'
				? block_body($test, $guard, 'header') .
				  block_body($test, $guard, 'latch')
				: $guard;
			die "$test: O1 forwarded an unproved backedge store in $name\n"
				if $guard =~ /^\s+%\w+ = phi i64 [^\n]*%\Q$stored\E\b/m ||
				   $reads !~ /^\s+%\w+ = load i64 \@cell$/m;
		}
		next;
	}
	if($test =~ /\d{3}-adjacent-noalias-scalar-copy/) {
		my $baseline = function_body($test, $o0, 'coalesce_noalias');
		my $positive = function_body($test, $o1, 'coalesce_noalias');
		die "$test: O0 unexpectedly coalesced the scalar-copy baseline\n"
			if $baseline =~ /^\s+copyobj /m;
		die "$test: O1 did not form one contiguous 16-byte copy\n"
			if $positive !~ /^\s+copyobj 16x8 /m;
		for my $name ('retain_aliasable', 'retain_volatile') {
			my $guard = function_body($test, $o1, $name);
			die "$test: O1 coalesced the guarded scalar copies in $name\n"
				if $guard =~ /^\s+copyobj /m;
			die "$test: guarded scalar traffic disappeared in $name\n"
				if $guard !~ /^\s+%\w+ = load(?: volatile)? i64 /m ||
				   $guard !~ /^\s+store(?: volatile)? i64 /m;
		}
		next;
	}
	if($test =~ /\d{3}-shared-loop-inline-policy/) {
		for my $name ('shared_a', 'shared_b') {
			my $shared = function_body($test, $o1, $name);
			die "$test: O1 cloned the shared ordinary loop into $name\n"
				if $shared !~ /^\s+%\w+ = call i64 \@shared_loop\b/m;
		}
		my $single = function_body($test, $o1, 'single_caller');
		die "$test: O1 did not inline the single-use loop\n"
			if $single =~ /\bcall i64 \@single_loop\b/;
		for my $name ('hinted_a', 'hinted_b') {
			my $hinted = function_body($test, $o1, $name);
			die "$test: O1 did not honor the explicit loop inline hint in $name\n"
				if $hinted =~ /\bcall i64 \@hinted_loop\b/;
		}
		next;
	}
	if($test =~ /\d{3}-cold-path-discounted-inlining/) {
		for my $name ('cold_a', 'cold_b') {
			my $cold = function_body($test, $o1, $name);
			die "$test: cold nonreturning-side work blocked $name inlining\n"
				if $cold =~ /\bcall i64 \@cold_helper\b/;
		}
		for my $name ('hot_a', 'hot_b') {
			my $hot = function_body($test, $o1, $name);
			die "$test: equivalent returning-path work was not charged in $name\n"
				if $hot !~ /\bcall i64 \@hot_helper\b/;
		}
		next;
	}
	if($test =~ /\d{3}-phi-integrity-survivors/) {
		for my $name ('repair_fold_target_phi',
			'inline_after_dead_predecessor', 'cross_slot_needed_phi') {
			check_phi_predecessors(
				$test, $name, function_body($test, $o1, $name));
		}
		my $dead = function_body(
			$test, $o1, 'inline_after_dead_predecessor');
		die "$test: dead predecessor value survived inlining\n"
			if $dead =~ /\bphi\b|\@undefined_path\b/ ||
			   $dead !~ /^\s+return i64 47$/m;
		my $cross = function_body($test, $o1, 'cross_slot_needed_phi');
		die "$test: cross-slot forwarding retained the destination slot\n"
			if $cross =~ /^\s+slot \$forwarded\b/m;
		my ($phi) = $cross =~ /^\s+%\w+ = phi i64 (.+)$/m;
		die "$test: cross-slot forwarding omitted the required join phi\n"
			if !defined($phi) || $phi !~ /:\s*5(?:,|\])/ ||
			   $phi !~ /:\s*9(?:,|\])/;
		next;
	}
	die "$test: no survivor-property predicate selected\n";
}

print "LowIR survivor properties: PASS (" . scalar(@tests) . "/" .
	scalar(@tests) . ")\n" unless $ENV{CPPGM_REPORT_QUIET};
