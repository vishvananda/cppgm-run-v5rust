## CPPGM Programming Assignment 25 (`cppgm++ -c` and link mode)

### Overview

PA25 does not introduce a new executable. It extends the same `cppgm++` binary
used since PA5 with practical compiler-driver behavior.

`cppgm++` has two required PA25 modes:

- compile mode, `-c`, which takes one C++ source file and writes one
  implementation-defined compiler object file
- default link mode, which takes one or more inputs and writes one native
  executable program

In link mode, each input may be either:

- a C++ source file, which `cppgm++` compiles as its own translation unit before
  linking
- a compiler object file previously produced by `cppgm++ -c`

The contract is source-driven. The PA25 tests start from C++ source
files, validate explicit separate compilation with `cppgm++ -c`, and then link
the resulting objects with `cppgm++`. The harness also checks two practical
driver consistency properties:

- linking the same source files directly through `cppgm++` must match explicit
  compile-then-link behavior
- linking a mixture of precompiled objects and remaining source files must also
  match explicit compile-then-link behavior

PA25 does not introduce a new language subset. It turns the C++ feature set
implemented through PA23 into a practical compile-and-link toolchain entrypoint.

### Prerequisites

Complete PA24 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST and PA6/PA7 semantic foundation
- the PA10-PA23 LowIR lowering path
- the PA24 native backend

The tests assume a POSIX-like shell environment with `make`, `bash`,
`perl`, and a working host C/C++ compiler for test helper objects. The harness
selects helper compilers from:

- `CPPGM_HOST_CC` or `CC` for C helper sources
- `CPPGM_HOST_CXX` or `CXX` for C++ helper sources

If those are not set, the harness searches for common compilers such as
`clang`, `gcc`, `cc`, `clang++`, `g++`, and `c++`. Some tests substitute the
Linux target name or the corresponding x86_64 Linux triple,
`x86_64-unknown-linux-gnu`, into driver flags.

### Starter Kit

The starter kit provides:

- your cumulative `dev/cppgm++.cpp` compiler entry point
- the shared implementation you built under `dev/src/`
- `pa25/cppgm++.cpp`, a link to `../dev/cppgm++.cpp`
- `pa25/Makefile`
- `pa25/scripts/`, the compiler-driver test harness
- `pa25/tests/general/`, the PA25 tests and checked-in reference files
- the shared `cppgm++` source grammar, exposed for this assignment as
  `pa25.gram`
- an HTML grammar explorer of `pa25.gram` in the sub-directory `grammar/`

Student code changes should go in `dev/`, especially `dev/cppgm++.cpp` and the
shared implementation files it calls. Do not edit generated `.my` files. Preserve supplied test inputs.
Corrections to reference outputs follow the
[reference policy](../TESTING_AND_REFERENCES.md).

Use `cppgm++-ref` to inspect example output.
Tests run your implementation against the checked-in contract fixtures.

### Driver Surface

Previously required:

- `--emit-ast`
- `--emit-types`
- `--emit-semantics`
- `--emit-lowir`
- `-o <outfile>`

New in PA25:

- compile mode: `-c`
- default link mode with source and object inputs
- include search: `-I <dir>` and `-I<dir>`
- library search: `-L <dir>`, `-L<dir>`, `-l <name>`, and `-l<name>`
- target selection: `--target <target>` or `--target=<target>`
- flags the fixtures pass, accepted and applied as the host compiler applies
  them: `-D<name>` and `-D<name>=<value>`, `-pedantic` and `-pedantic-errors`,
  `-fvisibility=hidden` and `-fvisibility-inlines-hidden`, `-O<n>` (PA32
  gives the levels their meaning; until then they are accepted) and `-g0`

Not yet required here:

- hosted preprocess mode `-E`
- hosted preprocessor-control flags such as `-U`, `-include`, and `-isystem`
- driver query flags such as `--version`, `-v`, `-dumpmachine`,
  `-dumpversion`, and `-print-search-dirs`
- static archives and shared libraries as link inputs

### Command-Line Contract

Required compile forms:

```sh
cppgm++ -c -o <objfile> <srcfile>
cppgm++ -c --target <target> -o <objfile> <srcfile>
cppgm++ -c -I <dir> -o <objfile> <srcfile>
cppgm++ -c -I<dir> -o <objfile> <srcfile>
cppgm++ -c --target <target> -I <dir> -o <objfile> <srcfile>
cppgm++ -c --target <target> -I<dir> -o <objfile> <srcfile>
```

