# Frozen preparation baseline

## Current oracle and course refresh — v5.1 (2026-10-14)

The oracle is cppgm-extended `3438f5aea24d8fcc9a0113316f7e27c890cd5ac6` (v5.1); the matching
assignment export is `0bf28e39bca37f7e4173ae1e9ca557659ada7523` (v5.1).
This refresh imports reviewed course fixtures, references and harness changes,
including control numbering/placement fixes and counted control_t .t inputs.
The clone-specific harness and comparison policy are preserved.

Compiler sources and all nine built oracle binaries retain their exact hashes
from ee3596b2; there is no compiler behavior or performance change in this refresh.
The frozen manifest expands from 6709 to 7090 anchors.
All 390 new/renamed routes passed C++ oracle self-comparison, and
the protected clone harness unit suite passed.

The resumed target is pa11: the refreshed Rust check passes 263/409;
all 240 previously passing case identities from the handoff remain passing.
Validation artifacts: `/home/vishvananda/work/private/v5rust/artifacts/owner-v5.1-refresh`.
Newly counted failures remain normal implementation work; no exceptions were added.

## Current oracle refresh — 2026-10-13 (PA9 reader)

The reference is cppgm-extended `ee3596b2df2dadf70d7d2028a6a32d18e1923761`,
pushed to main after PR #69 passed the full GCC/libstdc++ and clang/libc++
CI matrix. Its PA9 reader accepts the handout's optional template-argument
references on dependent `member` facts, using the existing graph and encoder.
Only `abimangle` changes; the other eight tools, including `cppgm++`, retain
their exact binary hashes. The graph-storage comparison correction remains
in effect. Performance evidence is in `docs/v5/abi-member-template-facts.md`
of the reference: +0.0142% instructions on the legacy-member stress input,
+0.00034% on mixed PA9 facts, no clear throughput regression.

The four new PA9 fixtures and generated references are added to the protected
manifest without dropping any case. PA9 grows from 119 to 123 frozen cases.
Protected-input preflight and oracle self-comparison pass (123/123). Rust
retains all original 119 passes; the four new comparisons fail until its
reader supports the updated contract and rejection diagnostics. This is an
implementation boundary, not a completed audit. Evidence and before/after
manifests are in `artifacts/observer/baseline-ee3596b2/`.

There is no current `obj/bootstrap` cache. Earlier archived bootstrap and
fixed-point receipts remain historical; PA34 must build against this source,
even though the C++ seed compiler hash is unchanged. Ralph state and native
goal status are not edited by this baseline refresh.

## PA9 statistics policy correction — 2026-10-13

The acceptance owner authorized excluding `abimangle stats:` numeric
`peak_graph_bytes` from exact comparison because it counts reserved graph
storage. Graph nodes, work counters, source/output bytes and ordinary
diagnostics remain exact; raw receipts retain both measured allocations.
The comparator, contract and tests are installed in the protected control
bundle and checkout, with only their three protected hashes updated.
All 30 harness tests and the full protected-input preflight pass.
A fresh sweep of all 120 PA9 inputs with `--stats` passes 120/120, with
zero deterministic counter or output differences. Evidence is retained in
`artifacts/observer/pa9-policy-20261013/`. Examples are upstream in
cppgm-ralph commit `daf4144`.

The reference remains `883778fd`: its independent member-template fact-reader
gap is being corrected and validated upstream. This policy correction does
not certify that syntax or complete the PA9 audit.

## Current oracle refresh — 2026-10-13

The reference is cppgm-extended `883778fdfa45d523cf9e135d15169af976f451a8`,
merged into main after PR #68 passed the complete GCC/libstdc++ and clang/libc++
CI matrix. It handles untyped overload-set operands during operator candidate
collection without changing ordinary comparison or resource qualification rules.

All previous fixed cases are retained. The sixteen new PA7 rejection fixtures
and two PA12 valid enum/class operator fixtures and their generated references
are copied into the run and protected manifest. PA7 grows from 198 to 214 cases;
PA12 grows from 316 to 318. The existing course controls and accumulated Rust
regressions retain their identities. All nine tool digests and reference source
hashes are refreshed; only the C++ compiler binary changes.

