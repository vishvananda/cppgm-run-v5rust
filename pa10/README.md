## CPPGM Programming Assignment 10 (`cppgm++ --emit-lowir`)

### Overview

PA10 is the point where `cppgm++` gains its first LowIR output mode. The earlier
`--emit-ast`, `--emit-types`, and `--emit-semantics` modes remain required.

The goal of this assignment is to establish the compiler's real backend boundary before the
later object-model and template milestones extend lowering further. PA10 completes the
non-class procedural lowering stage over the current PA7 semantic boundary: it lowers
namespace-scope functions, procedural expressions, control flow, and the supported
scalar/pointer global state into the PA8 LowIR subset.

### Prerequisites

You should complete Programming Assignment 9 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6 declarator/type model
- the PA7 procedural semantic analysis as the source of truth for resolved functions,
  locals, and expressions
- the PA8 LowIR contract
- the PA9 typed ABI-name model and encoder

The intended direction is:

- PA5 provides syntax
- PA6 provides scopes and types
- PA7 resolves procedural expressions and calls
- PA8 defines the backend boundary and runnable validation scaffold
- PA9 provides ABI names from typed semantic facts
- PA10 lowers the resolved procedural subset into LowIR

### Starter Kit

The starter kit contains:

- the student-editable `../dev/cppgm++.cpp` entry point, initially seeded from the course
  `cppgm++` scaffold and reached from this directory through the `cppgm++.cpp` symlink
- shared `../dev/` and `../dev/src/` support code from the earlier compiler pipeline
- the grammar for this assignment called `pa10.gram`
- an HTML grammar explorer of `pa10.gram` in the sub-directory `grammar/`
- checked-in golden output files under `tests/`
- a checked-in local test suite under `tests/`

Extend the driver and frontend you implemented in earlier assignments with
the PA10 lowering behavior.

Use the supplied reference tools to inspect example output. Tests compare
your compiler with the checked-in contract references.

### Driver Surface For This Assignment

Previously required:

- `--emit-ast`
- `--emit-types`
- `--emit-semantics`
- `-o <outfile>`

New in PA10:

- `--emit-lowir`
- `-O0` as the unoptimized LowIR test mode

No practical compile/link driver flags are introduced here yet. That later
surface starts in PA25.

### Input / Command-Line Arguments

The same as PA7 `cppgm++ --emit-semantics`, with the new LowIR emit mode. The PA10 test mode is unoptimized LowIR generation. `make test` passes `--emit-lowir -O0`
through the harness, so individual test files do not spell those flags themselves.

Behaviour is undefined unless the command-line arguments match:

    $ cppgm++ --emit-lowir -O0 -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

with the same relaxations as PA7.

Accepting `--emit-lowir` without an explicit `-O0` as the same unoptimized mode is fine,
but optimized LowIR output is not part of PA10.

### Output Format

`cppgm++` shall write LowIR text to `<outfile>`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA10 only needs the procedural
subset of that IR, but it must emit valid PA8 LowIR.

Example, for `int main() { return 0; }`:

    function @main() -> i32 [role=entry] {
      block ^entry:
        return i32 0
    }

PA10 writes a single concatenated LowIR program consisting of:

- zero or more `global` definitions
- zero or more `function` definitions

LowIR top-level declaration/definition order is a presentation convention, not
a dependency order. Reference outputs and canonical dumps use the order defined
in `../pa8/lowir.md`: `declare global`, `declare function`, `global`, then
`function`, but the relaxed LowIR comparison canonicalizes top-level entries
before comparison. Your output must still be repeatable for the same
inputs; `../pa8/lowir.md` defines the canonical reference presentation and
notes where internal LowIR symbol names are only a presentation tie-breaker.
Externally meaningful C++ symbols carry their Itanium ABI names (PA9's
rules): what `abimangle` produces for the same facts. Reusing PA9's encoder
from resolved declarations and types is the direct way; converting semantic
state to fact text and parsing it back is a slow path to avoid. One compact
identity per top-level symbol, with its spelling kept once and rendered only
when LowIR text or a diagnostic is written, keeps symbol handling cheap; no
test observes it.

Your output must also preserve order-sensitive LowIR regions when they are present: instruction order inside
blocks, item order inside structured globals, vtable slot order, and action
order inside generated initialization, finalization, constructor, destructor,
and cleanup bodies.

PA10 is still a purely procedural lowering stage. Its LowIR output should not include
class/object-model helper definitions such as synthesized constructors, destructors, copy
helpers, or class-lifetime startup/shutdown hooks. Tests that require those belong in PA11
or later.