Required link forms:

```sh
cppgm++ -o <outfile> <input1> <input2> ... <inputN>
cppgm++ --target <target> -o <outfile> <input1> <input2> ... <inputN>
cppgm++ -I <dir> -o <outfile> <input1> <input2> ... <inputN>
cppgm++ -I<dir> -o <outfile> <input1> <input2> ... <inputN>
cppgm++ -L <dir> -l<name> -o <outfile> <input1> <input2> ... <inputN>
cppgm++ -L<dir> -l <name> -o <outfile> <input1> <input2> ... <inputN>
```

Options may be combined when their meanings are compatible, for example
`--target <target>` with `-I` or `-L`/`-l`.

In link mode, each `<inputK>` may be:

- a C++ source file
- an object-like file produced by `cppgm++ -c`

For PA25, object files are identified by implementation-supported object-like
filenames such as `.o` or `.obj`. The checked-in tests use `.obj`.

`-I` adds user include search paths for any C++ source files compiled in that
invocation. These paths apply to quoted and angle-bracket includes and are
searched before compiler-provided shim include paths.

`-L` and `-l` search implementation-supported object-like libraries. The tests use simple helper objects named like `lib<name>.o` in a harness-created
library directory.

All linked inputs in one invocation must target the same native backend target.

### Output Format

In compile mode, `cppgm++` shall write one compiler object file to `<objfile>`.

In link mode, `cppgm++` shall write one native executable program to
`<outfile>`.

The PA25 object-file encoding is intentionally an internal `cppgm++` contract:
the file must be consumable by `cppgm++` link mode, but it does not need to be
accepted by the host linker. The exact object-file encoding is not directly
compared by the PA25 tests. The exact final binary encoding is also not
directly compared. Instead, the tests compare:

- compile/link exit status
- generated program exit status
- generated program standard output

### Error Handling

If an error occurs during preprocessing, parsing, semantic analysis, lowering,
object-file emission, linking, or native output writing, `cppgm++` shall exit
with failure.

Important PA25 error cases include:

- duplicate global symbol definitions, including definitions imported from
  separate helper objects or libraries
- unresolved external symbols
- missing `main`

For negative tests, exact diagnostics are not the grading contract. The harness
compares exit status first. If the reference compile/link path fails, stdout and
stderr are diagnostic side effects rather than required output.

### Standard Output And Error

Standard output and standard error are ignored for successful automated testing
of `cppgm++` in PA25. You may use them for diagnostics.

### Testing

Run the PA25 suite with:

```sh
make test
```

To run one test through the shared check target:

```sh
make check TEST=tests/general/100-two-source-call.t
```

The local tests live in `tests/general/`. They exercise practical
compiler-driver, separate-compilation, link, runtime, and consistency behavior.
They are not direct N3485 clause tests.

For each test anchor `x.t`, companion C++ sources are named:

```text
x.t.1
x.t.2
...
```

Optional sidecars include:

- `x.flags`: extra flags passed to `cppgm++`
- `x.lib.*`: host-built helper C or C++ sources that become object-like
  libraries for `-L`/`-l` tests
- `x.stdin`: standard input for the generated program

For each test case, the harness checks:

1. Explicit separate compilation:
   `cppgm++ -c` is executed once for each companion source file, and then
   `cppgm++` links the generated objects.
2. Direct source linking:
   `cppgm++` is executed directly on the same source files.
3. Mixed source/object linking for multi-source tests:
   one generated object and the remaining source files are linked together.

The checked-in `.ref.*` files are compared against the explicit compile/link
path. The direct and mixed paths are consistency checks: they must match the
explicit path.

This validates:

- compile mode
- link mode
- source-to-object lowering through the full language pipeline
- consistency between direct source linking and explicit separate compilation
- consistency between mixed source/object linking and explicit separate
  compilation
- cross-translation-unit data relocations that feed indirect calls
- namespace-scope startup hooks across translation units
- coalescible ODR emission when an out-of-class class-template member
  definition from a shared header is instantiated in multiple translation
  units

### Assignment Boundary

PA25 must support the C++ feature set already implemented through PA23, but
through a practical driver interface rather than one stage-specific binary per
milestone.

