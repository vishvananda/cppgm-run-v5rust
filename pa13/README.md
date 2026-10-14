## CPPGM Programming Assignment 13 (`cppgm++ --emit-lowir`)

### Overview

PA13 adds the first polymorphic object-model layer on top of the completed PA12
non-polymorphic value-semantics compiler. It extends PA12 with:

- virtual functions
- vpointers and vtables
- dynamic dispatch through ordinary member calls
- virtual destructors
- `override` / `final` checking for the supported virtual subset

### Prerequisites

You should complete Programming Assignment 12 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6 declarator/type model
- the PA7 call-resolution layer
- the PA10-PA12 LowIR lowering path
- the PA8 LowIR contract
- the PA11-PA12 class metadata, constructor/destructor machinery, and lifetime lowering

The intended direction is:

- PA5 provides syntax
- PA6 provides scope/type lookup
- PA7 provides the procedural expression/call core
- PA10 lowers the procedural subset
- PA11 adds the basic non-virtual object model
- PA12 adds the non-polymorphic value-semantics layer
- PA13 extends that same object model with scoped polymorphism

### Starter Kit

The starter kit contains:

- the student-editable `../dev/cppgm++.cpp` entry point, initially seeded from the course
  `cppgm++` scaffold and reached from this directory through the `cppgm++.cpp` symlink
- shared `../dev/` and `../dev/src/` support code from the earlier compiler pipeline
- the grammar for this assignment called `pa13.gram`
- an HTML grammar explorer of `pa13.gram` in the sub-directory `grammar/`
- a checked-in local test suite under `tests/`

Extend the driver and frontend you implemented in earlier assignments with
the PA13 lowering behavior.

Use the supplied reference tools to inspect example output. Tests compare
your compiler with the checked-in contract references.

### Input / Command-Line Arguments

The same as PA12 `cppgm++ --emit-lowir`. The PA13 test mode is unoptimized LowIR
generation. `make test` passes `--emit-lowir -O0` through the harness, so individual test
files do not spell those flags themselves.

Behaviour is undefined unless the command-line arguments match:

    $ cppgm++ --emit-lowir -O0 -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

with the same relaxations as PA12.

Accepting `--emit-lowir` without an explicit `-O0` as the same unoptimized mode is fine,
but optimized LowIR output is not part of PA13.

### Output Format

`cppgm++` shall write LowIR text to `<outfile>`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA13 extends the PA12
object/value-semantics subset of that IR with the polymorphic lowering needed by this
milestone.

PA13 writes a single concatenated LowIR program consisting of:

- zero or more `global` definitions
- zero or more `function` definitions

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

For supported polymorphic classes, PA13 extends the PA12 lowering convention by introducing:

- emitted LowIR global entries that represent vtable slots
- explicit vpointer stores in constructors and destructors
- indirect LowIR calls for supported virtual dispatch sites

The contents of each vtable global are order-sensitive. Vtable slots, including
the complete-then-deleting virtual destructor slot pair, are part of the LowIR
contract in `../pa8/lowir.md` even though the top-level position of the vtable
global itself is presentation.

Vtables, the RTTI objects they point to and those objects' names are
`storage=readonly` data (`../pa8/lowir.md`, "Facts The C++ Lowering States").
A virtual call's signature states the called function's boundary facts:
`unwind=no` and `return=noreturn` by its declaration, and `object_bytes` on
`this` by its class. The virtual call of the deleting destructor that
`delete p` makes states the destructor's, `unwind=no` when the destructor is
non-throwing (12.4p3).

The checked-in `.ref` files define the required LowIR facts for the tests. The
test harness checks exit status, LowIR well-formedness, and the
course-defined normalized LowIR output rather than requiring students to match every
non-semantic helper spelling or presentation choice. The normalization renames
temporaries, slots, blocks and parameters, ignores the order of slot
declarations and of top-level entries, and reads a literal by its value;
every instruction of a body must otherwise match the reference's, in its
order. `../pa8/lowir.md` ("What The Comparison Absorbs And What It Enforces")
is the one list of what it absorbs and of the conventions it enforces.

