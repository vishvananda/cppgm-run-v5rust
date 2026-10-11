## CPPGM Programming Assignment 29 (`cppgm++ -E` / `cppgm++ -c`)

### Overview

PA29 is the hosted source/header compatibility assignment. Its job is to make
`cppgm++` preprocess and compile the hosted standard-library and vendor
extension environment needed by later bootstrap-style builds.

This milestone is intentionally distinct from the previous host-toolchain
assignments:

- PA27 is about ordinary host-linkable object files.
- PA28 is about host C++ ABI/runtime correctness after host link.
- PA29 is about preprocessing, parsing, semantic analysis, and lowering
  compatibility for hosted source/header inputs.

To complete PA29, implement these goals:

- hosted preprocessor compatibility
- GNU/Clang parser concessions used by the selected hosted headers
- GNU builtin type and literal forms used by the selected hosted environment
- builtin traits, transforms, and intrinsics used by that hosted environment
- hosted-header/source compile compatibility for the lighter hosted workload
  covered by the PA29 tests, including C-wrapper headers and selected
  vendor/header forms

### Prerequisites

Complete PA28 before starting this assignment.

You will want to reuse:

- the full earlier language, template, and lowering stack
- the PA27/PA28 `cppgm++ -c` host-object path
- the PA28 host-ABI-compatible output path
- the earlier preprocessor pipeline, now with hosted-driver controls

The GNU/Clang concessions this assignment owns include several the selected
hosted headers depend on:

- the `aligned` attribute takes a constant expression, not only a literal, and
  inside a class template that expression may name the template's own
  parameters, so it is not knowable until instantiation; it is accepted in
  the `[[gnu::aligned]]` spelling too, and before a block-scope declaration or
  after a declarator it aligns the declared object
- a hosted type specifier such as `__uint128_t` or `__float128` spells itself as
  an identifier rather than a keyword, and introduces a functional cast the same
  way a fundamental type keyword does
- `_Atomic` is not `volatile`: a class with an `_Atomic` member is still a
  literal type and may have a `constexpr` constructor

The tests assume a Linux shell environment with `make`, `bash`, `perl`, and a
working host C++ compiler with hosted C++ headers installed.
You may override the compiler with `CXX=...`. `CPPGM_HOST_CXX` selects the host
compiler used for builtin macro/include probing. If it is not set, it defaults
to `CXX`.

The hosted tests rely on the host compiler's target, predefined macros, standard
library include paths, and standard-library selection flags. When you use a
non-default standard library, pass the same choice through `CPPGM_STDLIB_FLAGS`
so the course compiler and host compiler agree.

Library traits use their declared template definitions and specializations.
A familiar standard-library name does not define an incomplete primary or
replace its member declarations and values.

Those answers are discovered **once, when `cppgm++` is built**, and baked into
it. The compiler does not probe the host toolchain at run time to find its
include paths: a compiler that decided where the standard library lives each
time it ran would give different answers on different machines and would make
its own output depend on whatever compiler happened to be installed. A
consequence students see directly is that `-stdlib=` on the command line cannot
change the choice. It is checked against the selection the compiler was built
with, so it may agree, and a flag that disagrees is refused rather than
honoured -- honouring it would compile against one library's headers while
claiming another.

### Starter Kit

The starter kit provides:

- your cumulative `dev/cppgm++.cpp` compiler entry point
- the shared implementation you built under `dev/src/`
- `pa29/cppgm++.cpp`, a link to `../dev/cppgm++.cpp`
- `pa29/Makefile`
- `pa29/scripts/`, the hosted preprocessor/compile test harness
- `pa29/tests/preproc/`, hosted preprocessor tests and references
- `pa29/tests/compile/`, hosted compile-only tests and references
- `pa29/tests/run/`, small host-link/run smokes for hosted C-wrapper headers
  and builtin/runtime interop

Put your code changes in `dev/`, especially `dev/cppgm++.cpp` and the
shared implementation files it calls. Do not edit generated `.my` files. Preserve supplied test inputs.
Corrections to reference outputs follow the
[reference policy](../TESTING_AND_REFERENCES.md).

Use `cppgm++-ref` to inspect example output.
Tests run your implementation against the checked-in contract fixtures.

### Driver Surface

Previously required:

- the PA25 compile/link surface: `-c`, default link mode, `-I`, `-L`, `-l`, and
  `--target`
- the PA27/PA28 host-compatible behavior for the relevant compile-mode subset

New or newly required in PA29:

- hosted preprocess mode: `-E`
- hosted preprocessor-control flags:
  - `-D <macro>` and `-D<macro>`
  - `-U <macro>` and `-U<macro>`
  - `-include <file>`
  - `-isystem <dir>` and `-isystem<dir>`
- direct driver query forms:
  - `--version`
  - `-v`
  - `-dumpmachine`
  - `-dumpversion`
  - `-print-search-dirs`
