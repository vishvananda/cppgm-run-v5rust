## CPPGM Programming Assignment 22 (`cppgm++ --emit-lowir`)

### Overview

PA22 completes the remaining non-virtual object-model work that still fits the current
single-vptr ABI:

- non-virtual multiple inheritance
- member lookup and access across multiple base subobjects
- constructor, copy, and destructor generation across multiple non-virtual bases
- member pointer type formation, null/conversion handling, and `.*` / `->*`
  application over the completed non-virtual object model
- `dynamic_cast<void*>` for the current polymorphic single-inheritance ABI

PA22 still produces LowIR. It does not introduce a new output format.

### Prerequisites

You should complete Programming Assignment 21 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6-PA7 semantic foundation
- the PA10-PA21 LowIR lowering path
- the PA8 LowIR contract

### Starter Kit

The starter kit contains:

- `pa22/README.md`, `pa22/Makefile`, and the test scripts in `pa22/scripts/`
- your cumulative `dev/cppgm++.cpp` compiler entry point
- the `pa22/cppgm++.cpp` symlink back to `../dev/cppgm++.cpp`
- shared support sources and headers under `dev/src/`
- the grammar for this assignment called `pa22.gram`
- an HTML grammar explorer of `pa22.gram` in the sub-directory `grammar/`
- a checked-in local test suite under `tests/`

Students should implement the assignment in `dev/cppgm++.cpp` and any reusable
student-owned helpers they add under `dev/src/`. The assignment directory, grammar files,
test fixtures, comparison scripts, and checked-in reference outputs are support
files, not implementation files to edit for normal solutions. Reuse your earlier compiler infrastructure when implementing this milestone.

Use the supplied reference tools to inspect example output. Tests compare
your compiler with the checked-in contract references.

### Input / Command-Line Arguments

Behaviour is undefined unless the command-line arguments match:

    $ cppgm++ --emit-lowir -O0 -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

`-O0` is the PA22 test mode. Other optimization levels are later optimizer work and
are not required for this milestone.

### Output Format

`cppgm++` shall write LowIR text to `<outfile>`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA22 extends the PA21 lowering
surface only by making more of the C++ source language lower into the already-defined LowIR
family.

LowIR top-level declaration/definition order is a presentation convention, not
a dependency order. Reference outputs and canonical dumps use the order defined
in `../pa8/lowir.md`: `declare global`, `declare function`, `global`, then
`function`, but the relaxed LowIR comparison canonicalizes top-level entries
before comparison. Your output must still be repeatable for the same
inputs; `../pa8/lowir.md` defines the canonical reference presentation and
notes where internal LowIR symbol names are only a presentation tie-breaker.
Your output must also preserve order-sensitive LowIR regions when they are present: instruction order inside
blocks, item order inside structured globals, vtable slot order, and action
order inside generated initialization, finalization, constructor, destructor,
and cleanup bodies.