### Error Handling

If an error occurs during preprocessing, tokenization, parsing, semantic analysis, or LowIR
generation, `cppgm++` shall `EXIT_FAILURE`.

The output file is not required to be meaningful on failure.

### Standard Output / Error

Standard output and standard error are ignored for automated testing of `cppgm++`.

You are free to use them for debugging, tracing, or diagnostic messages.

### Testing

Tests compare your output with the checked-in references using the LowIR
comparison described in [the format guide](../pa8/lowir.md).

For each test case `x`:

- `cppgm++` is executed to produce `x.my`
- the exit status is recorded in `x.my.exit_status`
- `x.my` is validated as LowIR and compared against `x.ref` using the normalized
  LowIR comparison
- `x.my.exit_status` is compared against `x.ref.exit_status`

`make test` runs the checked-in local suite under `tests/` and supplies
`--emit-lowir -O0` through the harness.

The PA13 suite is split by test role:

- `tests/general/`: the default PA13 LowIR oracle suite. These tests cover polymorphic
  lowering, vtable/vpointer emission, multi-feature cases, and support-fixture
  cases whose primary contract is generated LowIR plus exit status.
- `tests/spec/`: focused C++ language-contract cases that cite a specific N3485 clause.
  Each source test in this directory starts with a comment of the form:

    // N3485 focus: <clause> [<stable-name>] <short topic>

`tests/spec/` covers virtual dispatch, virtual destructor overriding,
`override` / `final`, pure virtual declarations, covariant returns, and
explicit qualification suppressing virtual dispatch. `tests/general/` covers
polymorphic and LowIR-shape cases that are not tied to one specific C++11
clause.

PA13 is tested against the generated LowIR text.

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.t` runs one.

### PA13 Syntax Spec

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa13.gram`. The grammar defines accepted syntax only;
the PA13 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

As in the earlier assignments, that grammar defines accepted input syntax only. The output
format for `cppgm++` is specified by this README, PA8 `lowir.md`, and the checked-in
`.ref` files.

The virtual syntax used here was already preserved by PA5; PA13 is the first
milestone that gives it code-generation meaning.

Passing PA12 is necessary but not sufficient for passing PA13: an input may be syntactically
valid for PA5-PA12 and code-generation-valid for PA12 and still be outside the PA13
polymorphism slice described below.

A checked-in HTML grammar explorer for that grammar lives in `grammar/`. Treat
`pa13.gram` as the source of truth.

`pa13.gram` uses the same token vocabulary and the same extended BNF operators as
`../shared/source.gram`.

If this README and `pa13.gram` appear to disagree about source syntax, treat `pa13.gram`
as authoritative. If this README and PA8 `lowir.md` appear to disagree about LowIR syntax,
treat `lowir.md` as authoritative. If they disagree about the PA13 lowering slice, treat the
`Assignment Boundary` and `Out Of Scope` sections below as authoritative.

### Assignment Boundary

PA13 supports the following in addition to the PA12 subset:

- polymorphic root classes with one vpointer at offset `0`
- derived classes whose direct base is already polymorphic and therefore already carries the
  shared vpointer at offset `0`
- derived classes with non-polymorphic direct bases that introduce the first supported
  vpointer at offset `0`; ordinary pointer/reference conversion to such a base uses the
  resulting nonzero base-subobject offset and preserves its data members
- virtual member functions in the ordinary non-template class cases
- overriding of inherited virtual members by exact signature match in the current class model
- covariant pointer/reference return overrides when the class hierarchy is in
  the supported single-inheritance subset
- `override` checking for the supported virtual subset
- method-level `final` checking for the supported virtual subset
- an override's exception specification allows only exceptions permitted by
  the overridden declaration, including a narrower intermediate override;
  non-throwing overrides may use either `throw()` or `noexcept`
