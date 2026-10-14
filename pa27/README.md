## CPPGM Programming Assignment 27 (`cppgm++ -c`)

### Overview

PA27 is the host object/toolchain interoperability assignment. It does not add
new language features. Instead, it combines the PA25 compile-mode driver with
the PA9 ABI naming layer. PA9 built ABI names from normalized fact files;
PA27 connects that same naming work to the compiler's semantic/linkage facts
and writes the resulting names into real object files:

- `cppgm++ -c` must emit ordinary relocatable object files for the current host
  object format.
- Those objects must be accepted by the host C++ compiler driver when it performs
  the final link.
- Objects produced by `cppgm++` must interoperate with host-built C and C++
  objects, static archives, and shared libraries in the practical subset tested
  here.
- Header-emitted inline/template definitions must use host-correct symbol
  spelling and duplicate-definition/coalescing rules.

The PA27 harness does not normally use `cppgm++` as the final linker. It runs:

1. `cppgm++ -c` once for each C++ translation unit.
2. The host C++ compiler driver on the generated objects and any helper objects
   supplied by the test.
3. The linked program, when the link succeeds.

The main PA27 question is: can `cppgm++` produce ordinary host-linkable object
files?

### Prerequisites

Complete PA26 before starting this assignment.

You will want to reuse:

- the full C++ language pipeline through PA25
- the PA25 `cppgm++ -c` driver path
- the PA9 ABI naming layer
- the PA24 native backend and PA25 object-emission path
- the PA25 cross-translation-unit compile/link model

The tests assume a POSIX-like shell environment with `make`, `bash`,
`perl`, and a working host C/C++ toolchain. The harness selects host tools from
environment variables first:

- `CPPGM_HOST_CXX` or `CXX` for the host C++ compiler/link driver
- `CPPGM_HOST_CC` or `CC` for host C helper objects

If those are not set, the harness searches for common compilers such as
`clang++`, `g++`, `c++`, `clang`, `gcc`, and `cc`. Archive and inspection tests
also require `ar`, `nm`, and `readelf`. The checked-in tests assume the normal
x86_64 Linux host object ABI.

### Starter Kit

The starter kit provides:

- your cumulative `dev/cppgm++.cpp` compiler entry point
- the shared implementation you built under `dev/src/`
- `pa27/cppgm++.cpp`, a link to `../dev/cppgm++.cpp`
- `pa27/Makefile`
- `pa27/scripts/`, the host-interoperability test harness
- `pa27/tests/general/`, the PA27 tests and checked-in reference files
- `pa27/tests/controls/`, focused checks of the GNU `section` attribute

Put your code changes in `dev/`, especially `dev/cppgm++.cpp` and the
shared implementation files it calls. Do not edit generated `.my` files. Preserve supplied test inputs.
Corrections to reference outputs follow the
[reference policy](../TESTING_AND_REFERENCES.md).

Use `cppgm++-ref` to inspect example output.
Tests run your implementation against the checked-in contract fixtures.

### Command-Line Contract

PA27 strengthens the PA25 compile-mode surface on the host-compatible path
and uses PA9 ABI names for C++ object symbols. It also adds `-isystem <dir>`
and `-isystem<dir>` for explicit system-header search directories. Preserve
function bodies from those headers, including move constructors; system-header
origin does not make a body ignorable. PA29 adds automatic hosted include
discovery and the remaining hosted preprocessor controls.

Required forms:

```sh
cppgm++ -c -o <objfile> <srcfile>
cppgm++ -c --target <target> -o <objfile> <srcfile>
cppgm++ -c -I <dir> -o <objfile> <srcfile>
cppgm++ -c -I<dir> -o <objfile> <srcfile>
cppgm++ -c --target <target> -I <dir> -o <objfile> <srcfile>
cppgm++ -c --target <target> -I<dir> -o <objfile> <srcfile>
```

`<target>` may be `linux` or the corresponding x86_64 Linux host triple form
accepted by your implementation. PA27 only requires compile mode. The normal
PA27 final link is performed outside `cppgm++` by the host C++ compiler driver.

### Output Format

`cppgm++ -c` shall write one host-linker-compatible relocatable object file to
`<objfile>`.

The PA27 tests do not compare object bytes directly. They observe:

- `cppgm++ -c` exit status
- host final-link exit status
- final program exit status
- final program standard output
- optional object-inspection output for tests that include `.inspect.*` sidecars

### Error Handling

If preprocessing, parsing, semantic analysis, lowering, object emission, or
output writing fails, `cppgm++` shall exit with failure.

For negative tests, exact diagnostics are not the grading contract. The harness
compares exit status first. If the reference compile/link path fails, stdout and
stderr are diagnostic side effects rather than required output.

### Testing

Run the PA27 suite with:

```sh
make test
```

To run one test through the shared check target:

```sh
make check TEST=tests/general/100-host-main-argv.t
```

The local tests live in `tests/general/`. They cover host object
interoperability, host final-link behavior, symbol spelling/coalescing, and
object inspection where the object surface is directly checked. They are
not direct N3485 clause tests.

