## CPPGM Programming Assignment 33 (`lowir2native -O*`)

### Overview

Improve the native code produced from LowIR while preserving program behavior,
the ABI, unwinding and debug information. PA32 optimizes LowIR; PA33 optimizes
its translation to machine instructions. Reuse this backend in both
`lowir2native` and the `cppgm++` object and executable paths.

Complete PA24 and PA32 first. Continue working in `dev/lowir2native.cpp` and
your shared backend under `dev/src/`. The assignment wrapper points there.
The supplied `mir_model.h` and register definitions are the MIR vocabulary
the course solution also uses; the MIR's containers are yours.
[Design notes](design-notes.md) suggest a starting approach.

### Required behavior

Support these forms at each of `-O1`, `-O2` and `-O3`:

```sh
lowir2native -O1 -o <program> <lowirfile>...
lowir2native -O1 --dump-machine-ir <mirfile> <lowirfile>...
lowir2native -O1 --dump-machine-ir <mirfile> -o <program> <lowirfile>...
```

`-O0` remains the PA24 baseline. `--help` and `-h` exit successfully;
`--target <target>` retains its PA24 meaning. Reuse the PA8 LowIR language,
including entry-role and other explicit metadata.

- **O1:** meet the bounds for local cleanup, such as redundant copies,
  argument setup, address calculations and unnecessary jumps.
- **O2:** also meet the bounds for values live across blocks and calls,
  loop register use and frame size.
- **O3:** handle PA32's O3 output through the optimized native backend.
  A separate O3 machine pass is not required if your O2 backend meets the bar.

Pass order, register allocation and instruction selection are your choices.
If your existing backend already meets a bound, no extra pass is needed for
that case. Completion depends on the output, not on adding named optimizations.
Higher optimization levels must preserve the correctness established by
previous assignments. In `cppgm++`, apply PA32's LowIR optimization before
native lowering at the requested level.

The MIR dump uses PA24's text format and describes the same instructions and
ABI/frame facts that produce the executable. When both outputs are requested,
they must come from the same optimized program. Keep call argument uses,
stack arguments, variadic and no-return annotations, exception boundaries,
callee-saved register information and indexed-address operands accurate after
rewriting instructions.

Fail with a nonzero status for missing inputs or output options, missing option
arguments, unreadable or invalid LowIR, an unsupported target, an unwritable
output, or failed code generation/linking. Exact diagnostic wording and output
files left after failure are not prescribed.

### How output is checked

Every native fixture checks compilation status and the generated program's
stdout and exit status. Correct output is required regardless of optimization
level or allocation strategy.

Where `x.ref.expect` exists, the MIR must satisfy its predicates. They bound
quantities such as instruction count, blocks, calls, memory operands and frame
size, and state focused requirements for effects such as volatile accesses
and exception operations. The numbers in the sidecar are the actual limits;
you do not need to reproduce the reference registers, offsets, instruction
order or block layout. A missing expectation means execution alone is checked.
The predicates count mnemonics as the PA24 dump spells them (`load.*` and
`store.*` for memory accesses, `eh_push` and `eh_pop` for regions, an
implicit prologue; see PA24's machine-IR text).
The `.ref.mir` and `.ref.cmir` files are examples, not optimized-output oracles.

These bounds provide a deterministic, lightweight code-quality bar. They are
not measurements of execution time: fewer static instructions do not always
mean faster code. No timing threshold, Cachegrind installation or allocator
statistics are required to complete PA33. Execution time is still what the
bar stands for: PA34 builds the compiler with itself, so the code this backend
generates becomes the compiler's own speed.

For a failed expectation, inspect the reported predicate or run:

```sh
perl ../scripts/expect_ir.pl tests/o1/<name>.my.mir tests/o1/<name>.ref.expect
```

Debug fixtures carry `!dbg(file, line, column)` annotations. Their predicates
check source locations on surviving calls and returns without comparing the
whole instruction sequence. Removed computations may lose their locations;
combined computations may use a contributing location. Preserve meaningful
locations on surviving operations and do not invent locations from another
function or source statement.

### Testing

From this directory:

```sh
make test
make test-debuginfo
make check TEST='tests/o1/100-return-copy-coalesce.t'
```

`make test` runs the O1, O2 and O3 MIR and behavior buckets and the
self-checking C++ programs under `tests/driver/`.
The driver programs must exit zero and write no stdout at all three levels,
when built directly, through PA25 compiler objects (`.obj`), and through PA26
native objects (`.o`) linked with `CPPGM_HOST_CXX`, and through the compiler's
LowIR output passed to the student's `lowir2native`. This also checks that
the two tools agree on the LowIR boundary. The programs check language and
lifetime behavior; they do not require diagnostic counters or a particular
backend design.

`make test-debuginfo` runs `tests/debuginfo/o1`, `o2` and `o3`. Both targets
must pass. Finish with `make test-report-through-pa33` from the repository root
before continuing to PA34's self-host ladder. See
[Testing and references](../TESTING_AND_REFERENCES.md) for test selection.

The numbered buckets group backend features. `tests/o1`, `o2` and `o3` check
code-quality bounds as well as execution; `tests/behavior/` adds correctness
cases and applies bounds only where an expectation is supplied.

### Scope

PA33 does not require a new LowIR optimizer, source-language changes,
interprocedural machine optimization, instruction scheduling or vectorization.
What it requires is meeting the supplied predicates while preserving earlier
contracts. The predicates reward keeping values in registers over their whole
live ranges, across calls (in callee-saved registers) and around loops, more
than local rewrites of a conservative frame form; a liveness-based allocator
over the PA24 machine IR is the direct way to meet them, and the one later
work keeps improving.
