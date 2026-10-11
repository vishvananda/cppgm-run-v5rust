## CPPGM Programming Assignment 31 (`cppgm++ -c`)

### Overview

PA31 is the hosted header-emission and link/runtime compatibility assignment.

By PA29 and PA30, hosted source and heavy hosted headers should preprocess and
compile. PA31 asks whether the emitted inline, template, and header-generated
definitions also link and run correctly through the host toolchain.

This milestone is narrower than a second general host ABI assignment. It is
specifically about hosted header-emitted code on top of the ordinary host object
and ABI/runtime path established by PA27 and PA28.

### Prerequisites

Complete PA30 before starting this assignment.

You will want to reuse:

- the full earlier language, template, and lowering stack
- the PA29 hosted preprocess/compile compatibility surface
- the PA30 heavy hosted-header compile surface
- the PA27/PA28 host object and ABI/runtime path

The tests assume a Linux shell environment with `make`, `bash`, `perl`, and a
working host C/C++ toolchain with hosted C++ headers and libraries installed.
You may override the compiler with `CXX=...`.
`CPPGM_HOST_CXX` selects the host compiler/link driver used by the harness. If
it is not set, it defaults to `CXX`.

PA31 tests also use host object tools:

- `nm` for symbol inspection
- `c++filt` for demangling in optional object-inspection checks
- `readelf` for selected object/relocation checks
- `ar` and the host C/C++ compilers for helper libraries when a test provides
  `x.lib.*` sidecars

The checked-in tests assume the normal x86_64 Linux host C++ ABI. When you use
a non-default standard library, pass the same choice through
`CPPGM_STDLIB_FLAGS` so the course compiler and host compiler agree.

### Starter Kit

The starter kit provides:

- your cumulative `dev/cppgm++.cpp` compiler entry point
- the shared implementation you built under `dev/src/`
- `pa31/cppgm++.cpp`, a link to `../dev/cppgm++.cpp`
- `pa31/Makefile`
- `pa31/scripts/`, the hosted link/runtime test harness
- `pa31/tests/link/`, the PA31 tests and checked-in reference files

Put your code changes in `dev/`, especially `dev/cppgm++.cpp` and the
shared implementation files it calls. Do not edit generated `.my` files. Preserve supplied test inputs.
Corrections to reference outputs follow the
[reference policy](../TESTING_AND_REFERENCES.md).

Use `cppgm++-ref` to inspect example output.
Tests run your implementation against the checked-in contract fixtures.

### Command-Line Contract

PA31 does not introduce new `cppgm++` flags. It reuses the compile-mode surface
already required by PA29:

```sh
cppgm++ -c -o <objfile> <srcfile>
cppgm++ -c --target <target> -o <objfile> <srcfile>
cppgm++ -c -I <dir> -o <objfile> <srcfile>
cppgm++ -c -I<dir> -o <objfile> <srcfile>
cppgm++ -c -isystem <dir> -o <objfile> <srcfile>
cppgm++ -c -D <macro> -U <macro> -include <file> -o <objfile> <srcfile>
```

The normal PA31 final link is performed outside `cppgm++` by the host C++
compiler driver.

### Output Format

`cppgm++ -c` shall continue to write host-linker-compatible relocatable object
files.

The PA31 requirement is not a new file format. It is correct symbol ownership,
ABI spelling, and runtime behavior for hosted header-generated code once those
objects are host-linked.

Hosted objects should still be generated from the same LowIR facts exposed by
`cppgm++ --emit-lowir`. If hosted header emission needs symbol ownership,
object symbol spellings, TLS wrapper facts, or runtime hooks, those facts are
LowIR facts (metadata, declarations, definitions, object aliases), so that the
object serialized LowIR produces equals the direct one.

The PA31 tests observe:

- `cppgm++ -c` exit status
- host final-link exit status
- final program exit status
- final program standard output
- optional object-inspection output for needed symbol ownership, unresolved
  references, relocation, and ABI checks

### Error Handling

If preprocessing, parsing, semantic analysis, lowering, object emission, host
linking, or output writing fails, the relevant tool invocation shall report
failure. For `cppgm++`, that means exiting with failure.

For negative tests, exact diagnostics are not the grading contract. The harness
compares exit status first. If the reference compile/link path fails, stdout and
stderr are diagnostic side effects rather than required output.

### Hosted Symbol Emission Surface

Hosted headers expose many inline functions, function templates, constants,
helpers, and implementation-detail declarations. PA31 does not require
`cppgm++ -c` to emit every hosted entity that was parsed, referenced during
semantic analysis, or made visible by an include.

The emitted object should be demand-driven:

- emit the definitions needed by the current translation unit's generated code
  and by the required-definition closure of those definitions
- keep ordinary declarations, overload candidates, template patterns, and
  unused inline/header helpers available for semantic analysis without turning
  them into defined object symbols
- allow unresolved references for externally owned hosted library symbols, using
  the host ABI spelling expected by the configured toolchain

