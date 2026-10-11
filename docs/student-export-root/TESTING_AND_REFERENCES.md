# Testing and references

Run these commands from the repository root, replacing N with the assignment
number:

```sh
make build
make test-paN
make test-report-through-paN
make test-report
make inception
```

For PA1–PA33, a clean through-milestone report is the exit criterion. PA34
ends with inception. PA35 ends with `make -C pa35 test`, which is not part of
the report: its gates compare your compiler with the host and its own seed.
`make test` builds once and runs all assignment suites; `make test-report`
keeps going and summarizes failures. Use `ACTIVE_TEST_REPORT_PAS='pa8 pa10'`
to narrow a report.

## Local checks and toolchains

```sh
make -C pa8 check TEST='tests/spec/100-*.t'
make -C pa8 check TEST='tests/behavior/100-sum.t tests/behavior/200-swap.t'
make CXX=g++ CPPGM_HOST_CXX=g++
```

`TEST=` accepts a path, quoted glob or quoted list; mixed-input assignments
route each fixture to its runner. Finish with the root through target.
Keep a real host compiler in `CPPGM_HOST_CXX` when PA34 uses
`CXX=../dev/cppgm++`. Use a separate object root for different toolchains.

## Test locations and requirements

- Required fixtures live in `paN/tests/`, in the buckets named by the handout.
  `controls/` holds focused property checks with no complete-output oracle.
- Personal inputs and harnesses live in `student.tests/`; run them explicitly.
  Default assignment targets do not discover that directory.
- PA24 and PA32 also ship `tests/regression/`, which pins the course
  solution's particular design. Those fixtures are outside your exit criteria;
  default `make test` and the through reports run the course contract.

PA33 checks executable behavior, declared MIR bounds and debug locations.
It requires no profiler, allocator statistics or exact optimized MIR match.

From PA5 onward, filenames use a three-digit feature-cluster prefix
(`100-`, `200-`, ...); PA1–PA4 use finer subgroups. Controls and solution
regressions follow their own local naming. Required rejection cases are in the
owning suite; unspecified inputs do not add requirements.

A graded input is an `x.t` file in the tool's input language (companions
`x.t2`, `x.t.1`, ... as the handout describes); its reference outputs are
`x.ref` and `x.ref.*`; a control is a `.cpp` program. A leading
`// VALIDATION: ...` or `// N3485 focus: ...` comment in a fixture is
informational. `CPPGM_BATCH_TESTS=1` runs an assignment's cases through a
worker linked into each tool (`dev/src/support/testing/test_runner.cpp`),
which requires each tool's `main` to live in `dev/<tool>.cpp`; the ordinary
path runs the tool once per case.

## References

Use the oracle named by the handout: output, exit status, a focused property,
or a code-quality envelope. Diagnostic text is not compared. Informational
LowIR/MIR sidecars do not become grading requirements merely because they ship.

PA8 requires a LowIR model, reader/validator, writer and three construction
exercises. The harness uses `lowir-ref` for roundtrips and `lowir2native-ref`
to execute the student's constructed LowIR. Behavior checks accept alternative
valid LowIR and grade program outcomes. The student compiler must implement
LowIR construction and later C++ lowering itself.

From PA10 through PA23, a control whose leading comment says `expect exit N`
or `expect reject` (or with an `x.expect` sidecar) is a self-checking program
that executes your source-generated LowIR through the supplied native backend.
`make test` compiles it with your `cppgm++ --emit-lowir -O0`, builds the result
with `lowir2native-ref -O0` and requires that exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject it. A program
killed by a signal never has the stated status, and a frontend that crashes
does not reject. Any LowIR with the right behavior passes. Run one
assignment's controls alone with `make -C paN test-controls`. Your own native
backend is introduced in PA24.

Reference wrappers download and verify a pinned bundle built from the current
cppgm-extended solution. They provide examples of the required behavior and
are not the original CPPGM binaries. Fetch them ahead of time with:

```sh
make reference-binaries
```

Reference wrappers fail if a tool cannot be downloaded or verified; they
never fall back to your implementation.

Failed-case stdout is an informational example. Successful stdout and
required exit-status sidecars remain oracles.
[The LowIR specification](pa8/lowir.md#what-the-comparison-absorbs-and-what-it-enforces)
lists what normal LowIR comparison absorbs and what it enforces, including
the facts your lowering states (`unwind`, `return`, `effects`,
`storage=readonly`, `object_bytes` and the others), which PA10–PA23 compare
exactly.

Reference implementations and checked outputs are not guaranteed bug-free.
Preserve them by default. You may correct reference outputs when a reduced
reproducer and cited C++11 rules—or the LowIR contract for IR-only cases—prove
them incorrect. Document the proof and bundle revision. Compiler agreement
alone is insufficient. Never weaken required behavior, coverage, or comparison
rules.

## Debug and inspection checks

Run the debug, object and link checks required by the current handout.
`make test-debuginfo` collects the later debug suites; it is not an extra
prerequisite for early assignments.