- compatibility handling for common build-system flags that should either be
  honored or harmlessly accepted when they do not affect the tested output

### Command-Line Contract

PA29 continues extending the same `cppgm++` frontend used in PA25-PA28.

Required preprocess forms:

```sh
cppgm++ -E -o <outfile> <srcfile>
cppgm++ -E <srcfile1> <srcfile2> ...
cppgm++ -E -D <macro> -U <macro> -include <file> -isystem <dir> <srcfile>
```

Required compile forms:

```sh
cppgm++ -c -o <objfile> <srcfile>
cppgm++ -c <srcfile1> <srcfile2> ...
cppgm++ -c -D <macro> -U <macro> -include <file> -isystem <dir> -o <objfile> <srcfile>
```

Query flags are required only as direct driver queries. For example:

```sh
cppgm++ --version
cppgm++ -dumpmachine
cppgm++ -print-search-dirs
```

### Output Format

`cppgm++ -E` shall write the same structured posttoken/preprocessor stream
format used by the PA4 `preproc` frontend. When `-o <outfile>` is present, that
stream is written to `<outfile>`.

`cppgm++ -c` shall continue to write host-linker-compatible relocatable object
files as in PA27/PA28.

The new PA29 requirement is not a new object format. It is the ability to
preprocess and compile hosted source/header inputs successfully through the
`cppgm++` path.

Hosted compatibility should use the same source-to-LowIR-to-object pipeline as
ordinary compilation. Hosted include search, predefined macros, and builtin
probes can affect the source program being compiled, but `--emit-lowir` should
remain representative of the LowIR that object emission consumes: the object
`cppgm++ -c` writes must be the object the serialized LowIR produces (PA32's
object round trip checks it), so the facts hosted code needs in the backend
are LowIR facts; how they travel in memory is yours.

### Error Handling

If preprocessing, parsing, semantic analysis, lowering, object emission, or
output writing fails, `cppgm++` shall exit with failure.

For compile-only tests, exact diagnostics are not the grading contract. The
harness compares exit status and any checked output sidecars. If the reference
run fails, stdout and stderr are diagnostic side effects rather than required
output.

### Testing

Run the PA29 suite with:

```sh
make test
```

To run one test through the shared check target:

```sh
make check TEST=tests/preproc/300-has-include.t
make check TEST=tests/compile/600-builtin-transforms-and-traits.t
make check TEST=tests/run/800-hosted-cmath-ceil-run.t
```

PA29 has three test directories:

- `tests/preproc/`: hosted preprocessor compatibility. The oracle is the
  PA4-style structured preprocessor stream plus exit status.
- `tests/compile/`: hosted compile-only compatibility. The oracle is successful
  object emission, compile exit status, and any stdout/reference sidecars used
  by the harness.
- `tests/run/`: narrow host-link/run compatibility smokes. These tests cover
  hosted builtins, C library entry points, and lightweight hosted C-wrapper
  headers such as `<cassert>`, `<cmath>`, `<cstdarg>`, `<cstddef>`, `<cstdio>`,
  `<cstdlib>`, `<cstring>`, and `<csignal>`. They do not require broad template-heavy C++
  standard-library headers or general hosted header-generated runtime support.

The checked-in tests are hosted/vendor compatibility cases, standard-library
sentinels, reducers, and bootstrap-facing compile smokes rather than direct
N3485 clause tests.

Optional sidecars include:

- `x.env`: environment variables for one test, such as additional standard
  include paths
- `x.no-exceptions`: compile a `tests/compile` input with `-fno-exceptions`
- `x.cxx-standard`: compile in the named `c++11`, `c++14`, or `c++17`
  language mode
- `x.ref.impl.exit_status`, `x.ref.program.exit_status`, and
  `x.ref.program.stdout`: implementation and program-result references for
  `tests/run` host-link/run smokes

The default preprocessor references are intentionally host-agnostic. The test harness checks that checked-in PA29 preprocessor refs do not accidentally
pin local host macro values such as platform-specific integer or floating-point
limits.

### Hosted ABI Name Guidance

PA29 is mostly about accepting hosted headers, but compile-only hosted tests can
still expose symbol-spelling problems through emitted objects and unresolved
references. The `cppgm++ -c` path should continue producing host ABI names that
PA27 and PA28 already made observable.

A recommended implementation style is to keep the PA9 mangler in the
compile-mode path while adding hosted parser, semantic, builtin, and lowering
support. That style works best when hosted standard-library entities, inline
namespaces, ABI-tagged declarations, dependent template names, local entities,
and function template specializations keep the semantic information needed to
name them.

For compile-only tests, the immediate oracle may be successful object emission,
but the object must carry host ABI names that PA31 can link and run without a
separate hosted-only naming scheme.