The checked-in `.ref` files define the required LowIR facts for the tests. The
test harness checks exit status, LowIR well-formedness, and the
course-defined normalized LowIR output rather than requiring students to match every
non-semantic helper spelling or presentation choice. What that normalization
absorbs, and what it does not, is one list in `../pa8/lowir.md` ("What The
Comparison Absorbs And What It Enforces"). Read it before the first failing
fixture. In short, the comparison renames temporaries, slots, blocks and
parameters, ignores the order of slot declarations and of top-level entries,
reads a literal by its value, and lets the operands of a commutative
operation appear in either order; every instruction of a body must
otherwise match the reference's, in its order. It enforces the conventions
the course fixes in words rather than absorbing, chief among them:

- Branch sense follows the source: a conditional branch tests the value the
  source wrote, in the source's sense (`!=` is `cmp ne`, `!x` is
  `cmp eq x, 0`, a bare scalar is branched on directly), and the first
  target is the source's true path.
- A retype is a `copy`: a conversion that keeps the bits and only changes
  the LowIR type is written `copy <type> <value>`, not omitted and not
  written as `convert`.
- Instructions follow the source's evaluation order: the right operand of an
  assignment before the address of its left, and where the language leaves
  the order open, left to right (operands, call arguments, and the loads
  each needs).
- Instruction facts are part of the instruction (`../pa8/lowir.md`,
  "Instruction Facts"): every non-volatile load and store of a pointer, a
  floating type or an integer wider than a byte is `typed` unless it goes
  through a union member or a bit-field's storage unit; the `add`, `sub` and
  `mul` of signed `int` and `long` source arithmetic (and of `++`, `--` and
  the compound assignments) are `nowrap`; and the division of a pointer
  difference by its element size is `exact`.