Within that supported subset, PA25 should:

- compile one C++ source file to one compiler object file with `-c`
- link compiler object files into a native executable
- accept C++ source files directly in link mode by compiling each source as its
  own translation unit before linking
- support user include search paths through `-I`
- support source-level external declarations needed for ordinary separate
  compilation, such as `extern int g;`
- support ordinary external C function declarations and definitions through
  `extern "C"` in the practical subset needed for object-style library
  interoperability
- support object-like library search through `-L` and `-l`
- support simple complete-program runtime tests written in C++ and linked
  against harness-provided object-style support libraries, without requiring
  host libc or hosted headers
- allow an implementation-defined compiler object format with your own linker
  for PA25, as long as the `cppgm++` behavior matches the contract

To complete PA25, implement these goals:

1. Separate compilation from C++ source.
2. Direct source-link parity.
3. Mixed source/object parity.
4. Cross-translation-unit source semantics.
5. Toolchain-style include handling.
6. External object-library interoperability through the tested `extern "C"`
   and `-L`/`-l` subset.
7. Full-language-through-toolchain validation for previously implemented
   language features.
   The supported wide-integer extension is included in that runtime surface:
   truth conversion of `__int128` values must inspect the complete value, and
   mixed signed/unsigned 128-bit comparisons must follow the usual arithmetic
   conversions independently of operand order. Bitwise complement and left,
   logical-right, and arithmetic-right shifts must also work for runtime counts,
   including counts on either side of the 64-bit half boundary.
8. Source-driven runtime-program validation without host-library dependence.
   The runtime calls `main` with the process's arguments: a `main` declared
   `int main(int argc, char** argv)` receives the argument count and the
   argument strings, with `argv[argc]` null. Exceptions in that runtime are
   caught as the Itanium C++ ABI catches them; in particular a pointer
   exception is caught by value, so a pointer handler receives the thrown
   pointer, after any qualification, public-base or `void*` conversion the
   handler's type requires. How the runtime finds a handler is not part of
   that contract: handler records pushed and popped as regions open and
   close, and unwind tables like the host's, both meet it. The runtime also
   provides `__dso_handle` and `__cxa_thread_atexit`, so the main thread's
   `thread_local` objects are destroyed at exit, before process-exit
   destructors, as the host C library does.

### Out Of Scope

The following are out of scope for PA25:

- full system-compiler flag compatibility beyond the documented PA25 options
- static archives such as `.a`
- shared libraries such as `.so` or `.dylib`
- arbitrary foreign non-object library formats
- full `extern "C"` linkage-specification coverage beyond the practical
  function-oriented subset needed for PA25 interop
- dependence on host libc or hosted headers for the basic PA25 runtime-program
  coverage
- dependency generation flags
- precompiled headers
- build-system conveniences such as depfiles or compilation databases
- hosted preprocessor and hosted-header compatibility, which belong in
  PA29-PA31
- standalone ABI name construction, which belongs in PA9
- host-linker-compatible object output, which belongs in PA26/PA27

### Design Notes (Non-Normative)

PA25 should wrap the existing implemented pipeline, not replace it.

In particular:

- C++ source inputs should still flow through the existing semantic and LowIR
  lowering path.
- Build the object and link stages around the PA24 native backend and
  existing LowIR symbol and data representations.
- The direct source-link path should behave like repeated separate compilation
  followed by linking, not like a special one-off shortcut.
- Do not carry a private PA25 object encoding forward as the host-object
  solution. PA26/PA27 replace the internal compiler-object contract with a
  host-linker-compatible object contract.
- A compiler object needs what linking needs: code, data, symbols and
  relocations, and the course solution's holds those and nothing else. A
  design that keeps a serialized program in the object for link mode to
  compile again is acceptable if the parity checks pass; the host objects of
  PA26 onward carry nothing of the kind.
- A handler-record runtime, like the course solution's, is the simpler way
  to meet goal 8. A record restores the callee-saved registers as they were
  when its region was entered, so a function with regions keeps in memory
  the values its landings may read (PA21's design notes say what that
  costs); PA26's table-driven host model does not have that cost.

### Stage Handoff

PA26 adds host-compatible exception metadata and relocatable objects. PA27
then completes the host-object symbol and ABI surface using the PA9 encoder.