`make test` also runs the `section-*.t` controls in `tests/controls/`
(`../scripts/check_pa27_section_contract.pl`): a section name that is a safe
token must reach the LowIR as `section=` and the object as a section of that
name, and an unsafe or conflicting section attribute must be rejected.
`make check TEST=tests/controls/<name>.t` runs one.

For each test anchor `x.t`, companion C++ sources are named:

```text
x.t.1
x.t.2
...
```

Optional sidecars control or check the host flow:

- `x.system-includes`: system include search directories passed to `cppgm++`
  as `-isystem <dir>`
- `x.link.flags`: extra flags passed to the host link driver
- `x.lib.*`: host-built C or C++ helper sources
- `x.argv`: program arguments for the runtime check
- `x.inspect.cmd`, `x.inspect.expect`, or `x.inspect.plan`: object-inspection
  checks that use the host symbol tools

The checked-in PA27 tests cover:

- hosted `main(argc, argv)` behavior through the host CRT
- host linking across multiple `cppgm++`-generated objects
- host linking against host-built objects
- host linking against static archives and shared libraries
- import/export of host-built `thread_local` variables in the tested subset
- GNU `section` attributes on global objects whose nonempty ELF section name
  contains only ASCII letters, digits, `_`, and `.`
- duplicate-definition/coalescing behavior for inline and template output
- host symbol spelling for user-defined entities and selected template cases

PA27 does not require hosted standard-library header support. Your compiler does
not need hosted include search, hosted preprocessor compatibility, or semantic
support for hosted headers such as `<exception>` or `<typeinfo>` yet. The PA27
tests cover object emission, host linking, symbol spelling, and cross-object
interoperability through declarations and helper objects that expose the object
boundary directly. Hosted header compatibility begins in a later assignment, and
hosted exception-library runtime behavior is introduced after hosted headers
compile.

### Using PA9 ABI Names

PA27 does not require the broader host C++ ABI/runtime behavior exercised later,
but ordinary host object interoperability already requires correct raw symbol
spelling for user-defined entities.

The object-file requirement is that visible symbol names match the configured host
ABI. The PA9 ABI naming layer is the recommended path for producing those
names:

- the host linker sees raw symbol names, not demangled intent
- function templates must encode template-parameter references with the same
  `T_`, `T0_`, and related forms the host compiler uses
- repeated components inside one mangled name must reuse Itanium substitution
  slots in host-compatible order
- canonical qualified names matter, including inline namespaces when they are
  part of the ABI name

Where Itanium leaves a name to the implementation, the reference follows
clang. The object-inspection tests check the spelling of a symbol another
unit can name (external and weak symbols); a unit-local symbol they check
only by its binding and by a pattern that admits clang's and GCC's
spellings, and only when the program needs it (a private helper such as the
thunk behind a closure's function-pointer conversion is not required under
any name), so the forms below for local entities (`$_<n>`, `__invoke`, the
`_ZGR` numbering) describe the reference and are not required:

- a namespace-scope variable or function of internal linkage carries the `L`
  marker (`_ZL4data`, `_ZN2nsL1vE`, `_ZL4sfunv`), except inside an unnamed
  namespace, whose `_GLOBAL__N_1` prefix is already unit-local
- a function's static variable and its guard are local names
  (`_ZZ3getvE5local`, `_ZGVZ3getvE5local`); where the function is inline or
  a template specialization every unit names them alike, so they are one
  object across units; a constructor's or destructor's statics are named in
  its complete entry (`_ZZN1SC1EvE5count`, where GCC writes `C4`) and are
  one object for both entries
- a temporary whose lifetime a static-storage variable's initializer
  extends (a reference's, or an initializer_list's array) is named after
  the variable (Itanium 5.1.4.5): the first is `_ZGR1g_`, and the others
  are numbered in a pre-order walk of the initializer (`_ZGR4list_` for a
  reference's initializer_list object, `_ZGR4list0_` for its array); the
  symbols are unit-local, since every unit reaches the temporary through
  the variable
- a closure type has a mangling number (`Ul<sig>E<n>_`) when its lambda lies
  in an inline function or a template specialization, a lambda body
  included, in a default member initializer, a class template's static
  data member initializer or a variable template's initializer (named in the
  data member, `_ZNK8WithInit2fpMUlvE_clEv`,
  `_ZN5TableIiE4sizeMUlvE_8__invokeEv`, `_ZNK2vtIiEMUlvE_clEv`; GCC omits
  the `M` for a variable template) or
  in an in-class default argument (named in the function and parameter,
  `...Ed_NKUlvE_clEv`); any other closure type, and an unnamed class that
  no typedef names for linkage, is a unit-local `$_<n>` with local
  symbols (a lambda in a non-template class's static data member
  initializer is named in its class, `_ZN5Plain3$_08__invokeEv`); a
  captureless closure's static invoker is `__invoke` (GCC's is `_FUN`)
