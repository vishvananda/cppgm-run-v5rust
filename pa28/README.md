## CPPGM Programming Assignment 28 (`cppgm++ -c`)

### Overview

PA28 is the host C++ ABI/runtime interoperability assignment. It builds on the
ordinary host-linkable object requirements from PA27 and makes the behavior of
the host-linked program observable.

The main PA28 question is: once host link succeeds, does the resulting program
behave correctly under the ordinary host C++ ABI/runtime?

The tested ABI/runtime surface includes:

- virtual dispatch, vtable ownership, and imported/exported vtables
- RTTI object ownership and `dynamic_cast` / `typeid`
- covariant return adjustment, including layout-finalized fixed adjustments and
  virtual-base result projection through the returned object's vtable
- richer host exception handling in the exercised
  rethrow/cleanup/noexcept/RTTI subset
- host-compatible unwind and relocation facts where the tests inspect objects

### Prerequisites

Complete PA27 before starting this assignment.

You will want to reuse:

- the full C++ language pipeline through PA27
- the PA27 host-compatible `cppgm++ -c` path
- the PA24 native backend and PA25 object-emission path
- the PA26 exception/runtime lowering concepts

The tests assume a POSIX-like shell environment with `make`, `bash`,
`perl`, and a working host C/C++ toolchain. The harness selects host tools from:

- `CPPGM_HOST_CXX` or `CXX` for the host C++ compiler/link driver
- `CPPGM_HOST_CC` or `CC` for host C helper objects

If those are not set, the harness searches for common compilers such as
`clang++`, `g++`, `c++`, `clang`, `gcc`, and `cc`. Archive and inspection tests
also require `ar`, `nm`, and `readelf`. The checked-in tests assume the normal
x86_64 Linux host C++ ABI.

PA26 introduced the basic host-EH object-facts surface. PA28 keeps the same
host-link path while exercising richer host ABI/runtime interactions.

### Starter Kit

The starter kit provides:

- your cumulative `dev/cppgm++.cpp` compiler entry point
- the shared implementation you built under `dev/src/`
- `pa28/cppgm++.cpp`, a link to `../dev/cppgm++.cpp`
- `pa28/Makefile`
- `pa28/scripts/`, the host-ABI test harness
- `pa28/tests/general/`, the PA28 tests and checked-in reference files

Put your code changes in `dev/`, especially `dev/cppgm++.cpp` and the
shared implementation files it calls. Do not edit generated `.my` files. Preserve supplied test inputs.
Corrections to reference outputs follow the
[reference policy](../TESTING_AND_REFERENCES.md).

Use `cppgm++-ref` to inspect example output.
Tests run your implementation against the checked-in contract fixtures.

### Command-Line Contract

PA28 does not introduce new command-line flags. It reuses the PA27 compile-mode
surface:

```sh
cppgm++ -c -o <objfile> <srcfile>
cppgm++ -c --target <target> -o <objfile> <srcfile>
cppgm++ -c -I <dir> -o <objfile> <srcfile>
cppgm++ -c -I<dir> -o <objfile> <srcfile>
cppgm++ -c --target <target> -I <dir> -o <objfile> <srcfile>
cppgm++ -c --target <target> -I<dir> -o <objfile> <srcfile>
```

`<target>` may be `linux` or the corresponding x86_64 Linux host triple form
accepted by your implementation. PA28 only requires compile mode. The normal
PA28 final link is performed outside `cppgm++` by the host C++ compiler driver.

### Output Format

`cppgm++ -c` shall continue to write one host-linker-compatible relocatable
object file to `<objfile>`.

The PA28 tests do not compare object bytes directly. They observe:

- `cppgm++ -c` exit status
- host final-link exit status
- final program exit status
- final program standard output
- optional object-inspection output for ABI, unwind, relocation, RTTI, vtable,
  thunk, and symbol-ownership checks

### Error Handling

If preprocessing, parsing, semantic analysis, lowering, object emission, or
output writing fails, `cppgm++` shall exit with failure.

For negative tests, exact diagnostics are not the grading contract. The harness
compares exit status first. If the reference compile/link path fails, stdout and
stderr are diagnostic side effects rather than required output.

### Testing

Run the PA28 suite with:

```sh
make test
```

To run one test through the shared check target:

```sh
make check TEST=tests/general/200-host-eh-rethrow.t
```

The local tests live in `tests/general/`. They cover host C++ ABI/runtime
behavior, host-linked exception handling, RTTI, vtables, thunks, and object
inspection around those host-runtime surfaces. They are not direct N3485 clause
tests.

For each test anchor `x.t`, companion C++ sources are named:

```text
x.t.1
x.t.2
...
```

Optional sidecars control or check the host flow:

- `x.link.flags`: extra flags passed to the host link driver
- `x.lib.*`: host-built C or C++ helper sources
- `x.inspect.cmd`, `x.inspect.expect`, or `x.inspect.plan`: object-inspection
  checks that use the host symbol and object tools

