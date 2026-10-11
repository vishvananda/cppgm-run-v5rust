## CPPGM Programming Assignment 32 (`lowiropt`)

### Overview

PA32 adds the first explicit optimization stage to the compiler. The new tool,
`lowiropt`, reads PA8 LowIR text, applies a deterministic optimization
pipeline selected by `-O0`, `-O1`, `-O2`, or `-O3`, and writes LowIR text.

The same LowIR optimizer is also reached from `cppgm++` when source programs
are compiled with `--emit-lowir -O1`, `--emit-lowir -O2`,
`--emit-lowir -O3`, or through the ordinary compile/link driver at an
optimization level.

### How PA32 Is Specified

This README separates three kinds of statement, and the course suite is
built the same way.

- The **contract** is what every implementation must do: the command line,
  the LowIR it reads and writes, the behaviour it must preserve, and the
  object path that must accept its output.  Command Line, Output Format,
  Error Handling and Validation Modes are the contract.
- The **quality bar** is what the output must reach on the course fixtures:
  the outcome and the size envelope each fixture states in its
  `x.ref.expect` sidecar, and the lowering floor.  The Quality Bar section
  is normative.
- **One design** is how the course solution meets that bar, pass by pass,
  with the budgets and caps it chose.  The linked design notes are a worked example.  A different optimizer that meets the bar is a correct PA32; a
  course fixture never compares the shape of your output with the course
  solution's.

Run `make test` from this assignment directory to check the course contract.
See [Testing and references](../TESTING_AND_REFERENCES.md) for selecting tests.

### Prerequisites

You should complete PA31 before starting this assignment.

You will reuse:

- the PA8 LowIR syntax and semantics
- the PA10 through PA31 source-to-LowIR lowering pipeline
- the PA24 native backend, PA25 driver, PA9 ABI naming, and PA26 host-runtime path
- the PA31 hosted compiler driver surface

### Starter Kit

The starter kit supplies:

- `pa32/Makefile`
- `pa32/lowiropt.cpp`, linked to the editable `dev/lowiropt.cpp`
- a student-editable `dev/lowiropt.cpp` starter scaffold
- shared compiler support under `dev/src/`
- test directories under `pa32/tests/`
- harness scripts under `pa32/scripts/`
- checked-in `.ref` and `.ref.exit_status` files for the tests

The expected implementation work is in `dev/lowiropt.cpp` and shared optimizer
or driver support under `dev/src/`, especially the LowIR optimizer and
optimization-level plumbing. Reuse your PA8 LowIR reader/writer and later
driver implementation; the supplied harness does not implement these passes.

Use `lowiropt-ref` to inspect example output.
The harness checks the contract sidecars and quality envelopes.

### Command Line

`lowiropt` accepts exactly one optimization level, one output path, and one or
more LowIR input files:

```sh
lowiropt -O0 -o <outfile> <lowirfile>...
lowiropt -O1 -o <outfile> <lowirfile>...
lowiropt -O2 -o <outfile> <lowirfile>...
lowiropt -O3 -o <outfile> <lowirfile>...
```

`--help` and `-h` print usage information and exit successfully.

PA32 also requires the source driver to route these options through the same
optimizer:

```sh
cppgm++ --emit-lowir -g0 -O1 -o <outfile> <srcfile>...
cppgm++ --emit-lowir -g0 -O2 -o <outfile> <srcfile>...
cppgm++ --emit-lowir -g0 -O3 -o <outfile> <srcfile>...
cppgm++ --emit-lowir -gline-tables-only -O1 -o <outfile> <srcfile>...
cppgm++ --emit-lowir -gline-tables-only -O2 -o <outfile> <srcfile>...
cppgm++ --emit-lowir -gline-tables-only -O3 -o <outfile> <srcfile>...
```

The ordinary `cppgm++ -c` and link-driver paths must also accept `-O0`, `-O1`,
`-O2`, and `-O3` and use the same LowIR optimization level before object
generation. As with GCC and Clang, omitting `-O` selects `-O0`; optimization
must be requested explicitly.
Compile mode must also accept serialized LowIR text as an input:

