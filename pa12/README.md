## CPPGM Programming Assignment 12 (`cppgm++ --emit-lowir`)

### Overview

PA12 finishes the non-polymorphic class model so ordinary user-defined value types work
cleanly before virtual dispatch is added. It extends PA11 with the common value-semantics
paths:

- copy construction/assignment and the common move-construction/move-assignment cases
  needed by those same value paths
- pass-by-value and return-by-value of class objects
- temporary materialization in the common call/return/initialization paths
- delegating constructors
- out-of-class constructor and destructor definitions
- construction and destruction of base subobjects using definitions that appear
  later in the translation unit
- the ordinary user-defined copy/move constructors and assignment operators directly
  needed by that value-semantics work

### Prerequisites

You should complete Programming Assignment 11 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6 declarator/type model
- the PA7 call-resolution layer
- the PA10/PA11 LowIR lowering path
- the PA8 LowIR contract
- the PA11 class metadata, constructor/destructor machinery, and lifetime lowering

The intended direction is:

- PA5 provides syntax
- PA6 provides scope/type lookup
- PA7 provides the procedural expression/call core
- PA10 lowers the procedural subset
- PA11 adds the basic non-virtual object model
- PA12 extends that same object model into usable value semantics

### Starter Kit

The starter kit contains:

- the student-editable `../dev/cppgm++.cpp` entry point, initially seeded from the course
  `cppgm++` scaffold and reached from this directory through the `cppgm++.cpp` symlink
- shared `../dev/` and `../dev/src/` support code from the earlier compiler pipeline
- the grammar for this assignment called `pa12.gram`
- an HTML grammar explorer of `pa12.gram` in the sub-directory `grammar/`
- a checked-in local test suite under `tests/`

Extend the driver and frontend you implemented in earlier assignments with
the PA12 lowering behavior.

Use the supplied reference tools to inspect example output. Tests compare
your compiler with the checked-in contract references.

### Input / Command-Line Arguments

The same as PA11 `cppgm++ --emit-lowir`. The PA12 test mode is unoptimized LowIR
generation. `make test` passes `--emit-lowir -O0` through the harness, so individual test
files do not spell those flags themselves.

Behaviour is undefined unless the command-line arguments match:

    $ cppgm++ --emit-lowir -O0 -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

with the same relaxations as PA11.

Accepting `--emit-lowir` without an explicit `-O0` as the same unoptimized mode is fine,
but optimized LowIR output is not part of PA12.

### Output Format

`cppgm++` shall write LowIR text to `<outfile>`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA12 extends the PA11 object-model
subset of that IR with the value-semantics lowering needed by this milestone.

PA12 writes a single concatenated LowIR program consisting of:

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

For supported class value types, PA12 extends the PA11 lowering convention by introducing:

- indirect LowIR parameters for pass-by-value class objects
- indirect LowIR return destinations for return-by-value class objects
- explicit LowIR-level materialization of supported copy/move/value transfers

A class uses those indirect boundaries when it is non-trivial for the purposes
of calls (Itanium C++ ABI 3.1.2.3): it has a non-trivial copy or move
constructor that is not deleted, a non-trivial destructor, or only deleted copy
and move constructors. Base classes, size and template spelling do not change
that choice. The callee's parameter, or the caller's result object, is then the
object the selected constructor initialized (5.2.2p4, 8.5p15). Other class
values may cross the boundary directly as `obj<NxA>`; a result larger than 16
bytes still uses caller storage. The caller owns a by-value class argument
whose destructor is non-trivial: it is a temporary of the full-expression
containing the call, destroyed there after the callee returns, and the callee
never destroys its parameter.

Every pointer boundary that denotes a complete object also carries the PA8
`object_bytes=N` extent in emitted O0 LowIR. This includes the implicit object
parameter of supported member functions, class references and indirect class
arguments, and indirect result storage. An ordinary source pointer does not
gain an extent merely from its pointee type. The extent says that `N` bytes
are readable when the function is entered; it is semantic LowIR metadata,
compared like the other boundary facts, and must survive text and
compiler-object replay; it is not a native calling-convention annotation.