### Required Implementation Surface

To complete PA29, implement hosted compatibility for:

- predefined macro import, `__COUNTER__`, the GNU comma paste, `_Pragma`,
  `__has_*`, `#include_next`, `#warning`, ignored unknown pragmas, and hosted
  hex-float preprocessing forms such as `0x1p+4`.  `#warning` lets
  translation continue, like an ignored pragma; anything it prints goes to
  standard error, which the tests do not check.  `__COUNTER__` expands to
  `0` at its first use in a translation unit and to one more at each later
  use.  In a variadic macro's replacement list, the GNU form
  `, ## __VA_ARGS__` removes the comma when the variable arguments are empty
  as written; otherwise the comma stays and the arguments follow it, pasting
  nothing.  A probe answers for its whole family or not at all: a name the
  preprocessor treats as defined but does not substitute leaves the header's
  own fallback macro unused and the probe call sitting in the controlling
  expression.  `__has_cpp_attribute` takes an attribute-token, which N4868
  15.2/2 spells as an identifier or a scoped `identifier :: identifier`, and
  `__has_warning` takes a string literal; the rest take a single identifier.
  `__has_cpp_attribute` answers nonzero only for an attribute the compiler
  implements, here the standard `no_unique_address`, also spelled
  `__no_unique_address__`, and `0` for any other name, scoped ones included;
  `__has_attribute` likewise answers nonzero for the GNU and Clang
  attributes the compiler implements (`always_inline`, `noinline`,
  `noreturn`, `pure`, `const`, `used`, `weak`, `packed`, `aligned`,
  `section`, `vector_size`, `gnu_inline`, `abi_tag`, `using_if_exists` and
  `exclude_from_explicit_instantiation`, each also spelled `__name__`) and
  `0` for any other.
  `__has_builtin` answers from the registries that hold the builtins rather
  than from a list beside them: claiming not to have one that is implemented is
  worse than lacking it, because the header then takes a fallback written in
  later-standard machinery that has to be supported instead of the builtin
  already there.
  Probes for guarantees the compiler does not make, such as
  `__has_declspec_attribute`, answer no rather than failing
- GNU/Clang parser concessions commonly exercised by the selected hosted
  headers, including dependent nested-angle disambiguation, nested qualified
  template-ids used as outer template arguments, builtin-trait identifiers
  referenced as ordinary names, GNU `__decltype`, parenthesized
  throw-expressions emitted by hosted helper macros, and GNU builtin float type
  specifiers such as `__float128` / `_Float128`, plus GNU complex component
  operators `__real__` and `__imag__`, and the C++20 conditional
  explicit-specifier `explicit(constant-expression)`, which both host compilers
  accept in C++11 mode as an extension and libc++ writes unconditionally
  outside its C++03 branch.  Its condition decides whether the constructor
  takes part in copy initialization, so a parse that accepts the form and
  discards the condition silently mis-ranks overloads.  Attributes interleave
  with the specifiers around it rather than preceding them all
- builtin traits, transforms, intrinsics, and builtin families used during
  hosted compile acceptance, including lowering `__builtin_abort` as a
  non-returning call to the host C runtime; fixed-arity hosted integer
  intrinsics reject calls with the wrong number of operands.  This includes the
  type-shape traits a hosted library may spell directly rather than derive from
  partial specializations: `__is_const`, `__is_volatile`, `__is_void`,
  `__is_array`, `__is_bounded_array`, `__is_unbounded_array`,
  `__is_lvalue_reference`, `__is_rvalue_reference`, `__is_object`,
  `__is_arithmetic`, `__is_fundamental`, `__is_compound`, `__is_referenceable`
  and `__is_unsigned`.  Note that cv-qualification belongs to the type, so
  `__is_const` is false for a reference to const however it was written.
  `__is_base_of` relates two *class* types, so it is false when either operand
  is a reference even though the reference names a class; a hosted library
  turns on that answer to remove an rvalue-stream overload when the stream
  deduces to an lvalue reference.  It
  also includes the libm intrinsic family a hosted `<cmath>` forwards to, in
  its `float` / `double` / `long double` spellings.  Most take their own result
  type in every operand, so the arity is the whole signature; the rest name
  their own shape, reading an exponent back through a pointer (`frexp`),
  taking one in (`ldexp`, `scalbn`, `scalbln`), writing a whole part
  (`modf`), writing a quotient (`remquo`), taking a `long double` second
  operand (`nexttoward`), or returning an integral type rather than the
  operand's (`ilogb`, `lrint`, `lround`, `llrint`, `llround`).  All of them
  lower to a call to the C function of the same name.  A builtin is a
  global-namespace name, so `::__builtin_x` names the same one as
  `__builtin_x`, which is how a hosted library keeps a user's macro from
  intercepting the call; any deeper qualification names something else.  It
  also includes the variadic builtins `<cstdarg>` expands to: beside
  `__builtin_va_start` and `__builtin_va_arg`, which lower to the `va_start`
  and `va_arg` of `../pa8/lowir.md`, `__builtin_va_copy` copies the
  argument-list object and `__builtin_va_end` does nothing.
  `__builtin_expect(e, c)` is the value of `e` converted to `long`; LowIR has
  no form for the expectation, so it leaves no other trace.
  `__builtin_unreachable()` lowers to `unreachable`, as in PA11.
  `__builtin_assume` is not required; without it
  `__has_builtin(__builtin_assume)` answers 0 and a header takes its fallback
