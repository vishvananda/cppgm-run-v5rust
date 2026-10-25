# CPPGM Programming Assignment 35: Optimizing the compiler

Complete PA34, then make the compiler you built fast in two senses: the code
it generates, judged by compiling `cppgm++` with itself and running the
result, and the work `cppgm++` itself does, judged against the host compiler
on the same input. PA35 adds no language feature or output format.

The yardstick for generated code is the host compiler. The **seed** is
`dev/cppgm++` built by the host (`make build`, `g++ -O3`). The **self**
compiler is the same source compiled by the seed with the same flags and
linked by the host exactly like the seed. Seed and self compile the frozen
workload to identical objects, so any difference in the work they execute
comes from your compiler's code generation and optimization.

## Completion criterion

From the repository root:

```sh
make -C pa35 test
```

The command passes every gate in [`gates.txt`](gates.txt) and prints
`===== ALL TESTS PASSED SUCCESSFULLY! (17/17) =====`. Optimization must keep
the correctness established by earlier assignments:
`make test-report-through-pa33` and `make inception` continue to pass.

PA35 is not part of `make test-report`. Its gates compare your compiler with
the host and with its own seed, the speed gates time real compiles, and the
codegen gates run the workload under cache simulation, so a full check takes
minutes rather than seconds.

## The workload

`pa35/workload/` holds the frozen compile workload: `semantic_overload.cpp`
(about 9,300 lines of C++11 from a semantic analyzer) and the 51 project
headers it includes (about 16,000 lines). With libstdc++ it preprocesses to
roughly 100,000 lines. `MANIFEST.sha256` records every file, and the scripts
refuse to measure a modified workload. The host's standard-library headers
belong to the machine, so compare measurements only on one machine and
toolchain. Every measurement compiles it the same way:

```sh
cppgm++ -O<n> -c pa35/workload/semantic_overload.cpp -I pa35/workload/include \
  -U__LDBL_MAX__ -D__LDBL_MAX__=1.0e300L -U__LDBL_MIN__ -D__LDBL_MIN__=1.0e-300L \
  -U__LDBL_DENORM_MIN__ -D__LDBL_DENORM_MIN__=1.0e-310L \
  -U__LDBL_NORM_MAX__ -D__LDBL_NORM_MAX__=1.0e300L -o out.o
```

Valgrind emulates x87 arithmetic with 64-bit doubles, so the extreme
`long double` limits would overflow under it; the four macros are replaced by
representable values for every compiler and every run. The workload is
ordinary C++11: if your compiler rejects or miscompiles it, fix the compiler
first, with a reduced reproducer under `student.tests/`.

## Gates

`make -C pa35 test` runs `scripts/check.pl`. The gates form three groups,
checked in order; a failing group stops the check and the later groups are
reported as not run. `make -C pa35 test-correctness`, `test-speed` and
`test-codegen` check one group.

| Group | Gate | Measurement | Limit |
|---|---|---|---|
| correctness | `workload-compiles` | the seed compiles the workload at `-O0` and `-O3` | required |
| correctness | `behavior` | each `tests/behavior/*.cpp`, compiled by `cppgm++` at `-O0` to `-O3` and linked by the host, prints what the host build prints; its `-O3` LowIR lowers through `lowir2native` with a MIR dump | all pass |
| correctness | `generated` | seeded programs exercising narrow values, Booleans, returned objects, frames and unwinding, at `-O0` to `-O3`, against the host build | all pass |
| speed | `compile-O0-time` | seed `-O0` user time on the workload / host `-O0` | <= 0.50 |
| speed | `compile-O3-time` | seed `-O3` user time / host `-O3` | <= 0.50 |
| speed | `compile-O3-memory` | seed `-O3` peak RSS / host `-O3` | <= 1.00 |
| speed | `compile-scaling` | largest growth of the seed's compile time per doubling of input size, over the synthetic families of `scaling.pl` | <= 3.0 |
| codegen | `workload-identical` | seed and self (frozen with `scripts/freeze.pl --label check`) write byte-identical objects for the workload at `-O0` and `-O3` | required |
| codegen | `codegen-O0-instructions` | self / seed instructions executed in the compiler binary's own code (callgrind), workload at `-O0` | <= 1.08 |
| codegen | `codegen-O3-instructions` | the same at `-O3` | <= 1.12 |
| codegen | `codegen-O0-cycle-proxy` | self / seed cycle proxy for the whole process at `-O0` | <= 1.09 |
| codegen | `codegen-O3-cycle-proxy` | the same at `-O3` | <= 1.10 |
| codegen | `text-size` | self / seed `.text` bytes | <= 1.15 |
| codegen | `kernel-*` (4) | each kernel in `kernels/`, compiled by `cppgm++` and by GCC and linked against one GCC-built driver: instructions for the measured input relative to GCC | <= 1.30 each |