Synthesized copy/move constructors, assignment helpers, and related
temporary-materialization support are part of the PA12 semantic model, but `cppgm++` only
needs to emit the helper definitions that the lowered program actually requires. Unused
copy/move/value helpers do not need to appear just because they are synthesizable.

The indirect destination and source parameters of a same-class copy or move
constructor denote distinct live objects during construction, so the emitted
LowIR marks both `alias=noalias`, in every entry of the constructor and in a
copy or move constructor with further defaulted parameters too (12.8p2). A
constructor template, a converting constructor and an assignment operator,
whose operands may be one object, state none.

Focused course controls (the controls in `tests/controls/` that state no
outcome, checked by `../scripts/check_pa12_survivor_properties.pl`) validate
these boundary facts and object identity and lifetime relationships without
comparing a complete LowIR module: the two constructor parameters versus an
assignment operator's, the ordering and identity of construction, selected
use and destruction within one full expression, a parameter object's extent
and identity, the size, cookie and length check of an array new, and a named
return object. Most also compile and execute the program. These controls run
your `cppgm++ --emit-lowir -O0` frontend, then use the supplied
`lowir2native-ref` backend to execute its output. Native code emission and
the compile/link driver remain PA24 and PA25 work.

When a same-type conditional class prvalue is materialized in a private
temporary and then selected for copy or move construction into its final
complete-object destination, PA12 records the standard source-language
permission on that outer direct call as `[elision=copy]`, by the rule of
`../pa8/lowir.md`: a copy-initialization from a prvalue conditional of the
object's own class, never a direct-initialization, a `return` operand or a
derived prvalue sliced into a base; the comparison checks it.  This is serialized
optimization information, not an O0 elision: the emitted O0 LowIR retains the
distinct temporary, transfer call, and normal/exceptional destruction.  The
focused control checks those relationships and executes the O0 behavior
without prescribing complete generated LowIR text.

