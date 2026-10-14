## CPPGM Programming Assignment 21 (`cppgm++ --emit-lowir`)

### Overview

PA21 finishes the deferred first-tier language features that sit on top of the existing
single-inheritance object model:

- capturing lambdas
- `std::initializer_list` semantic interoperation
- RTTI and `typeid`
- pointer- and reference-form `dynamic_cast`
- exception-aware construction and full-expression cleanup

PA21 still produces LowIR. It does not introduce a new output format.

### Prerequisites

You should complete Programming Assignment 20 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6-PA7 semantic foundation
- the PA10-PA20 LowIR lowering path
- the PA8 LowIR contract

### Starter Kit

The starter kit contains:

- `pa21/README.md`, `pa21/Makefile`, and the test scripts in `pa21/scripts/`
- your cumulative `dev/cppgm++.cpp` compiler entry point
- the `pa21/cppgm++.cpp` symlink back to `../dev/cppgm++.cpp`
- shared support sources and headers under `dev/src/`
- the grammar for this assignment called `pa21.gram`
- an HTML grammar explorer of `pa21.gram` in the sub-directory `grammar/`
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

`-O0` is the PA21 test mode. Other optimization levels are later optimizer work and
are not required for this milestone.

### Output Format

`cppgm++` shall write LowIR text to `<outfile>`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA21 extends the PA20 lowering
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

Standard output and standard error are ignored for automated testing of `cppgm++`.

You are free to use them for debugging, tracing, or diagnostic messages.

### Testing

Tests compare your output with the checked-in references using the LowIR
comparison described in [the format guide](../pa8/lowir.md). The `Makefile` invokes
`cppgm++` with `--emit-lowir -O0`.

The local checked-in tests live in `tests/general/`. That directory contains
PA21 source-to-LowIR tests for capturing lambdas, initializer-list
interoperation, RTTI, `typeid`, `dynamic_cast`, and exception-source lowering
interactions. `tests/spec/` holds a test anchored to a specific clause; the
harness runs it like the general suite.

For each test case `x`:

- `cppgm++` is executed to produce `x.my`
- the exit status is recorded in `x.my.exit_status`
- `x.my` is compared against `x.ref`
- `x.my.exit_status` is compared against `x.ref.exit_status`

PA21 is tested against generated LowIR text using the relaxed LowIR comparator described
above. For execution feedback, use the supplied `lowir2native-ref -O0`
backend introduced in PA8. Your own native backend is implemented in PA24.

The shipped PA21 tests are the contract for this milestone.

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.t` runs one.

### PA21 Syntax Spec

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa21.gram`. The grammar defines accepted syntax only;
the PA21 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

As in the earlier assignments, that grammar defines accepted input syntax only. The output
format for `cppgm++` is specified by this README, PA8 `lowir.md`, and the
checked-in `.ref` files.

PA21 does not add a new source-language grammar format. It instead enables more
of the already-accepted C++11 syntax to participate in semantic analysis and
lowering.

A checked-in HTML grammar explorer for that grammar lives in `grammar/`. Treat
`pa21.gram` as the source of truth.

`pa21.gram` uses the same token vocabulary and the same extended BNF operators as
`../shared/source.gram`.

If this README and `pa21.gram` appear to disagree about source syntax, treat `pa21.gram`
as authoritative. If this README and PA8 `lowir.md` appear to disagree about LowIR syntax,
treat `lowir.md` as authoritative. If they disagree about the PA21 lowering slice, treat the
`Assignment Boundary` and `Out Of Scope` sections below as authoritative.

### Assignment Boundary

PA21 supports the following in addition to the PA20 subset:

- capturing lambdas with supported explicit by-copy and by-reference captures of local
  values, including class objects whose existing copy-construction path is supported
- default `[=]` and `[&]` captures over the same supported local-value and `this` subset
- explicit `this` capture for supported member-function cases
- `std::initializer_list<T>` interoperation for supported scalar elements and
  class elements whose construction, copy, and destruction stay within the
  PA11/PA12/PA19 object and template subset
- `typeid(type-id)`
- `typeid(expr)` for supported polymorphic lvalue expressions
- `dynamic_cast<T*>(expr)` and `dynamic_cast<T&>(expr)` for supported polymorphic
  single-inheritance conversions

Within this milestone, PA21 should produce valid LowIR for ordinary source programs over
that subset. That LowIR should be accepted by PA24 `lowir2native` for the supported cases.

To complete PA21, implement these goals:

1. Capturing lambda lowering.
   Explicit by-copy captures should materialize deterministic closure-object LowIR and the
   resulting closure object should be callable through the existing class/method lowering
   path. A catch parameter declared inside a lambda body is local to that body and is not an
   implicit capture.