- a `$_<n>` number is assigned when the name is first mangled: a
  function-local unnamed class or closure takes the next number within its
  function, and any other one takes the count of `$_` names the translation
  unit has assigned so far
- a function template's signature that depends on an expression keeps the
  expression as clang mangles it. A dependent decltype is `DT<expr>E`, a
  dependent template argument is `X<expr>E` and a dependent array bound is
  `A<expr>_`; a parameter the result names is `fp_`, and one a later
  parameter names is `fL0p_`. A non-dependent sizeof or template argument is
  written as its value (`Lm8E`, `Lb1E`), and a non-dependent class's static
  member as its declaration (`L_ZNSt14numeric_limitsImE6digitsE`). A callee
  that is a class's member function is written as its declaration, and an
  unqualified function callee as its name. A dependent name's qualifier is
  written without the template's default arguments
  (`sr11call_traitsIS1_T_FvT0_EEE8overload`), and a template-id that an
  alias parameter binds is written as its unqualified name
  (`sr9is_scalarIT_EE5value`)
- a numbered closure type, and a local class of an inline function or
  template specialization, is one entity in every unit: its member
  functions, the static invoker, the static variables in their bodies and
  the template specializations over it are weak (`W _ZZ1hvENKUlvE_clEv`),
  unit-local only where the function they belong to is
- a lambda expression has one closure type however many times its context
  is lowered: a lambda in a default member initializer is one closure in
  every constructor that initializes the member; one in an in-class default
  argument is one at every call that uses it; and one in a class template's
  static data member initializer is one per specialization

Reference:

- Local copy of Itanium C++ ABI, Chapter 5.1 "External Names (a.k.a.
  Mangling)": [`../doc/itanium-mangling.txt`](../doc/itanium-mangling.txt)

### Required Implementation Surface

To complete PA27, implement ordinary host-toolchain interoperability of emitted
object files within the supported subset:

1. Emit host-linker-compatible relocatable objects. The course links them
   without PIE; an imported object's address may be materialized through
   the GOT (as the reference does, which a default-PIE link also accepts)
   or by a PC-relative or absolute reference the host linker resolves.
2. Expose a hosted entrypoint through the host CRT.
3. Preserve cross-translation-unit behavior under host link.
4. Emit target-correct duplicate-definition semantics for header and template
   code.
5. Interoperate with host-built objects, archives, shared libraries, and tested
   `thread_local` variables through practical function/global boundaries.
6. Preserve the earlier class-value semantics in host-object mode: a
   nontrivial copy or move constructor or assignment runs its memberwise
   body exactly once, with no whole-object copy after it.
7. Retain every local function whose address or body emitted code requires.
   Omitting an unreferenced translation-unit-local function is the course
   solution's choice and is not graded.
8. Retain a reachable constructor or destructor base-entry definition needed
   at the object boundary even when optimization removes its last direct call.
   Retention does not by itself prohibit inlining that entry at other call
   sites.
9. For the supported GNU `section` attribute on a global object, require one
   string-literal name, reject conflicting redeclarations and names outside the
   token-safe Linux subset above, carry the name as global LowIR
   `section=<name>` metadata, and place the symbol and its relocations in that
   ELF section. Function sections, segment syntax, and quoted LowIR metadata
   are not part of PA27.

If the host linker rejects generated objects as ordinary objects, fix the
host-compatible object-emission path.

### Out Of Scope

The PA27 tests do not require:

- host C++ ABI/runtime behavior after link
- hosted standard-library header/source compatibility
- hosted header-emitted link/runtime behavior
- bootstrap or self-host builds

### Design Notes (Non-Normative)

A simple implementation strategy is to keep PA25's source-to-LowIR path, use
PA9 to derive concrete C++ object symbols, and retarget only the object
emission details needed by the host object format. The observable result is
whether the host toolchain can consume and link the result, not whether your
internal object pipeline has the same structure as the course implementation.

One practical integration point is the compiler semantic/linkage layer: after
semantic analysis determines the entity, owner scopes, function type, template
arguments, local context, ABI tags, and special-name kind, that layer can feed
those facts to the PA9 mangler and store the resulting raw symbol on the
LowIR/object symbol. This keeps lower object-format and toolchain-driver code
focused on preserving the spelling it was given instead of reconstructing C++
ABI names from text fragments.

Give every definition the narrowest binding the language allows. Put each
inline function and template specialization, and each other definition with
vague linkage, in its own COMDAT group with a weak symbol, and give a
definition with internal linkage a local symbol. Both make a definition
discardable: a translation unit that stops referring to it can leave it out,
because another unit that needs a vague-linkage definition emits its own
copy and nothing outside the unit names a local one. Item 7 relies on the
second; PA32's single-call inlining and pruning rely on both. Keep each
section's relocations in their own list, sorted by offset, so that emitting
or dropping one definition never scans the relocations of the whole object.

### After PA27

Later host-link tests keep the same object path and then exercise richer host
C++ ABI/runtime behavior.