For supported indirect return-by-value cases, PA12 lowers an eligible top-level named
local directly in `%ret` instead of building a separate local object and then copying or
moving it into the return destination. The references use that direct return-slot form,
and the comparison does not absorb the alternative (`../pa8/lowir.md`, "What The
Comparison Absorbs And What It Enforces").

Ref-qualified member functions extend the PA11 member-call model: overload resolution still
uses the implicit object argument, and the object expression's value category participates in
viability and ranking for supported `&` and `&&` qualified members.

The ABI identity of a member function is built from its declared source parameters; the
implicit object used by LowIR lowering is not part of that declared parameter list. In
particular, an out-of-class move-assignment definition retains its rvalue-reference parameter
in the ABI identity.

Temporaries and by-value arguments whose destructors are not trivial are
destroyed on the exceptional path as well as the normal one, inside cleanup
regions. The course solution places those regions by the shape of the
full-expression, not by whether its calls can throw, so the references have
regions around calls that cannot throw, and the comparison checks them like
any other region. A temporary's destructor is called even when its body is
empty: PA11's exception for a destructor with no effect covers only a single
automatic object and a base or member destroyed by another destructor.

For supported synthesized copy/move special members, PA12 lowers a leading trivially
copyable storage prefix directly as `copyobj <span> <src>, <dst>` instead of spelling that
prefix as separate field operations or a `__builtin_memcpy` helper call in the emitted
LowIR. The references use that direct storage-copy form, and the comparison does not
absorb the alternative.

When a synthesized copy/move constructor or assignment body handles adjacent
nonvolatile bit-fields in one supported 8-, 16-, 32-, or 64-bit allocation
unit, it shall transfer that allocation unit once.  A zero-width bit-field or
a change of storage offset or width starts a new unit.  Fields whose layout
cannot be transferred safely shall retain field-wise value semantics.

A base subobject owns only its class's non-virtual size: the rest of the
class's size can hold a derived class's member or virtual base (PA11 layout,
Itanium C++ ABI 2.4).  A class value transfer -- a trivial copy or move, a
synthesized storage copy, or a bit-field allocation unit -- therefore covers
at most that many bytes (`copyobj 1x4` for a 4-byte class whose data ends in
its first byte), and value-initializing a base subobject zeroes only those
bytes.  A mem-initializer `Base()` value-initializes a base whose default
constructor is not user-provided -- implicit, or defaulted on its first
declaration -- so the base is zeroed before any constructor runs.  A base
initialized from a prvalue of its own class (an elided copy) or, if it is an
aggregate, from a braced list takes that value; where the class's size
exceeds its nvsize the value is built in a temporary and its nvsize bytes
copied, since a derived class's member or virtual base may already sit past
them.  A by-value parameter of a base class given a derived object is
constructed from that object: the constructor taking the base binds to the
object's base subobject with the object's value category, so an lvalue is
copied and a prvalue moved.

For supported trivially copy-constructible class value transfers, PA12 lowers the
copy/move construction step itself directly as `copyobj <span> <src>, <dst>` instead of
spelling a call to a synthesized trivial copy/move constructor helper. The references use
that direct value-transfer form, and the comparison does not absorb the alternative.

A synthesized constructor, destructor or copy/move assignment operator, and
one defaulted on its first declaration, states `unwind=no` when every function
it directly invokes is non-throwing (15.4p14); the comparison checks it.

A class value that crosses a boundary directly as `obj<NxA>` states its
eightbytes' classes, `classes` on the parameter and `result_classes` on the
function or call signature, when some eightbyte is SSE or holds no member, or
when a parameter goes in memory (`../pa8/lowir.md`, Object ABI Conventions):
`struct { double a, b; }` is `sse_sse`, `{ double d; long i; }` is
`sse_integer`, an empty class is `none`, and a class with a `long double`
member is passed `memory`. The comparison checks them, and PA24's back end
passes such values in XMM registers by them.

For supported synthesized destructors, trivial union subobject destructor steps may be
omitted from enclosing synthesized destructors.

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

The PA12 suite is split by test role:

- `tests/general/`: the default PA12 LowIR oracle suite. These tests cover value-semantics
  lowering, copy/value helper emission, temporary materialization, ABI-shape
  cases, and cross-feature cases whose primary contract is generated LowIR plus
  exit status.
- `tests/spec/`: focused C++ language-contract cases that cite a specific N3485 clause.
  Each source test in this directory starts with a comment of the form:

    // N3485 focus: <clause> [<stable-name>] <short topic>

`tests/spec/` covers the PA12 value-semantics contract: defaulted/deleted
special members, copy/move construction and assignment, ref-qualified member
functions, delegating constructors, allocation expressions, unions, conversion
operators, and class value ABI behavior. `tests/general/` covers
value-semantics and LowIR-shape cases that are not tied to one specific C++11
clause.

PA12 is tested against the generated LowIR text.

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.cpp` runs one.
Controls that state no outcome are focused property checks run by this
assignment's own checkers.

### PA12 Syntax Spec

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa12.gram`. The grammar defines accepted syntax only;
the PA12 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

As in the earlier assignments, that grammar defines accepted input syntax only. The output
format for `cppgm++` is specified by this README, PA8 `lowir.md`, and the checked-in
`.ref` files.

Syntax for class value-semantics forms, including out-of-class constructor and
destructor definitions, is already part of that grammar; PA12 gives the
supported value-semantics subset semantic and lowering meaning.

Passing PA11 is necessary but not sufficient for passing PA12: an input may be syntactically
valid for PA5-PA11 and code-generation-valid for PA11 and still be outside the PA12
value-semantics slice described below.

A checked-in HTML grammar explorer for that grammar lives in `grammar/`. Treat
`pa12.gram` as the source of truth.

`pa12.gram` uses the same token vocabulary and the same extended BNF operators as
`../shared/source.gram`.

If this README and `pa12.gram` appear to disagree about source syntax, treat `pa12.gram`
as authoritative. If this README and PA8 `lowir.md` appear to disagree about LowIR syntax,
treat `lowir.md` as authoritative. If they disagree about the PA12 lowering slice, treat the
`Assignment Boundary` and `Out Of Scope` sections below as authoritative.

### Assignment Boundary

PA12 supports the following in addition to the PA11 subset:

- implicit copy constructors in the common field-wise/base-wise cases
- implicit copy assignment in the common field-wise/base-wise cases
- implicit move constructors in the common field-wise/base-wise cases needed by the
  supported value-semantics paths
- implicit move assignment in the common field-wise/base-wise cases needed by the
  supported value-semantics paths
- user-declared copy/move constructors and copy/move assignment operators in the ordinary
  non-template class cases needed by the supported value-semantics paths
- ordinary defaulted/deleted move-constructor and move-assignment cases in the supported
  non-template class patterns used by this assignment
- value passing of complete class objects to supported functions
- return-by-value of complete class objects from supported functions
- demand-driven LowIR emission of the copy/move/value helpers required by those supported
  paths
- raw `copyobj` lowering of a supported leading trivial storage prefix inside synthesized
  copy/move special members when the remaining suffix still needs ordinary field-wise
  lowering
- direct `copyobj` lowering of supported trivial class copy/move construction at the call
  site instead of forcing a separate synthesized trivial constructor call
- empty class objects and subobjects use the same address-based class copy paths as
  other class objects; lowering must not invent a scalar payload for an empty class
- an xvalue class glvalue bound to a reference through a derived-to-base
  conversion designates the existing base subobject; it is not materialized as
  a new complete object
- temporary class-object materialization in the common cases required by:
  - copy initialization from function results
  - pass-by-value call arguments
  - return forwarding through the supported value paths
- when a direct-register class call initializes a temporary whose destination
  is already known, its result is copied directly into that temporary; a
  conditional or full-expression cleanup boundary must not introduce a second
  call-result object
- direct reuse of the indirect return destination for supported `return local;` cases when
  the named local is the returned complete object
- ref-qualified member functions and out-of-class definitions of ref-qualified
  members, including xvalue propagation through non-static data-member access;
  ref-qualifiers are rejected on free functions, static members, constructors,
  and destructors, and an otherwise-identical member overload set cannot mix an
  unqualified declaration with a ref-qualified declaration
- rvalue-reference overload ranking after supported scalar pointer conversions,
  including null-pointer and pointer-qualification conversions
- delegating constructors; the delegating mem-initializer must be the only
  mem-initializer, and a delegation chain must not contain a cycle
- out-of-class constructor definitions
- out-of-class destructor definitions
- scalar `new` / `delete` expressions over the supported object subset,
  including class-specific allocation/deallocation selection, explicit global
  qualification, and suppression of scalar initialization after a supported
  non-throwing allocation returns null
- array `new` / `delete[]` expressions over the supported object subset; the
  request is a `std::size_t`, and the count is kept in an Itanium C++ ABI
  (2.7) cookie of max(sizeof(size_t), alignof(T)) bytes before the array
  unless T has a trivial destructor and its usual array deallocation
  function takes no size
- a runtime array `new` count that is negative, or whose request would exceed
  the implementation limit (`PTRDIFF_MAX` bytes, the limit a constant count
  is diagnosed against), obtains no storage: before the allocation function
  is called, the expression calls the `bad_array_new_length` runtime role,
  which throws `std::bad_array_new_length` (5.3.4p7); a count type that cannot
  hold such a value needs no check
- an array `new` braced-init-list copy-initializes the first elements from its
  initializer-clauses (8.5.1) and value-initializes the rest (8.5.1p7); an
  empty list value-initializes every element. A count smaller than the number
  of clauses is ill-formed when constant and obtains no storage at run time,
  as above. A constant count the list fills needs no default constructor
- union definitions and union object lifetime in the supported non-template
  class subset, including block-scope anonymous-member injection and an
  explicit variant initializer taking precedence over another variant's
  default member initializer; at most one variant may have a default member
  initializer; a namespace-scope anonymous union must be `static` in a named
  or the global namespace but not in an unnamed one (9.5p5)
- conditional class-value cases in the supported copy/move subset, including
  cv-combined glvalue operands, lvalue/prvalue conversion, and destruction of a
  containing branch temporary only after its selected member result has been
  materialized
- serialized `[elision=copy]` permission on the outer copy/move construction
  from a private same-type conditional prvalue, while retaining its ordinary
  O0 transfer and cleanup
- equal temporary-destruction suffixes in the same full-expression and unwind
  context use shared LowIR cleanup continuations, including conditional
  lifetime guards where the guarded object identity is the same
- class temporaries created earlier in an enclosing full expression remain
  alive across nested conditional and short-circuit branch edges, and are
  destroyed at the end of that full expression
- a class prvalue bound directly to a local reference remains alive until the
  reference's scope ends and is destroyed there rather than at the end of the
  declaration's full expression
- class-valued `if` condition declarations are constructed only on paths that
  reach the declaration and remain alive through the complete selection
  statement, including braceless nested statements
- non-template conversion operators that participate in the existing overload
  and conversion machinery

Within this milestone, PA12 should produce valid LowIR for ordinary non-polymorphic value
types over the supported PA11 procedural/class subset. That LowIR is intended
to be accepted by the later PA24 `lowir2native` backend for the supported
cases.
### Out Of Scope

The following are explicitly out of scope for PA12:

- virtual functions, vpointers, and vtables
- RTTI and `dynamic_cast`
- multiple inheritance
- member pointers
- generalized operator overloading beyond the supported value-semantics paths
- copy-elision perfection and the full set of standard temporary-materialization rules
- advanced move-generation rules beyond the common supported field-wise/base-wise cases
  above, and the full standard move-semantics corner cases
- throwing and catching exceptions (PA21); the cleanup regions described
  above are required all the same
- template-aware value semantics
- lambda expressions, range-for, and later general convenience syntax that is not
  needed by the PA12 value-semantics tests

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stage is PA13, which adds the polymorphic machinery that PA12
intentionally leaves out:

- virtual dispatch
- virtual destructors
- vtables
- override/final behavior

So PA12 should leave behind a clean non-polymorphic value-semantics object model and LowIR
lowering path rather than mixing virtual dispatch into the same milestone.

### Design Notes (Non-Normative)

The important point is to extend the existing PA11 behavior rather than inventing a second,
incompatible model just for copy/value behavior. Whether that reuse happens through shared
code, shared data structures, or a careful reimplementation is up to you.

An important implementation rule for this milestone is monotonic extension:

- PA12 should add value-semantics behavior only when the source actually requires it
- it should not perturb PA11 outputs for programs that remain entirely inside the PA11
  subset
- in practice, that means copy constructor / copy-assignment support should only become
  semantically visible when the program actually needs it, rather than eagerly changing the
  behavior or emitted output for every class
- "PA11 would have treated this as out of scope" is not a sufficient reason to let PA12
  change the observable output of a still-valid PA11 program

Useful intermediate representations include:

- class metadata that distinguishes ordinary methods, constructors, destructors, and
  synthesized special members
- explicit constructor/destructor/copy actions attached to declarations and returns
- for supported indirect-value local objects, those attached destructor actions should remain
  the source of truth for scope cleanup during LowIR lowering rather than being recomputed
  later from the lowered storage type alone
- a calling-convention layer that can lower class values indirectly without changing the
  source-level semantic types
- a stable way to identify the supported temporary-materialization points without requiring
  a fully general temporary lifetime model yet
- a value-category check that distinguishes prvalues needing storage from
  xvalue glvalues that already designate storage before applying a base
  projection for reference binding
- a destination-aware class-call lowering path that accepts the already
  planned temporary address for both direct-register and indirect-result ABI
  classes, then marks the temporary live only after the call and required
  direct-result copy complete
- extension of PA11 cleanup-state identities with temporary object and
  conditional-lifetime facts, built from the terminal backward so equal
  suffixes can be reused in expected constant time per action
- allocation expressions lowered as ordinary construction/destruction actions
  over explicit storage, rather than as a separate object model
- conversion operators represented through the same typed overload-resolution
  and conversion machinery used for ordinary calls
- a single linear classifier over completed member-layout facts that marks the
  first transferable bit-field in an allocation unit and suppresses later
  fields covered by that transfer

One small value returned from one function and passed to another is three
or four whole-object copies in this LowIR: field stores into the result, a
`copyobj` or a `return` of the object, and a `copyobj` into the argument's
storage. That is the intended shape. PA32 splits such slots into scalar
values when every use of their addresses is a direct load or store, a
`copyobj` or `zeroinit` at a constant offset, or an indirect result. A copy
lowered as a loop, a byte sequence or a call to a helper keeps the object in
memory.