```sh
cppgm++ -c -O0 -o <objfile> <lowirfile>
cppgm++ -c -O1 -o <objfile> <lowirfile>
cppgm++ -c -O2 -o <objfile> <lowirfile>
cppgm++ -c -O3 -o <objfile> <lowirfile>
```

This LowIR object input mode parses LowIR text, runs the same object-prep and
optimization path used by source object compilation, and writes the same
host-compatible relocatable object format.

### Output Format

`lowiropt` writes LowIR text to `<outfile>`. The output must remain valid LowIR
and must preserve the behavior of every defined input program.

The optimizer works on the same LowIR program representation that the object
path consumes. It may use typed internal data structures, but optimized output
must serialize back to valid LowIR, and object generation at a chosen
optimization level must not require extra semantic facts unavailable from that
optimized LowIR text.

This LowIR/object boundary is required in PA32. A correct compile path
may keep LowIR in memory for speed, but it must not pass private frontend or
semantic side data around the serialized LowIR representation. If object
emission needs a fact after optimization, that fact must either be represented
in LowIR or derived again by the object-lowering layer from LowIR. The direct
`cppgm++ -c` source object and the object produced by `--emit-lowir -O0`
followed by `cppgm++ -c -O<level>` on that LowIR file should therefore match
for the same source, flags, and optimization level.

This durability rule includes PA27 global section placement. A token-safe GNU
section attribute is serialized as global `section=<name>` metadata and must
survive `-O0` through `-O3`. The replayed object must keep the global in the
same named ELF section and keep relocations originating in that section aimed
at the same symbols; direct/replayed byte equality alone is not the feature
definition.

Your output must be deterministic and follow the [LowIR format](../pa8/lowir.md).
Optimization may change instructions and control flow while preserving their
meaning, including initialization, destruction, volatile accesses and cleanup.
`-O0` preserves semantic content without optimization. At higher levels, the
fixture expectations define the required simplifications and size bounds; the
choice and order of passes are yours.

### Error Handling

The tool must fail with a nonzero exit status when:

- no optimization level is provided
- `-o` is missing or has no following path
- there are no input files
- an input file cannot be read
- the input is not valid LowIR
- the output file cannot be written

For failure cases, diagnostics only need to be useful to a developer; exact
diagnostic text is not part of the grading contract. The contents of the
output file after a failed run are undefined.

### Quality Bar

The levels have this contract.  `-O0` parses the input program, preserves all
semantic content, and writes the canonical LowIR dump without running
optimizing transforms.  `-O1`, `-O2` and `-O3` each write valid LowIR that
preserves the behaviour of every defined input program; each higher level
may do everything the lower one does and more.  What the levels do is
your design.

What the course fixtures then check, for each `x.t`:

- the tool's exit status agrees with `x.ref.exit_status`;
- a successful output passes the LowIR structural validator;
- the output meets every predicate in its `x.ref.expect`
  (`../scripts/expect_ir.pl`);
- the output lowers through the object path at `-O0`
  (`cppgm++ -c -O0 x.lowir`) whenever the input does.  `make test` sets
  `CPPGM_LOWER_CHECK_APP` so the harness runs that floor; an optimizer's
  output is not correct until the backend accepts it;
- the optimized program behaves as the input's: both are built through the
  reference backend (`lowir2native-ref -O0`) and run, and the optimized one
  must exit the same way and print the same output (`make test` sets
  `CPPGM_LOWIR_BEHAVIOR_APP`).  An entry without parameters runs as
  written.  Otherwise a generated driver is the entry: it calls a
  parameterized entry with fixed scalar arguments or, when the fixture has
  no entry, every function of external binding whose parameters and result
  are scalars, and returns the sum of their results.  A helper the fixture
  declares and nothing defines gets a generated provider that returns zero,
  and when its declaration allows unwinding both programs also run with a
  provider that throws, so the handlers run too.  An `x.driver.lowir` beside
  a fixture replaces the generated driver and providers: it is appended as
  written, its `@main` is the entry, and the fixture's own `@main` is renamed
  `@__behavior_entry`.  The lane's summary line says how many inputs ran,
  how, and why the others did not (a pointer parameter or result, a helper
  no provider can define, a program that does not finish); an input the
  reference backend cannot build fails the lane.

