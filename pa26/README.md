## CPPGM Programming Assignment 26 (`cppgm++ -c` Host EH Facts)

### Overview

PA26 is the host exception-handling metadata assignment. Earlier assignments
lower C++ source to LowIR and native code; PA26 makes EH-bearing `cppgm++ -c`
objects participate in the host C++ unwinder.

The main PA26 question is: does a generated relocatable object contain the host
EH facts needed by the platform unwinder?

The required surface is the basic Itanium C++ ABI exception subset used by the
course:

- calls to host EH runtime helpers such as `__cxa_allocate_exception`,
  `__cxa_throw`, `__cxa_begin_catch`, `__cxa_end_catch`, and
  `_Unwind_Resume`
- a personality reference to `__gxx_personality_v0` when a function has landing
  pads
- host unwind metadata and LSDA/call-site information, such as
  `.gcc_except_table` and `.eh_frame`
- type-info references needed for typed catches
- no private course-only `cppgm_eh_*` runtime symbols in host-EH objects
- local object binding for compiler-generated functions whose enclosing source
  context is not ODR-mergeable, including a lambda call operator inside an
  ordinary non-inline function

PA26 is intentionally a host-object facts assignment, not a hosted standard
library assignment and not a private linker/runtime pipeline.

### Prerequisites

Complete PA25 before starting this assignment.

You will want to reuse:

- the PA8 LowIR parser and EH instruction model
- the PA24 native backend and PA25 object pipeline
- the PA19-PA23 source-to-LowIR surface
- the PA25 compile-mode driver path used by `cppgm++ -c`
- the PA9 ABI naming layer and runtime-role classification used by host object
  emission

The tests assume a POSIX-like shell environment with `make`, `bash`, `perl`, and
a working host C/C++ toolchain. The harness selects host tools from:

- `CPPGM_HOST_CXX` or `CXX` for the host C++ compiler/link driver
- `CPPGM_HOST_CC` or `CC` for host C helper objects

If those are not set, the harness searches for common compilers such as
`clang++`, `g++`, `c++`, `clang`, `gcc`, and `cc`. Object-inspection tests also
require host symbol/object tools such as `nm` and `readelf`.

### Starter Kit

The starter kit provides:

- your cumulative `dev/cppgm++.cpp` compiler entry point
- the shared implementation you built under `dev/src/`
- `pa26/cppgm++.cpp`, a link to `../dev/cppgm++.cpp`
- `pa26/Makefile`
- `pa26/scripts/`, the host-interoperability test harness
- `pa26/tests/general/`, the PA26 tests and checked-in reference files

Put your code changes in `dev/`, especially `dev/cppgm++.cpp` and the
shared implementation files it calls. Do not edit generated `.my` files. Preserve supplied test inputs.
Corrections to reference outputs follow the
[reference policy](../TESTING_AND_REFERENCES.md).

Use `cppgm++-ref` to inspect example output.
Tests run your implementation against the checked-in contract fixtures.

### Command-Line Contract

PA26 uses compile mode:

```sh
cppgm++ -c -o <objfile> <srcfile>
cppgm++ -c --target <target> -o <objfile> <srcfile>
cppgm++ -c -I <dir> -o <objfile> <srcfile>
cppgm++ -c -I<dir> -o <objfile> <srcfile>
cppgm++ -c --target <target> -I <dir> -o <objfile> <srcfile>
cppgm++ -c --target <target> -I<dir> -o <objfile> <srcfile>
```

`<srcfile>` is a C++ source file in the supported course language subset.
`<target>` may be `linux` or the corresponding x86_64 Linux host triple form
accepted by your implementation. PA26 only requires compile mode. The final link
in the tests is performed outside `cppgm++` by the host C++ compiler driver.

### Output Format

`cppgm++ -c` shall write one host-linker-compatible relocatable object file to
`<objfile>`.

The PA26 tests do not compare object bytes directly. They observe:

- `cppgm++ -c` exit status
- host final-link exit status
- final program exit status
- final program standard output
- normalized object-facts output for tests that include `.inspect.facts`
  sidecars

The object-facts sidecars are part of the PA26 test surface. The shared Perl
harness dumps platform-normalized facts such as required EH runtime imports,
unwind/LSDA section presence, relocation classes, decoded basic LSDA facts, and
absence of private `cppgm_eh_*` symbols.

### Error Handling

If preprocessing, parsing, semantic analysis, lowering, object emission, or
output writing fails, `cppgm++` shall exit with failure.

For negative tests, exact diagnostics are not the grading contract. The harness
compares exit status first. If the reference compile/link path fails, stdout and
stderr are diagnostic side effects rather than required output.

### Standard Output / Error

Standard output and standard error from `cppgm++ -c` are ignored for successful
tests. They may be used for diagnostics.

### Testing

Run the PA26 suite with:

```sh
make test
```

To run one test through the shared check target:

```sh
make check TEST=tests/general/100-host-eh-same-tu-throw-catch.t
```

The local tests live in `tests/general/`. They cover the basic host-EH fact
surface:

- same-translation-unit throw/catch
- cross-translation-unit throw/catch
- unhandled throw helper usage
- cleanup during unwind and `_Unwind_Resume`
- cleanup-only landing pads that resume without owning a throw helper
- LSDA/unwind sections, runtime-helper relocation classes, and class typeinfo
  facts used by typed catches
- reuse of host EH runtime declarations emitted by the frontend
- source-driven host-EH object smoke tests used to guard the backend path
- call-site coalescing safety across unprotected unwind barriers and distinct
  cleanup or catch continuations

The cases compile at `-O0`, including nested catch forwarding. Optimization
is introduced in PA32; its host-object lane checks that forwarding and unwind
metadata remain correct after optimization. PA26 requires no optimized build.