The generated LowIR must be well-formed and must match the checked-in `.ref` files under
the relaxed LowIR comparison used by the harness. That comparison still checks the
semantic LowIR shape and required IR facts, but it does not make helper metadata
presentation or other non-semantic text details part of the student contract.
The instruction facts and the boundary facts the lowering states (`unwind`,
`inline_hint`, `object_bytes`, `storage=readonly` and the others) are compared
exactly; `../pa8/lowir.md` ("What The Comparison Absorbs And What It
Enforces") lists both sides.

### Error Handling

If an error occurs during preprocessing, tokenization, parsing, semantic analysis, or LowIR
generation, `cppgm++` shall `EXIT_FAILURE`.

The output file is not required to be meaningful on failure.

### Standard Output / Error

Standard output and standard error are ignored for automated testing of
`cppgm++`.

You are free to use them for debugging, tracing, or diagnostic messages.

### Testing

Tests compare your output with the checked-in references using the LowIR
comparison described in [the format guide](../pa8/lowir.md). The `Makefile` invokes
`cppgm++` with `--emit-lowir -O0`.

The local checked-in tests live in `tests/general/`. That directory contains
PA22 source-to-LowIR tests over non-virtual multiple inheritance, multi-base
generated members, member pointers, `dynamic_cast<void*>`, and ambiguity
rejection. `tests/spec/` holds tests anchored to specific clauses, most of
them member pointers in templates; the harness runs them like the general
suite.

For each test case `x`:

- `cppgm++` is executed to produce `x.my`
- the exit status is recorded in `x.my.exit_status`
- `x.my` is compared against `x.ref`
- `x.my.exit_status` is compared against `x.ref.exit_status`

PA22 is tested against generated LowIR text using the relaxed LowIR comparator described
above. For execution feedback, use the supplied `lowir2native-ref -O0`
backend introduced in PA8. Your own native backend is implemented in PA24.

The shipped PA22 tests are the contract for this milestone.

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.t` runs one.

The focused `300-member-pointer-object-extent-boundary.t` control checks
complete-object extents on class references, result storage and indirect
member-function calls, then executes the program. It uses the shared
`check_value_survivor_properties.pl` property checker; no complete LowIR
module is compared. PA12 checks the earlier direct-call boundaries separately.

### PA22 Syntax Spec

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa22.gram`. The grammar defines accepted syntax only;
the PA22 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

As in the earlier assignments, that grammar defines accepted input syntax only. The output
format for `cppgm++` is specified by this README, PA8 `lowir.md`, and the
checked-in `.ref` files.

PA22 does not add a new source-language grammar format. It instead enables more
of the already-accepted C++11 syntax to participate in semantic analysis and
lowering.

A checked-in HTML grammar explorer for that grammar lives in `grammar/`. Treat
`pa22.gram` as the source of truth.

`pa22.gram` uses the same token vocabulary and the same extended BNF operators as
`../shared/source.gram`.

If this README and `pa22.gram` appear to disagree about source syntax, treat `pa22.gram`
as authoritative. If this README and PA8 `lowir.md` appear to disagree about LowIR syntax,
treat `lowir.md` as authoritative. If they disagree about the PA22 lowering slice, treat the
`Assignment Boundary` and `Out Of Scope` sections below as authoritative.

### Assignment Boundary

PA22 supports the following in addition to the PA21 subset:

- non-virtual multiple inheritance
- inherited field lookup and access across multiple non-virtual base subobjects
- inherited non-virtual method lookup and `this` adjustment across multiple non-virtual bases
- constructor, copy-constructor, copy-assignment, and destructor generation across multiple
  non-virtual bases
- member pointer type formation, null values, base-to-derived member-pointer
  conversions, and `.*` / `->*` application for non-virtual class layouts
- `dynamic_cast<void*>` for the existing polymorphic single-inheritance ABI

Within this milestone, PA22 should produce valid LowIR for ordinary source programs over
that subset. That LowIR should be accepted by PA24 `lowir2native` for the supported cases.

To complete PA22, implement these goals:

1. Multiple-base layout and field access.
   Distinct base subobjects should have deterministic offsets, and member access should lower
   through those offsets correctly.  The offsets are the Itanium C++ ABI's (2.4 II): each
   non-virtual base at the data size so far rounded up to its alignment, reusing a
   preceding non-POD base's tail padding; an empty base at offset `0` unless an empty
   subobject of its type is already there, otherwise from the data size on by its
   alignment until no two empty subobjects of one type share an offset.

2. Base-method lookup and `this` adjustment.
   Calling a method inherited from a later base must lower the implicit object argument to the
   correct base-subobject address.

3. Generated special members across multiple bases.
   Synthesized construction, copy, assignment, and destruction should sequence the supported
   non-virtual bases correctly.

4. Member pointer lowering over non-virtual layouts.
   Member pointer values should preserve the selected member target and supported base
   adjustment so `.*` and `->*` lower through the correct object address.
   Their contextual conversion to `bool` must distinguish a non-null target from a null
   member pointer without treating the adjustment word as an independent truth value.
   A data member pointer uses the Itanium C++ ABI representation (2.3), so its values
   cross object boundaries with host-compiled code: the member's byte offset in its
   class (`&S::m` is 0 for a first member), and -1 for null. Null constants,
   value-initialization and zero-initialization of objects that contain data member
   pointers (static storage included) produce -1; a base-to-derived or derived-to-base
   conversion adjusts only a non-null value; comparison with null and contextual
   conversion to `bool` test against -1; `.*` and `->*` add the offset unchanged.
   A member function pointer uses the Itanium representation (2.3) as well: two
   words, `ptr` then `adj`. `ptr` is the function's address or, for a virtual
   function, 1 plus the byte offset of its entry in the vtable, so an odd `ptr`
   marks a virtual function. `adj` is the byte adjustment added to the object's
   address to form `this`. Null is both words zero; a base-to-derived or
   derived-to-base conversion adjusts `adj` of a non-null value only;
   contextual conversion to `bool` tests `ptr`. `.*` and `->*` add `adj` to the
   object's address, then call `ptr` or, when `ptr` is odd, the entry `ptr - 1`
   bytes into the vtable of the adjusted object.
   The call's signature states `unwind=no` when the member pointer's
   declaration is non-throwing, and `object_bytes` on the object only when the
   member pointer's class has no base class, since `adj` may move the object to
   a base subobject (5.5p6). A statically initialized const member pointer, and
   a const array of them, states `storage=readonly`.

5. Remaining single-vptr RTTI case.
   `dynamic_cast<void*>` should lower for the existing polymorphic single-inheritance ABI
   without introducing new LowIR operations. A null operand stays null; otherwise
   load the vtable's offset-to-top entry to obtain the complete-object address.
   Evaluate the operand once and preserve its cv-qualification. This path needs
   neither a class-target runtime search nor a void RTTI object, and should not
   emit an unused runtime-search block.

6. Ambiguity handling.
   Ambiguous inherited member names must not silently resolve.

### Out Of Scope

The following are explicitly out of scope for PA22:

- virtual inheritance
- polymorphic multiple inheritance
- member-pointer behavior that depends on virtual-base or polymorphic
  multiple-inheritance adjustment
- `dynamic_cast` cases that depend on multiple or virtual polymorphic base layouts
- the remaining RTTI cases that require a broader multi-vptr or virtual-base ABI

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stage is PA23, which completes the broader virtual / RTTI ABI that PA22
still deliberately avoids.

So PA22 should leave behind:

- a stable non-virtual multi-base object model over the existing LowIR family
- deterministic lowering for multiple-base field access, method calls, and generated special
  members
- explicit remaining deferrals only where PA23 needs to take over:
  - virtual inheritance
  - polymorphic multiple inheritance
  - the remaining `dynamic_cast` / RTTI cases that depend on that ABI

### Design Notes (Non-Normative)

PA22 should extend the existing semantic and lowering path, not replace it.

The same monotonic-extension rule applies here:

- PA22 should add its new behavior only when the source actually uses the supported PA22
  feature set
- it should not perturb PA21 outputs for programs that remain entirely within the PA21
  subset
- in practice, multiple-base offsets and lowered base-adjustment paths should stay on-demand
  rather than changing earlier single-base outputs unnecessarily