The cycle proxy is `Ir/3 + 15 Bcm + 20 Bim + 10 I1mr + 10 (D1mr + D1mw)` from
callgrind's cache and branch simulation: deterministic, and closer to time
than instruction counts. The kernels isolate common weaknesses: an accessor
chain 64 calls deep around an opaque call in a loop (`-O2`, distinct and
aliasing pointers), an open-addressed hash lookup (`-O3`), and loop state
carried across calls and a `try` region (`-O3`). GCC and clang pass every
kernel limit. `scaling.pl` compiles generated inputs of growing size (many
scopes and lookups, a class with many members, many template
instantiations, many macros, one long function, many small functions):
linear work grows by about 2 per doubling, quadratic work by about 4.

The compile-time gates require at most half the host's user time at matching
optimization levels. Continue improving compile time and generated code after
meeting the acceptance limits.

Most limits are far from where a compiler starts, so a gate alone shows no
progress until it passes. `make -C pa35 test-progress` reports how far each
gate is as rungs of a ladder: `gates.txt` gives a measured gate a `from`
value and a step, and the rungs sit at the limit times `(1 + step)^k` up to
`from`. A measurement passes every rung at or above it, the last rung is the
gate itself, and steps follow measurement noise (0.5% for the workload's
instruction and cache-simulation counts, 1% for the kernels and text size,
2% for memory, 5% for timed runs). The command runs nothing: it
reads the measurements the last `test-correctness`, `test-speed` and
`test-codegen` (or `make -C pa35 test`) left in `pa35/obj/check`, counting a
group only when it was measured after the groups before it, and prints one
`<gate>: PASS|FAIL (n/m)` line per gate and the summary line over all rungs.

The gates use deterministic counts because wall time on a shared machine
moves by several percent between identical runs. Time remains the objective:
`make -C pa35 abba` compares self and seed with paired timing, and a seed
against a copy of itself measures run-to-run noise.

## Work loop

A full `make -C pa35 test` takes several minutes, because the cycle proxy
runs the workload under cache simulation at about 100 times its native
cost. Use the cheaper checks while working:

```sh
make -C pa35 quick                       # one change: seed, self, both levels
perl pa35/scripts/ab.pl pa35/obj/bin/seed-before   # one codegen change at constant work
make -C pa35 test-smoke                  # correctness: behavior, five generated seeds
make -C pa35 test                        # all gates
make -C pa35 test-progress               # rungs passed toward each gate, from the last gate runs
```

`make -C pa35 quick` rebuilds the seed, builds self from the same source,
counts both on the workload at `-O0` and `-O3` (instructions only), runs the
kernels at reduced iteration counts and the behavior tests, and prints the
results beside those of the previous quick run. `LEVELS=O3` limits it to one
level. On a machine with user-mode hardware counters (`perf stat -e
instructions:u`) the counts come from them at nearly native speed; otherwise
callgrind runs at about 30 times native speed. Results are cached by binary
hash in `pa35/obj/cache`, so an unchanged binary is never measured twice.

`make -C pa35 test-smoke` runs the correctness group alone, with five
generated seeds, into `pa35/obj/smoke` (it leaves the measurements that
`test-progress` reads alone). It runs no simulation and takes seconds once
your compiler is fast; run it after every optimizer or backend change.

`ab.pl OLD [NEW]` builds the current source with compiler `OLD` and with
`NEW` (default `dev/cppgm++`) and compares the two builds on the workload.
Both run the same source and do the same work, so the difference is the
generated code alone. Use it for optimizer changes: they also change the
work the workload's compile does, for the seed and self alike, so a fresh
seed/self ratio mixes two effects. Freeze a reference first
(`make -C pa35 freeze LABEL=before`).

Every measurement scales with the work your compiler does. Each compile in
`speed.pl` and `scaling.pl` is bounded in time and memory, and while the seed
is slower than the host the codegen group skips the simulated runs and
reports those gates as not measured. The scripts run as many jobs as the
machine has processors (at most 32); set `PA35_JOBS` to use fewer on a
shared machine.

## Compiler speed

Work in this order: the workload compiles; no input family grows faster than
linearly and compile time is within reach of its gates; then generated code;
last, a compiler-speed pass that leaves generated code unchanged. Front-end
changes rewrite the code the codegen gates measure, so codegen numbers taken
before them are spent twice, and a fast compiler makes every later
measurement cheaper.