- pure virtual declarations and pure-virtual vtable entries
- dynamic dispatch for ordinary member calls through:
  - object expressions of polymorphic class type
  - pointers to polymorphic class type
  - references to polymorphic class type
- explicit base qualification suppressing virtual dispatch for supported calls
- virtual destructors as part of the supported virtual set
- virtual `delete` over that supported set, including the deleting-destructor
  entry and selection of a PA12-supported class-specific deallocation function
- emitted vtable data for supported polymorphic classes
- constructor/destructor vpointer writes for supported polymorphic classes;
  an explicitly defaulted default constructor of such a class is not trivial
  (12.1p5) and installs the vpointer like an implicit one
- deterministic vtable slot order, including declaration order for ordinary virtual
  functions and the destructor slot order used by the checked references
- deterministic emitted destructor-entry order for each class: base entry,
  deleting entry, then complete entry, even when later cleanup sharing changes
  which entry is demanded first (like all top-level order, a presentation
  convention that the relaxed comparison does not grade)

Within this milestone, PA13 should produce valid LowIR for ordinary single-inheritance
polymorphic code over the supported PA12 subset. That LowIR is intended to be
accepted by the later PA24 `lowir2native` backend for the supported cases.

### Out Of Scope

The following are explicitly out of scope for PA13:

- multiple inheritance
- virtual inheritance
- RTTI and `dynamic_cast`, except the typeinfo objects (`_ZTI`, `_ZTS`)
  that each polymorphic class's vtable points to, which are required here
  with the vtable header that holds that pointer and the offset-to-top
- pointer-adjusting thunks or any ABI that requires base-subobject pointer adjustment
- class-level `final`
- full abstract-class enforcement beyond the pure-declaration/vtable cases above
- generalized exception-aware virtual cleanup beyond the deleting-destructor
  and deallocation path pinned by the checked references
- generalized operator overloading beyond the supported PA12 value-semantics paths
- template-aware virtual dispatch

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stage is PA14, which adds the first usable template layer on top of the
completed procedural/object/polymorphic compiler:

- function templates
- class templates
- template argument deduction
- basic instantiation

So PA13 should leave behind a clean single-inheritance polymorphic LowIR lowering path
rather than mixing templates into the same milestone.

### Design Notes (Non-Normative)

The important point is to extend the existing PA11/PA12 object-model behavior rather than
inventing a second, incompatible model just for polymorphism. Whether that reuse happens
through shared code, shared data structures, or a careful reimplementation is up to you.

The same monotonic-extension rule applies here:

- PA13 should add polymorphic behavior only for programs that actually use the supported
  virtual feature set
- it should not change PA12 outputs for programs that remain entirely within the PA12 subset
- in practice, vtables, vpointer writes, and virtual-call lowering should be driven by the
  presence of supported virtual members and polymorphic classes, not enabled eagerly for all
  class code

Useful intermediate representations include:

- class metadata that distinguishes ordinary methods, constructors, destructors, and
  virtual slots
- explicit vtable/vpointer metadata separate from the source syntax tree
- vtable layout derived deterministically from semantic class metadata rather
  than source-text scans
- explicit constructor/destructor/vpointer actions attached to the lowered function bodies
- lifecycle-entry grouping by semantic class and entry kind so final function
  order does not depend on the order in which call sites demand those entries
- a direct-call vs. virtual-call distinction in the semantic IR so codegen does not have to
  rediscover polymorphism from source syntax

A virtual call is an ordinary load of the vpointer, a load from the vtable
and an indirect call. Keep it that way, with vtables as `storage=readonly`
structured data and vpointer writes as ordinary stores in the constructors.
Then PA32 needs no virtual-call machinery to turn a call on an object of known
type into a direct one: once the constructor is inlined, memory forwarding
knows the vpointer, a load from readonly data at a constant offset yields the
function's address, and an indirect call through an `addr` becomes direct.