2. `std::initializer_list` interoperation.
   Supported braced-list calls should materialize deterministic lowered storage and expose
   the expected `__begin` / `__size` semantics to range-for lowering.
   A backing array with static storage duration, including one extended by a
   namespace-scope reference, is destroyed at exit element by element, last
   element first. An automatic backing array lives as long as its list: once
   it exists, every potentially throwing call later in its scope, including
   one in a condition, unwinds through cleanup that destroys the elements in
   reverse order and the earlier automatic objects.

3. RTTI and `typeid`.
   The compiler should emit deterministic RTTI globals and lower both static and dynamic
   `typeid` queries into ordinary LowIR address/load/branch operations. A dynamic
   `typeid` whose operand applies unary `*` to a pointer tests the pointer first:
   when it is null it calls the runtime's `bad_typeid` function, which throws
   `std::bad_typeid` and does not return (5.2.8p2). A reference or a named object
   cannot be null and is not tested.

4. Pointer- and reference-form `dynamic_cast`.
   The compiler should lower supported polymorphic single-inheritance pointer and
   reference casts into ordinary LowIR control flow without introducing new IR
   operations. A reference cast runs the same runtime search as a pointer cast; when
   the search fails it calls the runtime's `bad_cast` function, which throws
   `std::bad_cast` and does not return (5.2.7p9).