An `x.ref.expect` sidecar has two kinds of line.  Outcome lines state what
the fixture is about in terms of the LowIR it reads and writes:
`none(call @callee) in @caller` for a call that must be inlined,
`count(load) <= 1 in ^body` for a load that must be hoisted,
`none(phi) in ^merge` for a choice that must collapse.  Budget lines bound
the output's size: `instructions <= N`, `blocks <= N`, `count(call)`,
`count(load)`, `count(store)`, `count(phi)`, and `instructions <= N in
@function`.  The budgets were generated from an earlier course solution's
output at 10% tolerance plus one. Each sidecar states the actual required bounds
and outcomes. The adjacent `x.ref` is an informational example; matching its
instruction sequence is not required.

A source fixture in a driver lane is also run: the emitted LowIR is built
into a program through the object path and compared with the source built
at the lane's level (`CPPGM_BEHAVIOR_CHECK_APP`, set by `make test`); both
must print the same output and exit the same way.  A source that does not
build at that level is skipped.

To evaluate one sidecar by hand:

```sh
$ perl ../scripts/expect_ir.pl tests/o1/x.my tests/o1/x.ref.expect
```

### Design Notes (Non-Normative)

The fixtures are small; the optimizer's real input is PA34's self-build of
the whole compiler, where functions are large, calls sit inside loops and
most bodies have cleanups. A few principles matter there more than any one
pass:

- Run the passes over each function repeatedly, to a fixed point or a
  budget, rather than once over the program. Inlining exposes constants that
  folding removes, which exposes branches that simplification removes, which
  shrinks callers below the inlining threshold.
- Count what every admission rule excludes. A rule that skips a function
  for its size, its exception handling or a call in a loop costs nothing on
  the fixtures and may exclude most of a real program; know the fraction.
- Keep facts that cost nothing to keep. `typed`, `nowrap` and `exact` come
  from the lowering (`../pa8/lowir.md`, "Instruction Facts"); an optimizer
  that preserves them through copies and drops them where it speculates can
  use `typed` to keep a value in a register across a store of another type.
- When an optimization makes the output slower because the backend lowers
  the result badly, fix the backend. Withholding the optimization leaves both
  problems in place, and the rule that withholds it has to be found and
  undone once the backend improves.
- Know what blocks inlining in real code. A callee that contains exception
  instructions is the common case (any destructible local and a call), and
  LowIR's handler edges are implicit (see PA21's design notes). The course
  solution inlines such a callee only when constant arguments fold its
  exception instructions away; an optimizer that tracks the handler stack can
  admit a callee whose regions leave it balanced.
- Keep the optimizer's own work bounded. PA34 compiles the whole compiler
  at `-O3` with this optimizer; [the design notes](design-notes.md) describe
  how the course solution avoids repeating work.

No course fixture pins a cap: a sidecar states what must be simplified
(outcome lines) and how large the output may be (budgets with room above
the course solution's counts), and an upper bound on a function the
optimizer removed holds. The course solution's own caps (which loop callees
stay calls, what a single-call callee may cost, when a small callee made of
calls stays out of line, how far a loop unrolls) are pinned only in
`tests/regression`, outside your exit criteria.

### Design example

[Design notes](design-notes.md) describe the course solution’s passes and
implementation choices. They are a worked example, not additional requirements.

### Validation Modes

Successful outputs must pass the LowIR structural validator, the fixture’s
`.ref.expect` predicates, the object-lowering check and, for inputs with
an entry function, the execution check through the reference backend.
Source-driver cases also compare execution before and after LowIR
serialization. Failed cases are checked by exit status. See Quality Bar
for the sidecar syntax.

### Testing

From this assignment directory, run:

```sh
make test
```

Before moving on, run `make test-report-through-pa32` from the repository root.

`make test` runs:

- `tests/o0`
- `tests/o1`
- `tests/o2`
- `tests/o3`
- `tests/driver/o0`
- `tests/driver/o1`
- `tests/driver/o2`
- `tests/driver/o3`
- `tests/object-roundtrip`


These directories are organized by tool mode and validation mode, not by N3485
source-language clauses.