This keeps hosted object files small and avoids exporting implementation-detail
symbols just because a broad standard-library header was included. For example,
including `<functional>`, using `std::forward`, using placement `new`, or
instantiating an `unordered_set` should not by itself cause unrelated libc++ or
libstdc++ helper definitions to appear as defined symbols in the output object.

The course solution decides what to emit by the closure below; a different
closure whose objects link and run is acceptable. The closure starts from the
definitions the translation unit owns (its non-inline functions and variables with external linkage, and
the vtable of each class whose key function it defines, as the Itanium ABI
decides) and adds, until nothing new appears, every vague-linkage definition
that something already in it refers to:

- an inline function, function template specialization or member of a class
  template specialization that emitted code calls or takes the address of
- the vtable of a class with no key function, where emitted code refers to
  it, with the virtual functions, type information and thunks it names
- a static local variable of an emitted inline function or specialization,
  and its guard variable

At `-O0` the closure follows the code as lowered. At `-O1` and above, from
PA32 on, it is computed after inlining and pruning: an inline function whose
every call has been inlined is no longer referenced and is not emitted, so an
optimized object defines far fewer symbols than the `-O0` one.

### Hosted ABI Names

PA31 uses the ordinary host C++ ABI spelling for every hosted symbol that is
defined or referenced by an emitted object. The implementation should continue
to derive those spellings from semantic facts and the PA9 ABI naming layer
rather than from hard-coded library-private names.

The hosted emission policy decides which entities are defined or left
unresolved. Those decisions still need to preserve ordinary host ABI spelling
for every emitted or referenced hosted symbol.

For Itanium-style mangling on GNU/libstdc++ and Clang/libc++ style hosts:

- direct standard-library substitutions such as `St`, `So`, `Si`, `Sd`, `Ss`,
  and `Sa` must use the host ABI spellings
- numbered substitutions must continue across the whole mangled entity, not
  restart between the function-name template-argument list and the bare function
  type
- direct standard substitutions do not themselves become numbered substitution
  entries
- hosted weak/header-emitted definitions must still use the same symbol names
  that the host library expects for the corresponding inline/template bodies

When hosted library implementation details affect ABI names, preserve the
source semantic facts that imply the name: inline namespaces, ABI tags,
template arguments, local contexts, and owner scopes. Avoid constructing
already-mangled strings in the hosted emission path.

### Testing

Run the PA31 suite with:

```sh
make test
```

To run one test through the shared check target:

```sh
make check TEST=tests/link/600-hosted-std-function-call-link-smoke.t
```

The local tests live in `tests/link/`. The directory name reflects the oracle:
hosted compile plus host final link/run, with optional object inspection for
symbol ownership and unresolved-symbol checks.

For each test anchor `x.t`, companion C++ sources are named:

```text
x.t.1
x.t.2
...
```

Optional sidecars include:

- `x.link.flags`: extra flags passed to the host link driver
- `x.env`: environment variables for one test
- `x.lib.*`: host-built C or C++ helper sources
- `x.inspect.cmd` or `x.inspect.expect`: object/symbol checks

Some PA31 tests inspect intermediate object files with `nm`-style expectations.
These checks verify that needed inline and template definitions are present
and use the correct ABI names.

The checked-in tests are hosted link/runtime smokes, ABI spelling checks, and
object-inspection checks rather than direct N3485 clause tests.

### Required Implementation Surface

To complete PA31, implement hosted link/runtime behavior for:

- emitted inline/template/header definitions from hosted headers needed by the
  current object
- hosted standard-library code that compiles in PA29 or PA30 but still has to
  link and run through the plain host toolchain
- hosted link smokes where the main question is emitted symbol ownership, ABI
  spelling, or runtime behavior of hosted header-generated code

If hosted header code compiles but the emitted objects do not link or run
correctly, fix the hosted symbol emission, ABI spelling, object ownership, or
runtime lowering path.

### Out Of Scope

The PA31 tests do not require:

- new hosted preprocess/compile compatibility beyond the PA29/PA30 surface
- general host object or host ABI behavior outside the hosted-header-triggered
  surface already required before PA31
- build-system wrapper emulation
- recursive hosted-header coverage reporting
- bootstrap or self-host builds

### Design Notes (Non-Normative)

Treat header-emitted code as ordinary code with an ABI-sensitive ownership
policy. The implementation should preserve enough semantic information to know
which inline/template definitions are required, which declarations remain
external, and which unused hosted helpers should stay un-emitted.

A recommended integration style is to use the PA9 ABI naming layer for hosted
symbols in the same way PA27 and PA28 use it for ordinary host objects. Semantic
analysis can produce the facts for the entity being emitted or referenced, then
the mangler can produce the final raw symbol name before object emission. When a
hosted symbol case is missing information, prefer threading that semantic fact
forward instead of building already-mangled or partly-mangled strings in later
object/link stages.

### After PA31

After PA31, the compiler can preprocess, compile, link, and run hosted-header
programs through the host toolchain. Later tests use that foundation while
adding optimization and self-host workloads.