- hosted compiler intrinsics lowered directly to typed LowIR operations do not
  unwind; an explicit `noexcept` wrapper containing only such operations shall
  not acquire a terminate landing
- a force-inline function that does not return ends its caller's block: nothing
  follows the call, so there is no continuation to split off.  Inlining it must
  still place the callee's body, and the block the callee's returns would have
  landed in is simply not reachable.  A hosted library declares its throw
  helpers this way, so refusing the shape rejects the header
- semantic and lowering compatibility for hosted source patterns used by those
  headers, including post-declarator parameter
  attributes, explicit specializations of primary-template member functions,
  placeholder-return declarations without a body, such as a deleted overload
  declared `auto` (a C++14 form that newer hosted headers use; PA20 leaves it
  out of scope, and like PA20's deduced return types it is accepted in every
  language mode), non-standard hex-float compile acceptance on ordinary
  floating types,
  and `[[no_unique_address]]` empty-member layout through generated copy operations
- the standard `[[noreturn]]` attribute takes no argument and appertains only
  to the declarator-id of a function declaration (7.6.3p1): a variable,
  including a block-scope one, a data member, a parameter, a typedef or
  alias-declaration, a class or enumeration head, a statement, and the
  position after a parameter list
  (where it would appertain to the function type) are rejected. Leading
  attributes of a block item appertain to the declaration it introduces
- a function declared `[[noreturn]]` or GNU `noreturn` on any declaration
  states `return=noreturn` on its declarations and definition and on the
  call signature of a virtual call (`../pa8/lowir.md`, "Facts The C++
  Lowering States"; a direct call has no signature to carry it), and so
  do a function template's specializations, while an explicit specialization
  keeps only its own declarations'; a call through a pointer states none, and
  a call of such a function does not end its block. PA32's `-O0` driver lane
  compares the fact
- semantic validation of primary-source GNU inline function bodies even when
  emission is deferred; an unimplemented reserved compiler builtin may defer
  its wrapper, but ordinary lookup and type errors in an unused wrapper must
  still be diagnosed
- typed retention of a scalar GNU `vector_size` typedef's byte width for
  compile-time layout queries and semantic validation of unused inline wrapper
  literals; runtime vector operations and vector-expression lowering are not
  required
- lightweight hosted C-wrapper header and C runtime interop smokes where the
  header surface is mostly builtin macros/functions or ordinary C declarations
  (`<cassert>`, `<cmath>`, `<cstdarg>`, `<cstddef>`, `<cstdio>`, `<cstdlib>`,
  `<cstring>`, `<csignal>`)

If a failing PA29 test exposes a bug in syntax, semantic analysis, template
handling, lowering, or object emission that is shared with earlier language
features, fix the shared compiler behavior rather than adding a PA29-only path.

### Out Of Scope

The PA29 tests do not require:

- host object/link/runtime behavior beyond the PA27/PA28 host-compatible path
- compiling broad template-heavy C++ standard-library headers such as
  `<vector>`, `<tuple>`, `<functional>`, `<iostream>`, `<string>`, or container
  and stream headers
- general hosted C++ header-emitted link/runtime behavior
- full build-system emulation beyond the documented query and compatibility
  flags
- recursive hosted-header coverage reporting
- bootstrap or self-host builds

### Design Notes (Non-Normative)

Hosted compatibility is easiest to approach as a sequence of small compatibility
surfaces: preprocessor probes, parser concessions, builtin traits/types, and
then semantic/lowering cases. Keep fixes tied to the source pattern being
exercised. Avoid making broad source-text special cases when an earlier
semantic or template representation can carry the information directly.

The typed intrinsic kind can also serve exception-boundary analysis directly.
Keeping that fact on the semantic call avoids treating a lowered atomic,
vector, or compiler operation as an unknown indirect call.

That same preference applies to hosted ABI names: keep type structure, template
arguments, ABI tags, and local context available until the ABI naming layer can
consume them. Reconstructing those facts later from pretty-printed strings tends
to create fragile library-version-specific behavior.

### After PA29

Later hosted tests keep the same hosted source/header environment while adding
larger template-heavy standard-library headers and then link/run behavior for
code emitted from hosted headers.