- `tests/o0` runs `lowiropt -O0` on handwritten LowIR.
- `tests/o1` runs `lowiropt -O1` on handwritten LowIR.
- `tests/o2` runs `lowiropt -O2` on handwritten LowIR.
- `tests/o3` runs `lowiropt -O3` on handwritten LowIR.
- `tests/driver/o0` runs `cppgm++ --emit-lowir -g0 -O0` on source programs
  whose boundary facts come from attributes that PA27-PA29 introduce (GNU
  `noinline`, `pure` and `const`, `[[noreturn]]`): the C++ lowering's own
  output, compared as PA10-PA23 compare theirs, facts included.
- `tests/driver/o1` runs `cppgm++ --emit-lowir -g0 -O1` on source programs.
- `tests/driver/o2` runs `cppgm++ --emit-lowir -g0 -O2` on source programs.
- `tests/driver/o3` runs `cppgm++ --emit-lowir -g0 -O3` on source programs.
- `tests/object-roundtrip` compares direct `cppgm++ -c` output against an
  object produced by `cppgm++ --emit-lowir -O0` followed by `cppgm++ -c` on
  the generated LowIR file. This checks that object emission can be
  reconstructed from serialized LowIR instead of from hidden frontend side
  data. These tests may be standalone `.cpp` files or symlinks to existing
  `.t` harness cases; a selected `.t` test expands to its numbered `.t.1`,
  `.t.2`, ... source files when those sidecars exist. The harness checks
  no-debug objects at `-O0`, `-O1`, `-O2`, and `-O3`. A
  GNU-section reducer additionally checks the global-section and relocation
  relationships in both objects rather than relying only on byte equality. A
  `default-no-optimization` case also checks that omitting `-O` matches
  explicit `-O0`. A parameter-object-extent case additionally makes O3 object
  shape depend on a source-produced member-object boundary, so byte equality
  checks compiler-object and textual LowIR transport of that fact.

Run the debug metadata preservation lanes with:

```sh
make test-debuginfo
```

This also runs `tests/object-roundtrip` in debuginfo mode, comparing direct
`cppgm++ -c` output against LowIR-input `cppgm++ -c` output with
`-gline-tables-only` at `-O0`, `-O1`, `-O2`, and `-O3`.

`make test-debuginfo` runs:

- `tests/debuginfo/o1`
- `tests/debuginfo/o2`
- `tests/debuginfo/o3`
- `tests/debuginfo/driver/o1`
- `tests/debuginfo/driver/o2`
- `tests/debuginfo/driver/o3`

The direct debug-info tests run `lowiropt -O*` over LowIR containing
`!dbg(...)` metadata. The driver debug-info tests run
`cppgm++ --emit-lowir -gline-tables-only -O*` and check that source locations
survive the source-to-LowIR optimizer path: the locations of
`../pa8/lowir.md` "Debug Locations", which each surviving, moved or copied
instruction keeps.

For each `.t` test, the harness records the tool exit status and compares the
generated output against the oracle for that test directory. Failed reference
cases are judged by exit status; successful reference cases are judged by the
directory's LowIR validation mode. Outside `tests/driver/o0`, the comparison
validates the facts of `../pa8/lowir.md` "Facts The C++ Lowering States" but
does not compare them: an optimizer may add a fact it proves, such as
`unwind=no` on a function whose body cannot throw, or drop one.

### Out Of Scope

PA32 does not require:

- input programs to arrive in SSA form
- PRE that requires critical-edge splitting, speculative trapping operations,
  or an unbounded insertion/fixed-point schedule
- alias-driven aggressive dead-store elimination
- partial unrolling, peeling, vectorization, or loop transformations beyond
  the bounded transformations needed by the fixture expectations
- machine-IR scheduling or register-allocation optimization
- unbounded interprocedural cloning, externally observable ABI changes,
  semantic body merging, or indirect-call specialization
- size-specific `-Os` or `-Oz` behavior

### After PA32

Later optimization tests build on this assignment by optimizing after LowIR has
already been lowered to machine IR. PA32 focuses on the LowIR optimization
pipeline and the `lowiropt` contract and quality checks.