Before restart, the sixteen previously blocked operator probes pass exact
protected comparisons against the updated oracle. Their raw receipts and the
before/after manifest record are retained under
`artifacts/observer/baseline-883778fd/`. PA7 passes 214/214, its full course lane passes 346/346, and PA1–PA6
differential and course lanes pass before the audit resumes. Upstream validation and the PA35 hardware
measurements are recorded in the reference's
`docs/v5/untyped-operator-operands.md`; no measurable performance regression was
found on the frozen O0/O3 workload.

Old C1/C2 receipts below remain historical. The cached bootstrap workspace and
generations are archived as `obj/bootstrap-before-883778fd/`, so PA34 must copy
the updated reference fixtures and rebuild the complete fixed-point proof. This
baseline update does not mark the Rust audit complete or advance Ralph's state.

## Previous oracle refresh — 2026-10-12

The reference at that refresh was cppgm-extended `0fe8e3b4bde9929c2419fb7906028df83bb17de5` (PR #65, merged).
It fixes speculative template failures being reused across different name
interpretations. The key now includes the selected conflict category,
alternative-reading mode, and following-call requirement. Argument parsing
sets angle mode itself; the incoming mode is excluded to preserve linear
failed-list work.

The three new PA5 fixtures and their generated references are added to the
protected manifest: the stage now has 196 frozen cases. Existing cases and
comparison policy are retained. All nine tools and source hashes are refreshed
with the reference. Previous C1/C2 receipts below are historical and cannot
certify the new source; the PA34 checker invalidates them by seed hash and
must rebuild its complete proof. Existing live PA5 findings must be rechecked
against this revision; this refresh does not certify the Rust implementation.

Upstream validation: CI passed GCC/libstdc++ and clang/libc++ on Ubuntu 24.04
and 26.04, full strict tests, debug-info, self-hosting, export and inception.
Fresh before/after PA35 release checks passed 17/17 with 100 generated seeds
in both course and student builds. Paired hardware-counter instruction changes
are consistent with zero; all measured output objects are byte-identical.

Prepared 2026-10-11; refreshed to cppgm-extended commit
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1` after correcting the PA33 test harness.
Compiler sources and all nine oracle binaries retain their original hashes
from `72181d97e2a8b90af16229a328fc8e72bfe71f4b`; the C1/C2 receipts remain applicable.

- Fresh C++ build: all nine tools, g++, original default host flags.
- Strict C++ report: 6,634/6,634 passing.
- Exact C++ fixed point: C1 and C2 match across 226 objects and
  all nine tools, using the protected PA34 replay recipe at identical paths.
  Receipts and hashes: [cpp-fixed-point.json](cpp-fixed-point.json).
- Differential harness self-comparison: 6684
  case anchors across PA1-PA33, all passing; counts in
  [oracle-routing.json](oracle-routing.json).
- Initial Rust PA1 result: 0/58 (status 86); oracle self-comparison: 58/58.
  This is the intended initial state, not a failed preparation.
- Rustfmt, strict Clippy, debug/release Rust tests, file audit and harness tests
  pass on the scaffold. The compiler implementation remains to be written.
- Bubblewrap read-only reference/control checks and the 64 GiB systemd scope
  have been exercised on the dedicated 100 GiB run volume.

PA33 `459-parameter-address-folding.t` originally rejected loads folded into
`add` memory operands even though the generated program was correct. The
checker now accepts those reads while retaining offset, call placement,
callee-saved register and frame checks. The maintainer Makefile propagates
failure from every structural checker while still running the later checks.
The original fixture is unchanged. Both staged O2/O3 paths and the ordinary
O3 driver path must pass and match the oracle (five compiler invocations).
There are no expected-failure exceptions in the frozen manifest or checker.
Regression tests cover missing/non-reading operands, call placement and
failure propagation from each checker. After the correction, the full strict
report passed 6,634/6,634 and all 306 harness tests passed. PA33's complete
differential self-check passed again with zero course-check failures; the
previously failing control now completes all five exact invocations.
The refreshed isolation/setup checks and all 14 clone harness tests also pass.

Companion `.t.1` files are inputs of their `.t` case, not separate tests.
Focused PA32/PA33 regression roots are routed to their owning Makefile lanes;
the addressed-scalar-slot case uses the same owner as `test-regression` because
its original focused route is missing. Full original course targets remain
separate stage-acceptance gates. Debug lanes are due in the PA33 release check.

Numeric time/RSS/storage measurements on recognized stats lines are retained
but not compared for equality. All other diagnostic/counter bytes and all
compiler output artifacts are strict. This allows a safe Rust representation
without requiring identical process timing or storage allocation.

The full Rust bootstrap proof has not run: R is still a scaffold. PA34 must
add CR and C3, rerun clean C1/C2, and prove all four equal before completion.

Preparation logs live under
`/home/vishvananda/work/private/v5rust/artifacts/`.

## Live-run gate review

After the first PA1 implementation handoff, comparison failure handling was
strengthened: focused cases force nonzero comparator failures; full course
runs check both `.test_failed` and per-lane counts even when make exits zero.
The PA34 final gate repeats Rust release/debug checks and the ownership census,
then runs PA1-PA33 and PA8/PA32/PA33 debug lanes with all primary and helper
tools bound to the rebuilt C++ generation.

Validation includes 17 clone harness tests, a real keep-going comparator
failure/recovery, all 140 PA33 oracle cases with strict focused exits, and all
33 course suites plus all three debug suites using the saved C++ fixed-point
binaries. Evidence lives
under `/home/vishvananda/work/private/v5rust/artifacts/observer/`.
The first implementation's compiler files, fixtures and source ledger were
not changed by this gate update.

## Owner-authorized resource policy — 2026-10-12

The C++ revision and fixed fixture manifest are unchanged. The protected
comparator now qualifies native stack-guard results by bounded same-input,
same-stack reruns requiring exact successful outputs, rather than matching
physical exhaustion columns. Raw runs are retained; ordinary course checker
requirements and the PA34 byte-equality proof remain required. The harness,
contract, prompt/goal sidecars and protected hashes are updated together.

## Parallel course gates — 2026-10-14

Owner enabled eight regular course workers in both clone settings copies and
installed the matching protected checker from Ralph b5f5c59. Comparison policy,
frozen cases, oracle, compiler implementation and bootstrap transport are unchanged.
A fixed PA1–PA10 snapshot measured 227s with one course worker and 74s/71s with
eight. All 1,156 focused cases and 2,046 course invocations passed with identical
counts and compiler-output hashes. The installed 30-test harness suite passed.
Benchmark evidence: `/home/vishvananda/work/private/v5rust/artifacts/prior-parallel-benchmark`.


## Complete stage inventory — 2026-10-19

The protected stage command previously executed only 181 frozen anchors while
Ralph's worktree inventory counted additional executed regressions. The resulting
unknown tail prevented the preservation gate from accepting an otherwise valid
partial checkpoint audit. This is a checker inventory defect, independent of
Scratch's reference-lowering corrections.

Ralph `1004cf7` retains all frozen anchors, including missing files as required
cases, and adds every discovered `paN/tests/**/*.t` once. Stage, prior/through,
focused-case and oracle-self checks share the inventory; added cases use the
same exact comparator and passing-identity preservation. Both script/test copies
are synchronized; only their two reviewed protected hashes changed. The oracle,
immutable anchors, Rust implementation, spec, guide and source map are unchanged.

Validation: 32 harness tests and protected preflight pass. The fresh candidate
stage check reports **192/193**, with exactly the same sole unfinished case,
`tests/general/100-initializer-list-without-header-bad.t`. All 192 previously
accepted passing identities are retained. Oracle self-check passes **193/193**.
Raw per-case traces, results, checks and pre-recovery state are saved under
`/home/vishvananda/work/private/v5rust/artifacts/owner-stage-inventory-20261019/`.

Record this observed baseline and retry the required checkpoint audit with a
fresh native goal. Keep the old blocked goal and failed-attempt history. No
stage advancement, goal completion or implementation progress is inferred from
the reporting correction. The audit may reuse qualified evidence for unchanged
code and must leave the unfinished initializer-list implementation visible.