- The boundary facts are part of the output (`../pa8/lowir.md`, "Facts The
  C++ Lowering States"): a function whose exception-specification allows no
  exception (`noexcept`, `noexcept(true)`, `throw()`, or `noexcept(e)` with
  `e` true) states `unwind=no`; a statically initialized global of a const,
  non-volatile type that is not `thread_local` states `storage=readonly`;
  and a reference parameter to a complete object type states
  `object_bytes=N`, `N` the size of the object it refers to.

A fixture that fails on one of these is telling you which convention
your output departs from; the canonical diff the harness writes beside the
output shows where.

The facts are what later stages may rely on: PA32's optimizer can assume a
typed store does not change a typed load of another type, remove the
exception region around a call that cannot unwind, fold a load from readonly
data to its value, and load through a reference before a loop that may not
run; PA24's backend can skip renormalizing a result that does not wrap.
Stating them now costs one flag each; deriving them again after lowering is
not possible.

For reference initialization, the course includes the later
[CWG2352 resolution](https://cplusplus.github.io/CWG/issues/2352.html):
reference-related types include similar types, and a permitted qualification
conversion can make them reference-compatible. Such a compatible lvalue binds
directly; it does not require a converted pointer temporary. In case 2 of
`tests/general/100-reference-cv-qualified-lvalue-boundaries.t`,
`const int* const* const& r = pp` therefore makes `r` denote `pp` itself.
This is an explicit compatibility rule beyond literal N3485 8.5.3, not a
requirement to implement every later language change.

For supported scalar conversions, PA10 may canonicalize widened integral immediates directly
to their final LowIR literal value instead of spelling those same conversions through
intermediate `binary shl` / `binary shr` sign-extension shells.

For built-in `&&` / `||` used directly as statement conditions (`if`, `while`, `do`, `for`),
the expected LowIR shape is direct short-circuit control flow. In that condition context,
the compiler should branch through the operand blocks rather than first materializing a
separate `land__*` / `lor__*` boolean slot.

The generated LowIR is intended to become input for the later PA24
`lowir2native` backend. That future native path is not the PA10 grading
contract, but PA10 should avoid emitting LowIR that only works for this one
text comparison.

For execution feedback while implementing lowering, feed the generated
LowIR to the supplied `lowir2native-ref -O0`. This uses the same native
reference introduced in PA8, before your own backend is implemented.
The output contract for this assignment remains the required LowIR facts.

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
  LowIR comparison (`../pa8/lowir.md`, "What The Comparison Absorbs And What
  It Enforces", lists exactly what that comparison ignores and what it holds
  you to)
- `x.my.exit_status` is compared against `x.ref.exit_status`

`make test` runs the checked-in local suite under `tests/` and supplies
`--emit-lowir -O0` through the harness.

The PA10 test suite uses:

- `tests/general/`: the default PA10 LowIR oracle suite. These tests cover the procedural
  lowering contract and integration cases that are validated by generated LowIR
  text and exit status. The covered source features are namespace functions and
  globals, procedural statements, condition declarations, scalar expressions,
  references, arrays, pointer operations, enums, built-in casts, and resolved
  calls over the PA7 semantic subset.

PA10 is tested against the generated LowIR text.

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.t` runs one.
Controls that state no outcome are focused property checks run by this
assignment's own checkers.

### PA10 Syntax Spec

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa10.gram`. The grammar defines accepted syntax only;
the PA10 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

As in the earlier assignments, that grammar defines accepted input syntax only. The output
format for `cppgm++` is specified by this README, PA8 `lowir.md`, and the checked-in
`.ref` files.

Because PA10 is a code-generation assignment layered directly on PA5-PA7, the
grammar keeps parser/AST behavior stable while the `Assignment Boundary` below
defines which already-parsed constructs PA10 must analyze and lower.

Passing PA7 is necessary but not sufficient for passing PA10: an input may be syntactically
valid for PA5 and semantically valid for PA7 and still be outside the PA10 code-generation
subset described below.

A checked-in HTML grammar explorer for that grammar lives in `grammar/`. Treat
`pa10.gram` as the source of truth.

`pa10.gram` uses the same token vocabulary and the same extended BNF operators as
`../shared/source.gram`.

If this README and `pa10.gram` appear to disagree about source syntax, treat `pa10.gram`
as authoritative. If this README and PA8 `lowir.md` appear to disagree about LowIR syntax,
treat `lowir.md` as authoritative. If they disagree about the required PA10 lowering slice,
treat the `Assignment Boundary` and `Out Of Scope` sections below as authoritative.

### Assignment Boundary

This PA10 milestone supports the following:

- namespace-scope function definitions and declarations in a single generated program,
  including named namespaces, C language linkage, and deduplication of repeated
  compatible declarations
- a required `main` definition
- functions returning integral, pointer, or `bool` results from the supported PA7 subset
- up to five parameters in the supported PA7 procedural type subset
- global integral/pointer/function-pointer objects with constant initializers or zero-init,
  including object addresses and constant array-element addresses
- narrow string literals used as pointer values, such as call arguments and
  pointer or pointer-array initializers: each literal's backing array is an
  internal structured global with `storage=readonly`
- `storage=readonly` on every statically initialized global definition of a
  const, non-volatile type that is not `thread_local`: a scalar, a pointer,
  or an array of them (an array's qualifiers are its element type's,
  3.9.3p5); a volatile, thread-local, dynamically initialized or non-const
  object states none, and PA11 extends the rule to class objects
- volatile scalar lvalue-to-rvalue conversions and stores represented by
  `load volatile` and `store volatile` at the LowIR boundary, including local,
  pointer-indirect, and class-member access; the marker belongs to the access,
  while an adjacent nonvolatile member access remains ordinary
- recognized memory builtins use ordinary pointer parameters and preserve the
  function-level runtime identity and effects needed by later stages: their
  declarations carry `object=cppgm_builtin_memcpy` and
  `object=cppgm_builtin_memmove` with `unwind=no` (`../pa8/lowir.md`, "Facts
  The C++ Lowering States"); non-overlapping `memcpy` boundaries carry
  `alias=noalias`, while potentially overlapping `memmove` boundaries do not
- block-scope function declarations and `extern` variable declarations, which
  name members of the innermost enclosing namespace (3.5p6) and have no
  automatic storage
- local scalar objects, scalar/function references, function pointers/references, and bounded
  arrays in the supported PA7 procedural type subset; an omitted array bound is inferred
  from its initializer, missing elements are zero-initialized, and excess elements are
  rejected; an `extern` array of unknown bound may be referenced without requiring its
  layout in the current translation unit
- automatic nonvolatile scalar arrays with completely known literal initializers
  use one copy from readonly constant data, including omitted zero elements;
  mutable arrays still have distinct automatic storage. PA16 extends eligibility
  to initializers evaluated through general constexpr expressions
- expression statements
- `return`
- `if` / `else`
- condition declarations in `if` and `switch`, including the lifetime of the
  condition-scope binding
- `switch`
- `while`
- `do`
- `for`
- `break` / `continue`
- direct calls to resolved non-template namespace-scope functions, including supported
  default arguments resolved in the declaration context where the default was introduced
- calls through function pointers and function references in the PA7 subset
- lvalue references, including reference parameters, reference locals, reference
  returns, and aliasing through supported calls
- reference parameters use LowIR's shared `ptr [pass=by_address]` boundary:
  callers preserve the required addressable-storage behavior without retaining
  a separate source-reference passing label
- array-to-pointer decay, subscript expressions, pointer arithmetic, one-past
  pointer values, pointer compound assignment with element-size scaling, and
  pointer differences measured in elements; because the byte difference of
  two pointers into the same array is exactly divisible by the element size,
  a positive power-of-two size is lowered as an arithmetic right shift,
  while other element sizes retain signed division; either way the division
  is `exact`
- array-to-pointer and function-to-pointer decay produce an ordinary LowIR
  `ptr` using the existing address, index, parameter, or `copy ptr` operations;
  do not add a decay-specific unary operation or parameter-passing annotation
- scoped and unscoped enums, enum constants, enum promotion/comparison, and
  enum lowering
- built-in casts over the supported scalar, function, reference, and pointer
  types, including C-style casts, `static_cast`, and `const_cast`
- source-to-LowIR floating scalar literals and conversions among supported
  scalar types, including float/integer conversions needed for calls, returns,
  comparisons, and branch conditions
- C-style variadic function calls over supported scalar arguments, including
  source-to-LowIR default argument promotion before the call
- expressions:
  - integer literals, floating literals, and `true` / `false`
  - id-expressions naming supported locals, globals, and resolved functions
  - `sizeof(expr)` and `sizeof(type-id)` when PA7 has resolved them
  - unary `+`, `-`, `!`, `~`, `&`, `*`, prefix `++`, and prefix `--`
  - postfix `++` and postfix `--`
  - simple assignment to supported lvalues
  - built-in arithmetic, bitwise, shift, logical, comparison, conditional, comma, and
    subscript forms from the PA7 procedural subset

As required by PA8, every LowIR `cmp` instruction produces an `i64` truth
value. When a comparison or logical expression must be materialized as the
course `bool` representation (`u8`) for storage, an argument, or a return,
emit an explicit conversion from that `i64` result. A branch may consume the
canonical comparison result directly.

Compiler-generated slots and helper names must remain distinct from source
identifiers so a source declaration cannot redirect an internal temporary.
(Carrying compact value, slot, block and symbol identities into the typed
LowIR model, with each spelling kept once and generated temporaries numbered,
avoids building or hashing a presentation string per operand; no test
observes it.)

The generated LowIR for this supported subset is intended to be accepted by the
later PA24 `lowir2native` backend.
### Out Of Scope

The following are explicitly out of scope for this PA10 milestone:

- object initialization backed by a string literal, such as a character array
  initialized from one
- global or local initialization forms that require a richer constant-evaluation or aggregate
  initialization layer than PA7 currently provides
- function-local static objects and guard variables
- class/object semantics
- synthesized class helper output of any kind
- template code generation
- exception-aware control flow
- fully general shadowing-sensitive lowering of same-name local bindings
- native backend/runtime parity for floating-point conversions and variadic promotions
- hosted or vendor integer extensions such as 128-bit integer types

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stages are:

- PA11: extend this procedural lowering path into the basic non-virtual object model:
  object layout, methods, constructors/destructors, lifetime, and single inheritance
- PA12: build on that PA11 object model with non-polymorphic value semantics:
  copy construction/assignment, pass-by-value, return-by-value, and the common
  user-defined operator paths needed by value types
- PA13: add the polymorphic machinery on top of the PA11/PA12 class model:
  virtual dispatch, vtables, and virtual destructors
- later template-aware assignments: reuse the same procedural lowering path for instantiated
  template code once template semantics exist

So PA10 should leave behind a reusable procedural `C++ -> LowIR` lowering path rather than
trying to absorb class or template semantics early.

### Design Notes (Non-Normative)

The cleanest reuse path is to keep PA7 as the semantic source of truth and lower from that
resolved procedural representation rather than rebuilding expression semantics again inside
PA10.

Useful intermediate representations include:

- a resolved procedural tree shared with PA7
- explicit object identities for globals, locals, references, arrays, and
  function objects
- explicit local slot/layout information
- a centralized type-to-LowIR lowering and conversion layer
- a stable mapping from resolved expressions to LowIR values and stack locations

It is useful for the lowering layer to derive result types from the LowIR
operation as it creates a temporary. In particular, keeping the canonical
`i64` comparison result there avoids duplicating result-type decisions at
each later use.

The LowIR this assignment emits is the optimizer's input at PA32, which is
why its shape is simple rather than efficient. Every local lives in a slot
and every use loads it: PA32 promotes slots to values, so there is no need to
avoid the loads here. `unreachable` states a fact the optimizer uses to
delete code after a call that does not return; it is not a trap to execute.
The instruction facts (Instruction Facts in `../pa8/lowir.md`) are the
semantic facts the optimizer cannot recover from LowIR alone: C++'s type-based
aliasing and its undefined signed overflow. State them where the rules say,
even though nothing before PA32 reads them.

Lower each function straight into the PA8 model, and decide by demand from
the start which definitions to lower. A definition the translation unit need
not emit, such as an inline function, an implicit member or a template
instantiation, is lowered only when something already emitted refers to it.
Later assignments bring such definitions in from headers by the thousand,
and most are never used; lowering them all and discarding the unused ones
costs time on every compile.