5. Full-expression cleanup through condition control flow.
   Temporary-owning call arguments inside nested `&&` and `||` expressions
   should be destroyed exactly on evaluated paths, and every nested logical
   result used by an outer condition yields a value the outer condition tests.
   Guarded local-static initialization should destroy initializer temporaries
   on the initialization edge before that edge joins the already-initialized path.
   An aggregate whose initialization may throw constructs each nontrivial
   member by a constructor call (not a representation copy), and the cleanup
   region after each call destroys exactly the members constructed so far.
   Synthesized copy/move constructors destroy completed bases, members and
   array elements if a later construction step throws. Representation-copy
   prefixes still establish subobject lifetimes. A partial member-array
   cleanup continues with earlier subobjects of the enclosing constructor.
   Exceptions escaping a source `try` inside a constructor or destructor body
   still destroy the appropriate bases and members. An enclosing body handler
   may catch the exception before that cleanup; a function-try handler runs
   after it. A delegating constructor destroys its completed target when its
   own body fails. A second exception during unwind destruction terminates.
   A destructor declared without an exception specification has the one its
   implicit declaration would have (12.4p3, 15.4p14), normally non-throwing,
   and states `unwind=no` exactly when that one is non-throwing; a base's or
   member's `noexcept(false)` destructor makes it potentially throwing. A call
   through a pointer, reference or pointer to member whose declaration (a
   variable, parameter or data member) is non-throwing states `unwind=no` on
   its call signature, as `noexcept` of the call answers true (5.3.7p3); a
   call through the result of a call or a conditional states none, and the
   call keeps its unwind path either way. A
   non-throwing function, such a destructor included, whose definition may
   throw runs its whole definition under a terminate guard: one region whose
   handler calls `std::terminate` (15.5.1). A definition may throw when it
   calls a function that may throw, has a throw-expression, or initializes
   or destroys a subobject in a way that may throw; a delete-expression
   counts as its destructor call does, since the deallocation functions do
   not throw. A definition in which nothing can unwind opens no guard.
   Function-template instantiation retains function-try bodies and handlers,
   including constructor mem-initializers within the function-try block.
   A cleanup call names the destructor of the concrete specialization that
   owns the subobject; instantiating that destructor must not require the
   enclosing class to be complete earlier than the standard allows (14.7.1),
   so a recursively containing type is accepted.
   A class return value built in the caller's destination by a constructor,
   copy or call that may throw unwinds the function's automatic objects
   first; each arm of a class conditional does so too, and a completed result
   stays owned until the return's cleanup succeeds.
   A by-value class parameter with a non-trivial destructor stays owned by the
   caller (Itanium C++ ABI 3.1.2.3): the caller destroys it at the end of the
   full-expression containing the call, and on the unwind path when the callee,
   a later argument or a later part of that full-expression throws. The callee
   never destroys its parameter, and an inheriting constructor forwards the
   parameter's address unchanged.
   Once an exception object has been initialized, destroy the throw operand's
   temporaries; no later cleanup region destroys them again. A temporary from an
   untaken throw branch must not appear in a sibling call's cleanup path.
   A pointer exception is caught by value (the runtime's begin-catch result is
   the thrown pointer, converted to the handler's type), so a handler declared
   as a reference to a pointer binds to a copy of the caught pointer. A
   pointer handler also matches a qualification conversion, a conversion to
   an unambiguous public base at the first level, an object pointer as cv
   `void*`, and `std::nullptr_t` (15.3p3); a class's type_info describes a
   single base with `__si_class_type_info` only when that base is public,
   non-virtual and at offset zero.
   Operandless `throw;` rethrows the dynamically handled exception, including
   from a called function or destructor. With no active exception it terminates
   at runtime; it does not require a lexical enclosing handler.
   Reference initialization preserves cleanup of earlier automatic objects.
   A temporary whose lifetime is extended by the reference stays guarded until
   initialization completes, then remains alive until scope exit; a reference
   returned by a call does not extend the lifetime of its temporary arguments.
   If a conditional initializer arm throws before the destination object is
   constructed, do not schedule destruction of that destination on the unwind path.
   A reference cast from a conditional class prvalue materializes the complete
   object before selecting a base subobject; lifetime extension retains that
   complete object and its destructor.
   A class-valued conditional may have a `throw` operand on either side. Its
   value arm retains ordinary construction and cleanup; its throwing arm
   unwinds objects that were already live before the branch.
   When a potentially throwing call is reached through a branch in an active
   handler, its unwind path must finish the handler and destroy objects that
   remain live from scopes outside the corresponding `try` statement.
   A deallocation function declared without an exception specification is
   non-throwing (15.4p15).
   If initialization after ordinary scalar allocation throws, call the selected
   usual deallocation function after construction cleanup and propagate the exception.
   Array new initializes its elements in order, from a braced-init-list's clauses
   and then by value-initialization or default construction; if one throws,
   destroy the elements already constructed in reverse order, then call the
   selected usual deallocation function.
   Successful initialization transfers the allocation to the result.
   If construction of a class subobject throws, destroy exactly the already
   constructed bases and members in reverse construction order.
   Aggregate member and array-element initialization must retain the completed
   prefix when a later initializer throws, including a later scalar initializer.
   Initializer temporaries participate in that completion order and remain live
   through the full expression. A conditional join must retain the guarded
   temporaries from whichever arm was evaluated.
   Two unwind paths whose remaining cleanup sequence and handler context are
   identical share one block, the later path jumping into the earlier block
   where the sequences coincide (the references share them so, and the
   comparison does not absorb the alternative).

### Out Of Scope

The following are explicitly out of scope for PA21:

- init-captures
- class captures that require unsupported copy construction, destruction, or object-model
  features
- `std::initializer_list` class elements that require unsupported construction,
  copy, destruction, or later object-model behavior
- `dynamic_cast<void*>`
- multiple inheritance and virtual inheritance
- any PA21 feature path that depends on unsupported later object-model or ABI work

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stage is PA22, which completes the remaining non-virtual object-model work
that PA21 still deliberately avoids, especially non-virtual multiple inheritance and the
remaining single-vptr RTTI case `dynamic_cast<void*>`.

So PA21 should leave behind:

- a stable advanced-language semantic layer over the existing single-inheritance model
- LowIR lowering for the supported RTTI, lambda-capture, and initializer-list subset
- explicit remaining deferrals only where PA22 really needs to take over

Virtual inheritance and polymorphic multiple inheritance remain intentionally deferred beyond
PA22.

### Design Notes (Non-Normative)

PA21 should extend the existing semantic and lowering path, not replace it.

Cleanup continuation keys can use dense identities for the complete active
exception-region stack. This permits expected constant-time state interning
without comparing rendered LowIR or rescanning the region stack at each call.

The same monotonic-extension rule applies here:

- PA21 should add its new behavior only when the source actually uses the supported PA21
  feature set
- it should not perturb PA20 outputs for programs that remain entirely within the PA20
  subset
- in practice, RTTI globals, closure helpers, and dynamic-cast support should stay
  on-demand rather than eagerly changing the behavior of ordinary earlier programs that do
  not use those features

#### What later stages need from the exception shape

A function with a destructible local and a call that may throw has a cleanup
region, so in real C++ code, this compiler's own source included, most
functions have one. The shape this assignment emits is therefore the common
case for PA32's optimizer and for the PA24 and PA33 backend, not a rare path
they can treat conservatively. A few properties decide how much of that code
they can improve:

- Close full-expression regions before control-flow joins, and give the
  paths that merge the same protected-call state (PA26 states this as a
  requirement for host objects). PA32 removes a region that nothing inside
  can unwind out of, and promotes slots across the function; both rely on
  every block having one stack of active regions.
- Handler edges are implicit in LowIR: an exception leaves from the region
  opener's block, not from the call that throws. An optimizer that does not
  model those edges must keep a value in memory where a handler might read
  it, and cannot inline a callee that contains exception instructions. The
  course solution's PA32 is in that position: it inlines such a callee only
  when constant arguments fold its exception instructions away. Region
  granularity is therefore a performance decision as well as a correctness
  one.
- A backend that treats each landing pad as a successor of every call that
  can unwind into it can keep values in registers across those calls, as
  the host model of PA26 allows. A runtime whose handler records restore
  callee-saved registers to their values when the region was entered does
  not, and in the course runtime a function with regions keeps its
  cross-block values in memory.
