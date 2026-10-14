# Testing and references

This guide is for the compiler solution repository. The student export has
[its own guide](docs/student-export-root/TESTING_AND_REFERENCES.md) and pinned
downloadable reference tools. Here, every `*-ref` wrapper invokes the matching
tool built from `dev/`.

## Run tests

Run from the repository root:

```sh
make
make test-pa8
make test-report-through-pa10
CPPGM_LOWIR_DIRECT_TEXT_COMPARE=1 make test-report
make test-debuginfo
make test-harness
make inception
```

The report covers PA1–PA33; PA34's final check is inception, and PA35's gates
are a release check ([below](#pa35-smoke-and-release-check)). Narrow the report
with `ACTIVE_TEST_REPORT_PAS='pa8 pa10'`. Control parallelism with
`TEST_REPORT_ASSIGNMENT_JOBS` and `TEST_REPORT_SUBTEST_JOBS`; `ORDERED=false`
streams completed jobs. Use `CPPGM_TEST_RUNNER=0` to diagnose ordinary versus
batch-runner behavior. Architecture audits and self-host checks are listed in
[AGENTS.md](AGENTS.md).

For quick iteration:

```sh
make -C pa8 check TEST='tests/spec/100-*.t'
```

`TEST=` accepts a path, quoted glob or quoted list. Mixed-input assignments
route each selected fixture to its owning runner.

## Fixture ownership

Every required fixture lives under its owning `paN/tests/`. From PA5 onward,
the three-digit prefix identifies a feature cluster (`100-`, `200-`, ...).
PA1–PA4 use finer subgroups. The placement auditor checks the latest feature
used; put a reducer in the earliest assignment that owns all its requirements.
Required `controls/` inputs follow the same feature-cluster prefixes and
earliest-owner rule. A control grades behavior or LowIR properties instead of
a complete reference module; its source and checker must stay within the
owning handout's requirements. Personal inputs and scratch
harnesses belong in `student.tests/` and are run explicitly.

A fixture must also sit where a lane runs it: a fixture beside the roots its
Makefile names ships to students and is never graded. `make test-harness`
runs `scripts/check_fixture_lanes.py`, which derives each assignment's lanes
from the recipes of its `test*` and `ref-test*` targets and fails on an input
no `test*` lane selects, or on a file under `tests/` that is neither an
input, a companion named after one (`x.ref*`, `x.t.N`, `x.lib.*`, ...), nor
a header or note that a fixture uses. Lanes a Makefile cannot show, such as
PA35's, are listed in the script with their reason.

Controls are `.t` cases in `tests/controls/`, run as the `control_t` shape by
`run_all_tests_common.pl` and `compare_results_common.pl`. They use the ordinary
run/check commands, result files, per-lane counts and failure reporting. Their
oracle is the stated behavior or focused predicate, so a complete `.ref` text
output is unnecessary. They contribute to assignment and cumulative totals.

In the LowIR-graded assignments (PA10–PA23), a control that states its
expected outcome is a self-checking program: `expect exit N` or
`expect reject` in its leading comment, or an `x.expect` sidecar (`exit N`
or `reject`, with optional exact output in `x.stdout`).
`scripts/check_controls.pl` compiles it with `cppgm++ --emit-lowir -O0`,
builds the LowIR with `lowir2native-ref -O0`, runs it and checks the status
and output, or that the frontend rejects it; the LowIR text is not compared.
A program killed by a signal fails whatever status it states (SIGABRT is not
`exit 134`), and a frontend killed by one, or stopped by an internal compiler
error (status 70), does not reject.
`make test` runs controls after the LowIR suite, `make -C paN test-controls`
runs one assignment's controls alone, and controls need no references. Use this lane for a runtime
reducer whose features the assignment owns. Host-linked, multi-unit,
optimized or g++-output-compared programs belong to PA25 and later. Controls
without a stated outcome stay with PA10–PA12's focused checkers.

PA24, PA32 and PA33 also have `tests/regression/`, which pins this solution's
particular output shapes. Maintainer `make test` runs both `test-course` and
`test-regression`; student-export `make test` runs the course contract only.
PA33's export omits its regression fixtures and `maintainer/` tooling entirely.
The source-only `pa33/maintainer/Makefile` retains census and Cachegrind
targets; these are not PA33 student requirements.
Keep solution-specific expectations out of graded course fixtures.

## Reference policy

Choose the oracle required by the handout: output, exit status, a focused
property, or a code-quality envelope. Diagnostic text is not graded. PA8's
three construction exercises grade program outcomes through `lowir2native-ref`,
allowing alternative valid LowIR. Informational IR/MIR sidecars are not oracles.

An expected `EXIT_FAILURE` is a rejection: a tool stopped by an internal
compiler error (status 70) or killed by a signal records `EXIT_INTERNAL_ERROR`
or `EXIT_SIGNAL` instead, and fails the case.

Regenerate references from the built solution:

```sh
make ref-test-pa8
make ref-test
make ref-test-debuginfo
```

Before changing a LowIR reference, distinguish behavior from presentation;
[pa8/lowir.md](pa8/lowir.md) defines what normal comparison absorbs. Maintainer
CI enables byte-exact LowIR comparison. Regeneration on an unchanged compiler
must leave tracked references unchanged. Never change a fixture or reference
merely to hide an implementation failure.

Failed-case stdout is an informational diagnostic example, generated on Linux
by the export and untracked here. Successful stdout and required exit-status
sidecars remain tracked. Export validation regenerates and verifies every
portable reference before packaging the student repository.
CI also installs the bundle in a fresh client and exercises every exported
reference wrapper. Publication repeats this check against the uploaded release
URL before pushing the student repository.

## Symbol names

The course names C++ entities as clang names them: clang is the oracle for
every mangled name, internal ones included, and GCC only a cross-check.
LowIR comparison relaxes `object=` spellings, so a wrong name fails no LowIR
fixture; check them against clang after changing ABI naming:

```sh
python3 scripts/check_clang_names.py --jobs 8
python3 scripts/check_clang_names.py --verbose pa20 pa21
```

The maintainer script compiles each C++ LowIR fixture's source with clang
(`-std=c++11 -O0`, `--clang` or `CLANG` selects the compiler) and compares
every symbol of the reference with clang's: a name spelled differently from
clang's name for the same entity is reported (an internal-linkage marker, a
closure or unnamed type, a local entity, an external name, or a LowIR
spelling standing for a C++ entity). An internal symbol whose name is not
mangled is compared with clang's local symbol for the same entity: a
temporary that a static-storage variable extends is compared with clang's
`_ZGR` name for that variable and position, and every clang `_ZGR`
temporary of a variable the reference defines must appear in it. Which
constructor and destructor entries a unit emits (C1/C2, CI1/CI2,
D0/D1/D2) is an emission choice and is not judged, nor is a symbol clang
does not emit. A signature whose clang spelling mangles an
instantiation-dependent expression (decltype, an expression template
argument or array bound) is compared like any other: the course retains
those expressions. The script exits 1 when a mismatch is not listed with
its reason in the script's `DOCUMENTED` table, and `--verbose` lists every
mismatch of a fixture.

## PA35 smoke and release check

PA35 has no reference files. Its gates compare at test time: generated code
with the compiler's own host-built seed and with GCC, compile time and memory
with the host compiler. What the course keeps from its solution is a
calibration.

`make -C pa35 test-smoke` runs the correctness group alone: the workload at
`-O0` and `-O3`, every `tests/behavior` program at `-O0` to `-O3` with its
MIR dump, and generated programs from five seeds, in seconds (8 at twelve
jobs here). Run it
with the other checks in [AGENTS.md](AGENTS.md) after a change to the
optimizer, the backend or the lowering. `make test-harness` includes
`scripts/tests/test_pa35_harness.py`, which checks the gate table and the
handout's limits and ladder steps, the progress ladder, the release check's
margins in both builds and the runners without the course compiler. Neither
`make test` nor `make test-report` runs PA35: like PA34 it is outside the
report, its speed gates time real compiles, and its cycle proxy runs the
workload under cache simulation, so a full run takes minutes.

`make -C pa35 ref-test` is the release check. It runs every gate on the
course build (`check.pl` with 100 generated seeds), then builds the same
source as the student export's Makefile does (plain `-O3`, the C library's
`malloc`; the seed by g++, self by that seed) and measures its speed and
codegen groups, the configuration students' gates compare. It writes
`pa35/calibration.tsv` (the student build's value and margin for each gate,
the measurements behind them, and the machine, toolchain and commit, named
generically) and `pa35/maintainer/calibration-course.tsv` (the course
build's). Commit both. The check fails when a gate fails in either build, or when a value
in either build does not clear its limit by the release margin of its kind
(margin = limit / value - 1):

| Kind | Gates | Release margin | Spread measured when PA35 entered |
| --- | --- | ---: | --- |
| count | own instructions at `-O0` and `-O3`, the four kernels | 2% | under 0.01%: 81 seed/self pairs relinked in shuffled object orders, 8 environment sizes, 3 workload paths; kernels identical with relinked seeds |
| proxy | cycle proxy at `-O0` and `-O3` | 1.5% | 1.22% at `-O0` and 0.51% at `-O3` over the 81 relinked pairs (standard deviation 0.27% and 0.13%); environment and path under 0.03% |
| text | `.text` ratio | 1% | 0.05% over the relinked pairs (alignment padding) |
| memory | `-O3` peak RSS ratio | 25% | 13% over nine runs of the course build |
| time | `-O0` and `-O3` time ratios, scaling growth | 25% | 15% (`-O0`) and 2.5% (`-O3`) over nine runs of the course build; scaling 8% there, and 24% (1.86 to 2.30) across the course, student and reference seeds |

The margins cover noise; the limits in `gates.txt` stay what they are. A
flat 2% for every kind would have failed `text-size` (1.75% at entry) and
left the `-O0` proxy (2.16% at entry) within the layout spread of failing;
the deterministic text ratio needs less. The student build's codegen
margins are the thinner ones: its seed is built with strict aliasing, so
the seed does less work for the same self.

Re-calibrate:

- at each release, on the release machine;
- after a `dev/src` sync that changes code generation, the optimizer or the
  front end's hot paths;
- after the host compiler, libstdc++, glibc or valgrind changes (the C
  library's allocator is part of the seed: linking another one once moved
  the course build's proxies from 1.0376 / 1.0292 to 1.0427 / 1.0424);
- after changing the workload (and its `MANIFEST.sha256`), the kernels or the
  generators.

When either build falls short, improve the solution and re-sync it;
never move a limit to fit it. The export ships `calibration.tsv` and drops
`pa35/maintainer/`.

