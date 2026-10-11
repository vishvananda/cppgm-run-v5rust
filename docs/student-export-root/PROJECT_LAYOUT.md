# Project Layout

`cppgm` is organized as one cumulative compiler implementation plus PA-specific
assignment harnesses.

## Repository Layout

- `Makefile`: root build, test, report, reference, and inception targets
- `dev/`: compiler entry points
- `dev/src/`: shared compiler implementation files and support headers
- `dev/frontend_source_sets.mk`: per-tool lists of `dev/src/*.cpp` files to
  link into each compiler binary
- `paN/`: assignment handouts, Makefiles, tests, scripts, and reference
  fixtures for the active milestones
- `student.tests/`: personal tests and harnesses, run explicitly
- `doc/`: public reference material, including `doc/n3485.txt`
- `obj/`: generated build artifacts
- `reference-binaries/`: reference-binary manifest; the large binary payloads
  download automatically when `*-ref` wrappers need them

Most `paN/` directories are thin wrappers around the shared implementation.
They define the milestone contract and test surface; production compiler code
should stay in `dev/` and `dev/src/`.

When you add a new implementation source file under `dev/src/`, also add its
basename to each tool that needs it in `dev/frontend_source_sets.mk`. Use the
path without `.cpp`; for example, `dev/src/syntax/foo.cpp` is listed as
`syntax/foo`.

## Assignment Arc

- PA1-PA4: tokens, literals, preprocessing expressions, and full preprocessing
- PA5-PA7: AST, types, lookup, conversions, calls, and overload resolution
- PA8: LowIR model, reader/writer, and required construction exercises
- PA9: typed Itanium ABI naming
- PA10-PA13: procedural lowering, classes, value semantics, and virtual dispatch
- PA14-PA19: templates, constant evaluation, and template integration
- PA20-PA23: remaining source-to-LowIR language and object-model closure
- PA24: native backend from LowIR
- PA25-PA26: compile/link driver integration and host exception metadata
- PA27-PA31: host ABI and hosted compatibility
- PA32-PA33: LowIR and machine-backend optimization
- PA34: inception, rebuilding `cppgm++` with `cppgm++`
- PA35: optimizing the compiler, its generated code and its own speed

## Assignment Map

| PA | Tool or target | Main focus |
| --- | --- | --- |
| PA1 | `pptoken` | preprocessing tokens and translation phases 1-3 |
| PA2 | `posttoken` | post-token conversion and literals |
| PA3 | `ppexpr` | controlling-expression evaluation |
| PA4 | `preproc` | full preprocessing, including macros and directives |
| PA5 | `cppgm++ --emit-ast` | AST construction |
| PA6 | `cppgm++ --emit-types` | types, scopes, and lookup |
| PA7 | `cppgm++ --emit-semantics` | conversions, initialization, and overload resolution |
| PA8 | `lowir` | LowIR model, reader/writer, and required construction exercises |
| PA9 | `abimangle` | typed standalone ABI name construction |
| PA10 | `cppgm++ --emit-lowir` | procedural C++ lowering to LowIR |
| PA11 | `cppgm++ --emit-lowir` | basic classes and object layout |
| PA12 | `cppgm++ --emit-lowir` | value semantics and assignment |
| PA13 | `cppgm++ --emit-lowir` | virtual dispatch |
| PA14 | `cppgm++ --emit-lowir` | basic templates |
| PA15 | `cppgm++ --emit-lowir` | specialization and compile-time evaluation |
| PA16 | `cppgm++ --emit-lowir` | constant evaluation |
| PA17 | `cppgm++ --emit-lowir` | template entities and specialization model |
| PA18 | `cppgm++ --emit-lowir` | deduction, substitution, and SFINAE completion |
| PA19 | `cppgm++ --emit-lowir` | template integration across PA14-PA18 features |
| PA20 | `cppgm++ --emit-lowir` | core language closure |
| PA21 | `cppgm++ --emit-lowir` | advanced language closure |
| PA22 | `cppgm++ --emit-lowir` | non-virtual multi-base object model |
| PA23 | `cppgm++ --emit-lowir` | virtual/RTTI object-model completion |
| PA24 | `lowir2native` | native backend from LowIR |
| PA25 | `cppgm++` | separate compilation and compile/link driver integration |
| PA26 | `cppgm++ -c` | host exception metadata and runtime-helper facts |
| PA27 | `cppgm++ -c` | host-linkable object interoperability |
| PA28 | `cppgm++ -c` | host C++ ABI and runtime behavior |
| PA29 | `cppgm++ -E`, `cppgm++ -c` | hosted header/source compatibility |
| PA30 | `cppgm++ -c` | heavy hosted-header compile compatibility |
| PA31 | `cppgm++ -c` | hosted header-emitted link/runtime compatibility |
| PA32 | `lowiropt` | LowIR optimization |
| PA33 | `lowir2native -O1/-O2/-O3` | machine/backend optimization |
| PA34 | inception targets | rebuild `cppgm++` with `cppgm++` |
| PA35 | `make -C pa35 test` | generated code against the seed and GCC, compile speed against the host |

## PA34

PA34 rebuilds your compiler with itself. The host-built `dev/cppgm++` produces
`cppgm++-self`, which produces `cppgm++-inception`. Completion requires the
last two binaries to match byte for byte. The `test-through-paN` ladder runs
earlier assignment tests with self-built tools while you work toward that
comparison; see [PA34](pa34/README.md) for the commands.

## PA35

PA35 makes the compiler you finished in PA34 fast. Its gates compare the
self-built compiler with the host-built seed on a frozen workload, a few
generated-code kernels with GCC, and the seed's compile time and memory with
the host compiler's. `make -C pa35 test` runs them; see [PA35](pa35/README.md).