The checked-in PA28 tests cover:

- cleanup, rethrow, noexcept termination, and foreign catch-all behavior beyond
  the basic PA26 host-EH facts surface, including exactly-once same-frame local
  cleanup around class-exception allocation and payload construction
- class, base, transitive-base, and virtual-base host exception catches
- host EH interaction with RTTI, `typeid`, lambdas, templates, and control flow
- virtual dispatch, imported/exported vtable ownership, and polymorphic header
  duplication
- RTTI-driven `dynamic_cast` and `typeid`
- covariant return adjustment
- host ABI mangling for dependent/template/lambda/standard-library-adjacent
  names needed by this milestone
- object facts such as unwind sections, relocation classes, weak/undefined
  symbols, and vtable/RTTI ownership when a test includes an inspect sidecar

PA28 does not require general hosted-header support. Your compiler must support
the host ABI behavior for user-defined RTTI and exception cases: emitted RTTI
objects, `dynamic_cast`, `typeid`, vtables, catches, cleanup, and unwind
interoperability. The PA28 tests exercise that behavior without including hosted
headers. For `typeid`, it is enough in this assignment to support the language
operation with a narrow declaration of `std::type_info`; the tests do not depend
on the hosted `<typeinfo>` header or on `std::type_info` member APIs.

Parsing hosted `<exception>` and `<typeinfo>` headers, implementing APIs such
as `std::type_info::name()` or `hash_code()`, compiling hosted exception
classes, and supporting `std::exception_ptr` are later hosted-header/runtime
work.

### Host ABI Symbol Names

PA28 extends the PA27 object requirements into host C++ ABI/runtime behavior. The
same ABI naming behavior from PA9 and PA27 is still observable for every C++
symbol that the host linker, unwinder, RTTI system, or virtual-dispatch
machinery can observe.

The important observable result is that these entities have host ABI names that
match the configured toolchain:

- ordinary functions, methods, constructors, destructors, and function
  templates
- vtables, VTTs, RTTI objects, covariant thunks, non-virtual thunks, and
  virtual-base thunks
- exception, cleanup, and helper symbols that must interoperate with host-built
  code
- local classes, lambdas, dependent/template names, ABI tags, and inline
  namespaces when they affect the host name
- ABI tags carried by class typeinfo names, typeinfo objects, and vtables

### Required Implementation Surface

To complete PA28, preserve practical host-linked C++ ABI/runtime behavior within
the supported subset:

- virtual dispatch and imported/exported vtable ownership
- RTTI-driven `dynamic_cast` / `typeid`; a reference-form `dynamic_cast` that
  fails throws `std::bad_cast` through the host runtime (5.2.7p9), and an
  ordinary handler catches it
- covariant return adjustment
- ordinary host-linked rethrow and advanced catch behavior
- cleanup and unwind interactions beyond the basic PA26 fact owners
- foreign catch-all interaction in the tested subset
- GNU `pure` and `const` attributes on ordinary functions and function
  templates, preserved on the canonical callable as read-only-memory and
  no-memory-access effects, respectively: `effects=readonly` and
  `effects=readnone` on its declarations and definition and on the call
  signature of a virtual call (`../pa8/lowir.md`, "Facts The C++ Lowering
  States"). A later declaration
  adds the attribute, one on a lambda-declarator is the call operator's, and
  a function template's reaches its specializations, while an explicit
  specialization takes only its own declarations' attributes, as GCC reads
  them; `noinline` follows the same rules for `no_inline=yes`, and PA32's
  `-O0` driver lane compares these facts

If host link succeeds but the host C++ ABI/runtime behavior is wrong, fix the
host ABI/runtime lowering, metadata, or object-emission path.

### Out Of Scope

The PA28 tests do not require:

- basic host-EH metadata/object facts beyond the PA26 surface
- private course-only exception/runtime ABI details that are not visible through
  the host-linked program or object checks
- hosted standard-library header/source compatibility
- hosted header-emitted link/runtime behavior
- bootstrap or self-host builds

### Design Notes (Non-Normative)

PA28 is not just a runtime-output assignment. Runtime behavior is the primary
oracle, but some tests inspect object facts because host C++ ABI correctness is
often decided before the program starts: symbol names, weak ownership, unwind
sections, RTTI/vtable objects, and relocation classes must match the host
toolchain's expectations closely enough for ordinary linking and unwinding.

A recommended implementation style is to continue using the PA9 ABI naming
layer before object emission. Feed semantic facts for the entity into the
mangler, then let the object writer preserve the final raw symbol name. That
keeps ABI spelling decisions close to semantic information and avoids a second
name-construction path in object-format code.

Function effects can be represented as a small ordered enum on the canonical
semantic binding and copied to the existing LowIR call-boundary metadata. This
lets redeclarations and template instantiations merge the strongest declared
effect without a string-keyed side table.

### After PA28

Later hosted tests keep the same host object and ABI/runtime path while adding
hosted source/header compatibility.
