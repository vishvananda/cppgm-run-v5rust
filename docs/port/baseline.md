# Frozen preparation baseline

## Current oracle refresh — 2026-10-12

The current reference is cppgm-extended `0fe8e3b4bde9929c2419fb7906028df83bb17de5` (PR #65, merged).
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