For each test anchor `x.t`, companion C++ sources are named:

```text
x.t.1
x.t.2
...
```

Optional sidecars control or check the host flow:

- `x.link.flags`: extra flags passed to the host link driver
- `x.lib.*`: host-built C or C++ helper sources
- `x.inspect.facts`: normalized host-EH object facts to dump and compare
- `x.inspect.cmd`, `x.inspect.expect`, or `x.inspect.plan`: specialized
  object-inspection checks that use host symbol/object tools

For each test case:

1. `cppgm++ -c` is executed once for each companion C++ source file.
2. The host C++ compiler driver links the generated objects.
3. Any inspect sidecar is run against the generated objects. For
   `.inspect.facts`, the harness records normalized text facts in
   `x.my.inspect`.
4. If linking and inspection succeed, the generated program is executed.
5. The recorded `.my.*` outputs are compared with the checked-in `.ref.*`
   oracle files.

### Required Implementation Surface

To complete PA26, implement the basic host-compatible EH metadata and
runtime-helper object surface for `cppgm++ -c` within the supported subset:

1. Lower `throw` expressions to host ABI throw helper calls.
2. Lower typed catches to host landing-pad selector dispatch and
   `__cxa_begin_catch` / `__cxa_end_catch` calls.
3. Emit unwind metadata (a CIE and an FDE) for every function, since an
   exception passes through frames that have no handler, and the host
   personality and LSDA for EH-bearing functions.
4. Emit LSDA/call-site/action/type-info facts sufficient for basic catch and
   cleanup paths.
5. Preserve cleanup/resume paths using `_Unwind_Resume`.
6. Keep private course-only exception runtime symbols out of host-EH objects,
   and encode the host personality without a direct PC-relative data relocation
   that would make a default-PIE host link unsafe.
7. Enter every landing pad with one set of active regions: a path into a
   block carries the same protected-call state as every other path into it
   (`../pa8/lowir.md`, "Structural Rules").
8. Preserve translation-unit-local object binding for generated functions in a
   non-ODR-mergeable local context; do not export a local lambda call operator
   as a weak or global host symbol.
9. If adjacent LSDA call-site ranges are coalesced, keep an unprotected
   potentially-throwing range as a barrier and never combine ranges with
   different landing pads or action continuations.
10. Reach `std::terminate` through the host runtime when an exception
    leaves a non-throwing function: a landing that calls `__cxa_begin_catch`
    and then `std::terminate`, or the runtime's `__cxa_call_terminate`. How
    many landings or helpers do it, and what they are named, is not graded;
    the program's behavior is.
11. End every cleanup landing by calling `_Unwind_Resume` with the exception
    object it received. How many resume sites a function has is your choice.
12. Cover every potentially throwing call in an LSDA-bearing function with a
    call-site entry, a null landing pad where nothing handles it, so that
    unwinding continues through the function. Whether entries also cover the
    instructions between calls, and whether adjacent unprotected calls share
    an entry, is the table's layout and is not graded.

If object inspection shows missing or malformed host EH metadata for a basic
throw/catch/cleanup case, fix the host-EH lowering or object-emission path.

### Out Of Scope

The following are out of scope for PA26:

- a private object/link/runtime pipeline
- general host object interoperability unrelated to EH metadata
- richer host ABI/runtime behavior after the basic EH facts exist
- complex RTTI/vtable/virtual-base catch interactions
- multi-frame or nested rethrow/cleanup behavior
- rethrow behavior and `__cxa_rethrow`
- hosted standard-library header/source compatibility
- bootstrap or self-host builds

Later host-EH tests keep the same host-link path while exercising richer
host ABI/runtime interactions such as foreign catch-all, virtual-base catches,
nested cleanup chains, and hosted library EH behavior.

### Design Notes (Non-Normative)

A useful implementation shape is to keep frontend LowIR EH operations stable and
classify runtime roles below LowIR. Object emission can then map those roles to
host ABI symbols and platform EH metadata:

- ELF uses `.eh_frame`, `.gcc_except_table`, and the corresponding relocation
  records.

Do not construct host EH facts from source text. The object backend should work
from typed semantic/runtime-role information and final machine layout.

Generate each function's call-frame instructions from the frame facts the
encoder records as it emits the prologue and epilogues: each push, the stack
reservation, where the prologue sits, and each epilogue, with its own rows
when returns have their own epilogues. Do not write them from a fixed
`push rbp; mov rbp, rsp` template. Build the LSDA's call-site table from the
final addresses of the calls after block layout. With both, later
optimization can omit the frame pointer, push callee-saved registers, start
the prologue after an early exit, give each return its own epilogue, and move
cold blocks and landing pads to the end of the function, all without
changing the unwind writer. A table built from an assumed frame shape or an
assumed block order has to be rewritten for each of those. Keep layout and
the call-site table in step: when layout removes a jump so that one block
falls into the next, the calls of the next block still need the entries of
their own region, and a landing pad is entered only by unwinding, never by
falling into it.

A compact terminate boundary can pass the typed exception value to a single
internal helper. This keeps the handler-entry ABI sequence in one place while
leaving ordinary source catch handlers independent.

The host-object layout walk can count MIR resume operations once, allocate a
terminal only for a function that needs one, and branch each resume to it. A
single typed frame-slot identity is sufficient; rendered slot or label names
are not needed.

The same layout walk can retain exact unprotected potentially-throwing call
ranges. Merge those ranges with protected call sites in address order when
writing the LSDA, coalescing unprotected calls only within one interval between
protected sites. This avoids reconstructing call-site coverage from the full
function byte range.
