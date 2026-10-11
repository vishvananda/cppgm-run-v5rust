# Frozen preparation baseline

Prepared 2026-10-11 from cppgm-extended commit
`72181d97e2a8b90af16229a328fc8e72bfe71f4b`.

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

The copied handout harness has one known focused assertion inconsistent with
its own pinned C++ implementation:
`pa33/tests/regression/controls/459-parameter-address-folding.t` fails with
`native O2 lost a post-call direct field load`, exit 2. Reproduced directly
in the unmodified reference checkout and through the differential adapter.
The manifest records that exact message, exit and four invocations. This case
still requires exact Rust/C++ bytes for every invocation. Its input and
checker are retained unchanged; the port must not alter code generation to
satisfy a stale structural expectation. The ordinary full strict report passes.

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
