## CPPGM Programming Assignment 30 (`cppgm++ -c`)

### Overview

PA30 is the hosted header-compile assignment. It is split out of PA29: where
PA29 establishes that hosted source/header inputs preprocess and compile at all
(intrinsics, parser concessions, builtin traits, header-free conformance
anchors, and lightweight C-wrapper header smokes). PA30 raises the bar to
**compiling the heaviest real C++ standard-library headers** end to end with
`cppgm++ -c`, within a workable compile time.

By PA29, hosted headers and source should preprocess and compile for the
lighter hosted workload. PA30 raises the bar to large, template- and trait-heavy
STL headers (`<vector>`, `<unordered_map>`, `<tuple>`, `<random>`,
`<functional>`, and the iostream/string/exception machinery).

The tests require clean object emission from heavy hosted headers. PA31 then
checks whether emitted header code links and runs.

To complete PA30, implement these goals:

- compile the heaviest hosted standard-library headers cleanly with `cppgm++ -c`
- carry the template instantiation, trait evaluation, and overload-resolution
  depth those headers exercise without exponential blow-up
- stay within a workable compile time on the heavy-header workload (the
  harness stops a compile after 45 seconds by default; it does not limit
  memory)

### Prerequisites

Complete PA29 before starting this assignment.

You will want to reuse:

- the full earlier language, template, semantic, and lowering stack
- the PA29 hosted preprocess/compile compatibility surface (intrinsics, parser
  concessions, builtin traits/types, header-free conformance anchors, and
  lightweight C-wrapper header smokes)
- the PA27/PA28 `cppgm++ -c` host-object path

The tests assume a Linux shell environment with `make`, `bash`, `perl`, and a
working host C++ compiler with hosted C++ headers installed. You may override the
compiler with `CXX=...`. `CPPGM_HOST_CXX` selects the host compiler used for
builtin macro/include probing; if unset it defaults to `CXX`. When you use a
non-default standard library, pass the same choice through `CPPGM_STDLIB_FLAGS`
so the course compiler and host compiler agree.

### Starter Kit

The starter kit provides:

- your cumulative `dev/cppgm++.cpp` compiler entry point
- the shared implementation you built under `dev/src/`
- `pa30/cppgm++.cpp`, a link to `../dev/cppgm++.cpp`
- `pa30/Makefile`
- `pa30/scripts/`, the hosted `-c` compile test harness (shared with PA29)
- `pa30/tests/compile/`, the heavy-STL header-compile tests and checked-in
  reference files

Put your code changes in `dev/`, especially `dev/cppgm++.cpp` and the
shared implementation files it calls. Do not edit generated `.my` files. Preserve supplied test inputs.
Corrections to reference outputs follow the
[reference policy](../TESTING_AND_REFERENCES.md).

Use `cppgm++-ref` to inspect example output.
Tests run your implementation against the checked-in contract fixtures.

### Command-Line Contract

PA30 introduces no new `cppgm++` flags. It reuses the compile-mode surface
already required by PA29:

```sh
cppgm++ -c -o <objfile> <srcfile>
cppgm++ -c -isystem <dir> -o <objfile> <srcfile>
cppgm++ -c -D <macro> -U <macro> -include <file> -o <objfile> <srcfile>
```

### Output Format

`cppgm++ -c` shall write a host-linker-compatible relocatable object as in
PA27/PA28/PA29. For PA30 the emitted object is discarded — the requirement is
simply that the heavy hosted header compiles cleanly to an object without error.

The hosted header path should still lower through the same LowIR representation
used by `cppgm++ --emit-lowir`. Performance work may avoid unnecessary file
I/O; the object must remain the one serialized LowIR produces (PA32's object
round trip), so the facts the backend needs for hosted code are LowIR facts.

### Error Handling

If preprocessing, parsing, semantic analysis, lowering, object emission, or
output writing fails, `cppgm++` shall exit with failure. Exact diagnostics are
not the grading contract; the harness compares exit status and checked output
sidecars.

### Testing

Run the PA30 suite with:

```sh
make test
```

To run one test through the shared check target:

```sh
make check TEST=tests/compile/600-const-unordered-map-find.t
```

The local tests live in `tests/compile/`. Each test is `.t` (source) plus an
empty `.ref` base, a `.ref.exit_status`, and a `.ref.stdout`. Most expect
`EXIT_SUCCESS` and pass iff `cppgm++ -c` compiles them cleanly; the few
negative tests expect `EXIT_FAILURE`. Each test includes a real heavy
header together with a semantic check — a trait, `decltype`, `sizeof`, or
`static_assert` that cannot be satisfied without analyzing the header,
so a test cannot pass by skipping or stubbing the include.

Optional `x.no-exceptions` and `x.cxx-standard` sidecars disable exceptions or
select the language mode for a compile test. The standard sidecar currently
accepts `c++11` and `c++14`.

### Required Implementation Surface

To complete PA30, support:

- compiling the heaviest hosted standard-library headers within that time limit
- the template, trait, and overload-resolution depth those headers exercise
  during `-c` compilation

The PA30 tests do not require:

- new intrinsics, parser concessions, header-free conformance anchors, or
  lightweight hosted C-wrapper behavior beyond the hosted compatibility already
  needed before PA30
- hosted header-emitted link/runtime behavior
- bootstrap or self-host builds

If a failing PA30 test exposes a bug in syntax, semantic analysis, template
handling, lowering, or object emission that is shared with earlier language
features, fix the shared compiler behavior rather than adding a PA30-only path.

### Design Notes (Non-Normative)

The heavy headers stress the same machinery PA29 enables, just far harder:
deeply nested template instantiation, trait and `decltype` evaluation, partial
specialization selection, and large overload sets. The productive failures here
are usually performance cliffs (re-resolving the same bound template pack,
re-instantiating the same specialization) rather than missing features. Prefer
memoizing repeated resolution over deepening recursion guards.

PA30 is the first assignment whose inputs make the harness's time limit a
budget rather than a guard against hangs: each test brings a heavy header,
with its macros, overload sets and thousands of specializations, into one
translation unit. The limit sits far above what a sound design needs, so it
catches cliffs, not constant factors: work that grows with the square of the
input, or that is redone at every use instead of once. The speed comes from
structure the earlier assignments set up:

- PA4: a header inside an include guard is skipped on later inclusions, and
  an inactive group is scanned only for its directives
- PA6: a lookup costs the scopes it searches, not the size of the program
- PA14 and PA15: a specialization is found in one probe, by its template and
  canonical arguments, with constant arguments as values
- PA14's on-demand instantiation and the deferred instantiation of PA18 and
  PA19: a body is instantiated only when it is used
- PA16: the constant-initialization probe runs on every namespace-scope
  object of every hosted header, so it has to give up cheaply
- PA17: an explicit-instantiation declaration (`extern template`) leaves the
  library's own instantiations to the library

When a PA30 compile is slow, find which of these it breaks before tuning
anything else.

### After PA30

Later hosted tests keep the same hosted header environment and then check that
the emitted code from those headers also links and runs.