```sh
make -C pa35 speed                       # -E, -O0 and -O3 against the host
make -C pa35 scaling                     # growth per doubling, against the host
make -C pa35 profile [LEVEL=-O0] [INPUT=file.cpp]   # where the time goes
```

`speed.pl` compares matching levels (`-O0`/`-O0` and `-O3`/`-O3`). It reports
preprocessing (`-E`) separately from whole compiles, which
separates the preprocessor from the rest of the front end. `scaling.pl --keep
DIR` keeps its generated sources; profile the smallest one that is already
slow. `profile.pl` samples at native speed and stops after a time limit, so a
compile that blows up still shows where its time goes; `--exact` uses
callgrind for exact costs and allocation counts.

## Focused builds and checks

All scripts live in `pa35/scripts/` (Perl, plus a small C timing helper the
scripts build). Each starts with a usage comment.

| Command | Purpose |
|---|---|
| `make -C pa35 freeze LABEL=L` | freeze `pa35/obj/bin/{seed,self}-L` from the current tree |
| `make -C pa35 measure LABEL=L [SIM=1]` | callgrind on seed-L and self-L at `-O0` and `-O3`: own-binary instructions, mix by instruction class, call and function excess, in `pa35/obj/results/L` (`L-sim` with `SIM=1`) |
| `make -C pa35 abba LABEL=L [LEVEL=-O3]` | paired timing of seed-L and self-L |
| `perl pa35/scripts/build_with.pl --compiler C --out DIR` | build `cppgm++` from this source with any compiler |
| `make -C pa35 identity OLD=A NEW=B` | whether two compilers generate identical code for every `cppgm++` object and the workload, and their compiler work |
| `make -C pa35 kernels` / `behavior` / `generated` / `speed` / `scaling` | one gate family, using `dev/cppgm++` |

The gate scripts take `--compiler C` (and `check.pl` takes `--seed A --self
B`), so they also run against the reference tools: for example
`perl pa35/scripts/kernels.pl --compiler reference-binaries/cppgm++` after
`make reference-binaries`.

`freeze.pl`, `build_with.pl` and the codegen checks take the seed's compile
and link lines from a dry run of `make build`: keep a `make build` that
compiles each `dev/src` object with `-c` and links `cppgm++` (or
`cppgm++.tmp`, which the shipped Makefile then moves into place) from those
objects, so the self build can repeat the same lines with another compiler
and the same host link. An `OBJ=` override or another object root is fine.

### Comparing with the reference

Run `make -C pa35 parity` after `make reference-binaries` to compare your
compiler with the provided reference on your own machine. It measures both
side by side: compile time and
memory on the workload (the figures themselves, with each compiler's ratio
to the host beside them), and generated code at constant work, since both
compilers build this repository's own source and the two builds run the
workload (`ab.pl reference-binaries/cppgm++`; `SIM=1` adds callgrind's
cycle proxy). It prints the reference's figure, yours and the ratio, and
says when a figure is missing. Use the limits in `gates.txt` to determine
whether your implementation meets the assignment requirements.

## Boundaries

- **Attribute a codegen gain to the optimizer and backend.** The
  generated-code gates compare two builds of the same source, so rewriting
  the compiler's hot paths to avoid a code-generation weakness changes both
  and proves nothing about the code generator; `ab.pl` (one source built by
  an old and a new compiler) shows what a change did to the generated code
  alone, and `identity` whether a compiler-speed change left it untouched.
  A change may improve both compile time and generated code; measure each
  effect on its own.
- **No profile feedback, link-time optimization or ISA extensions.** The self
  build may use only what one `cppgm++ -O3 -c` of one translation unit can
  know, as the seed's `g++ -O3` does.
- **Earlier contracts stay intact.** `-O0` output still meets PA24's
  contract. PA32 and
  PA33 fixture budgets, MIR dumps that describe what is encoded, debug
  locations and the LowIR boundary keep their meaning. Seed, self and
  inception emit identical bytes.
- **No special cases.** Do not recognize the workload, kernel or test
  sources, file names, symbol names or library type spellings.
- **Same allocator.** `freeze.pl` and `build_with.pl` link self like the seed;
  PA34's `cppgm++-self` may link a different `malloc` and is not comparable.

PA35 does not require vectorization, profile feedback, link-time or
whole-program optimization, a new IR, or any particular allocator, pass
order or inliner.

## Correctness

