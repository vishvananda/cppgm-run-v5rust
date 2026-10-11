# Agent instructions

Build one cumulative C++11 compiler for Linux x86-64, working through PA1–PA35.
Read the current `paN/README.md` and [Testing and references](TESTING_AND_REFERENCES.md)
before changing code. [Project layout](PROJECT_LAYOUT.md) gives the assignment map.

## Implementation

- Work in `dev/` and `dev/src/`; extend earlier work at each milestone.
- Register new implementation sources, without `.cpp`, in the appropriate
  tool lists in `dev/frontend_source_sets.mk`.
- Treat `paN/` as handouts, fixtures, references, harnesses and wrappers.
- Use real language and compiler logic; do not hardcode fixture answers.
- Implement required output yourself. Do not delegate it to reference binaries,
  another solution or a host compiler unless the handout requires that host
  interaction. PA8's supplied native backend is called by the grading harness.
- Keep personal tests in `student.tests/` and run them explicitly.
- Do not change tests or references to hide missing or incorrect behavior.
  Do not commit generated objects, logs or `.my*` outputs.

## Validation

```sh
make test-paN
make test-report-through-paN
```

Replace N with the assignment number. For PA1–PA33, pass the root through
report before advancing. `perl scripts/cppgm_file_audit.pl --paths dev`
checks the implementation's shape: file and function size limits, counted in
lines of code, and shortcuts such as reading the environment or running other
programs from the compiler. It is advisory: no exit criterion runs it, the
shipped files pass it, and a tree that fails it usually has a shortcut or a
file too large to review. PA34 ends with `make inception`, PA35 with
`make -C pa35 test`. Use the debug and inspection targets required by the
owning handout when changing those surfaces.

Default tests run the course contract. PA24 and PA32's `tests/regression/`
describe the course solution's design and are outside your exit criteria.
PA33 requires behavior, MIR bounds and debug checks, with no profiler or
allocator-specific diagnostics.

## References

Checked-in contract fixtures and the handout define the assignment. Reference
tools are provided for observation; behavior outside
the fixtures can contain bugs. Prefer the handout and C++11 standard to copying
an erroneous reference result. Diagnostic text is not graded.

PA8's harness uses `lowir-ref` to compare roundtrips and `lowir2native-ref` to
execute student-constructed LowIR. Implement the LowIR machinery and later
C++ lowering yourself.

The self-checking controls of PA10–PA23 also execute your source-generated
LowIR through the supplied native backend. Your own native backend is
introduced in PA24.

Preserve inherited attribution. [NOTICE](NOTICE) explains how this extensively
revised course derives from the original CPPGM material.