The `behavior` programs come from legality reviews of transformations like
the ones this assignment invites: narrow values, object words, loop loads
across calls that may free, zero-trip loops, unrolled exits, variadic
arguments through inlining, unions and type-based aliasing, `setjmp`,
shrink-wrapped frames and their unwinding. Each was a real miscompile in
some compiler. Every program is free of undefined behavior (GCC and clang
builds under UBSan and ASan run clean and agree), so a difference is a
compiler bug. Add a reduced program under `student.tests/` for each new
transformation, run `perl pa35/scripts/generated.pl --seeds 2000` after any
allocator or machine-IR change, and run `make test-report-through-pa33` and
the inception build regularly, not only at the end: a miscompile that the
gates do not reach can otherwise live for many steps.

## Design notes (non-normative)

The earlier assignments' design notes describe the structures this work
stands on: an in-memory LowIR that a function can edit in place, with every
fact spelled and dense numbering (PA8), and the facts the lowering states
(`lowir.md`, "Facts The C++ Lowering States"); an exception shape with
regions closed before joins (PA21); a machine IR whose instructions state the
registers they define, use and clobber, with placement decided apart from
selection and the frame finished last (PA24); unwind tables generated from
the frame facts the encoder records (PA26); an optimizer that iterates per
function and keeps its own work bounded (PA32); and a frequency-weighted
allocator (PA33). Where one of those is missing, building it is usually the
first and largest step.

Measure both workload levels from the start: `-O0` mostly exercises the front
end and instruction selection, `-O3` the optimizer's own code, and the two
gaps can differ widely. Use instruction counts to iterate and paired timing
to confirm; frame homes, taken branches and store-forwarding stalls cost more
time than their instruction counts suggest, while register moves and
well-predicted branches cost almost nothing. Small counted wins add up even
when no single one is visible in time.

Attribute before changing anything. The excess by instruction class usually
names the mechanism: frame loads and stores point at allocation, callee-saved
saves at callee-saved policy and shrink-wrapping, register moves at
coalescing, jumps and compares at block placement and switch lowering,
extensions at narrow-value handling, calls at inlining. Compare functions one
by one only when their call counts match in seed and self; otherwise inlining
has moved cost between them.

Incremental work depends on a few foundations. Check them first:

- register assignment separate from instruction selection, over a machine IR
  that records each instruction's defs, uses, clobbers and tied operands, so
  allocation and later cleanup passes can work on the whole function;
- an optimizer that can rewrite a function in place and iterate, so
  simplification, promotion and inlining feed each other;
- no blanket exclusions such as "no dataflow for functions with a call inside
  a loop" or "no promotion in functions with cleanups": they exclude most hot
  code, and usually hide a weakness in a later stage that should be fixed
  where it lives;
- a correctness net as strong as the changes: reduced tests for each new
  transformation, and many generated seeds after allocator or machine-IR
  changes (`generated.pl --seeds 2000`).

Useful directions, in the order the measurements usually point:

- allocation weighted by estimated block frequency rather than loop depth,
  with coalescing, splitting around loops and clobbering blocks,
  rematerialization, callee-saved costs per function entry, shrink-wrapping,
  and allocation inside functions that handle exceptions;
- selection that fuses compares with branches, folds addresses and loads
  into memory operands, keeps narrow values unextended where possible, and
  expands small copies, switches and constant divisions well;
- inlining decided on simplified callees, bottom-up, without cascades: raising
  a global size cap makes callees grow past their own callers' caps;
- block placement, loop rotation and jump threading;
- alias facts, scalar replacement of small aggregates, and copies that do not
  reload just-stored narrow fields as wider words.

Prefer fixing an over-conservative model (a constraint stricter than emission
needs) to tuning heuristics, which usually nets little. Re-measure a dropped
idea after related changes land. PA32 and PA33's fixtures require specific
outcomes and upper bounds. If a broad change breaks one, investigate the
extra work or lost simplification before adjusting the change.

Compile time that grows faster than its input usually comes from per-query
work proportional to the whole translation unit: a lookup, overload check or
instantiation that scans every scope, declaration or specialization, or
allocates a structure sized to them; or from repeated work: re-parsing text,
re-instantiating, re-checking, or lowering and emitting definitions nothing
uses. A front end that is uniformly slow usually pays for its representation:
per-character lexing with per-character records, names compared and hashed as
strings, a hash lookup on every syntax-tree edge, facts kept in side tables
instead of on the nodes, fat value types copied around, and allocation per
node. Those need representation changes, not tuning. In the optimizer, look
for passes that report changes they did not make, analyses recomputed for
unchanged functions and structures rebuilt after local edits. Keep
compiler-speed changes free of any effect on generated code, and check that
with `make -C pa35 identity`, so speed and codegen effects stay separable.
