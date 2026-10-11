## CPPGM Programming Assignment 20 (`cppgm++ --emit-lowir`)

### Overview

PA20 closes the first large batch of ordinary C++11 language features that were deferred
while the compiler was still building its type, object, template, and backend layers.

This milestone focuses on:

- `auto` variable type deduction
- `auto` function return type deduction for function definitions with visible
  bodies, including member functions, function templates and member templates
  (a C++14 feature, which the course accepts in C++11; clang and g++ reject it
  under `-std=c++11`)
- direct braced initialization of supported scalar and array objects
- captureless lambdas plus the supported by-reference local / `this`-capture subset
- range-for over bounded arrays and supported user-defined `begin` / `end` ranges

PA20 still produces LowIR. It does not introduce a new output format.

### Prerequisites

You should complete Programming Assignment 19 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6-PA7 semantic foundation
- the PA10-PA19 LowIR lowering path
- the PA8 LowIR contract

The intended direction is:

- PA5 provides syntax
- PA6-PA7 provide typed semantic analysis
- PA10-PA19 lower the supported language subsets to LowIR
- PA20 extends that same lowering path with the remaining first-tier core-language features

### Starter Kit

The starter kit contains:

- `pa20/README.md`, `pa20/Makefile`, and the test scripts in `pa20/scripts/`
- your cumulative `dev/cppgm++.cpp` compiler entry point
- the `pa20/cppgm++.cpp` symlink back to `../dev/cppgm++.cpp`
- shared support sources and headers under `dev/src/`
- the grammar for this assignment called `pa20.gram`
- an HTML grammar explorer of `pa20.gram` in the sub-directory `grammar/`
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

`-O0` is the PA20 test mode. Other optimization levels are later optimizer work and
are not required for this milestone.

### Output Format

`cppgm++` shall write LowIR text to `<outfile>`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA20 extends the PA19 lowering
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
PA20 source-to-LowIR tests, cross-feature combinations, and boundary cases over
the broad core-language closure slice. `tests/spec/` holds a few tests
anchored to specific clauses; the harness runs them like the general suite.

For each test case `x`:

- `cppgm++` is executed to produce `x.my`
- the exit status is recorded in `x.my.exit_status`
- `x.my` is compared against `x.ref`
- `x.my.exit_status` is compared against `x.ref.exit_status`

PA20 is tested against generated LowIR text using the relaxed LowIR comparator described
above. For execution feedback, use the supplied `lowir2native-ref -O0`
backend introduced in PA8. Your own native backend is implemented in PA24.

The shipped PA20 tests are the contract for this milestone.

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.cpp` runs one.

### PA20 Syntax Spec

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa20.gram`. The grammar defines accepted syntax only;
the PA20 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

As in the earlier assignments, that grammar defines accepted input syntax only. The output
format for `cppgm++` is specified by this README, PA8 `lowir.md`, and the checked-in
`.ref` files.

PA20 does not add a new source-language grammar format. It instead enables more
of the already-accepted C++11 syntax to participate in semantic analysis and
lowering.

A checked-in HTML grammar explorer for that grammar lives in `grammar/`. Treat
`pa20.gram` as the source of truth.

`pa20.gram` uses the same token vocabulary and the same extended BNF operators as
`../shared/source.gram`.

If this README and `pa20.gram` appear to disagree about source syntax, treat `pa20.gram`
as authoritative. If this README and PA8 `lowir.md` appear to disagree about LowIR syntax,
treat `lowir.md` as authoritative. If they disagree about the PA20 lowering slice, treat the
`Assignment Boundary` and `Out Of Scope` sections below as authoritative.

### Assignment Boundary

PA20 supports the following in addition to the PA19 subset:

- `auto` in variable declarations when exactly one declarator is present and an initializer
  is provided
- `const auto` and similar cv-qualified `auto` variable declarations over the same subset
- a function, member function or function template declared with an `auto`,
  cv-qualified `auto`, `auto*`, `auto&` or `auto&&` return type and defined
  with a body: the return type is deduced from the `return` statements as an
  `auto` variable's type is from its initializer, once per specialization for
  a template; `return` statements that deduce different types, or a use of
  the function before its return type is known, are rejected
- direct braced initialization for supported scalar objects
- direct braced-init expressions over the supported scalar / array / class subset when the
  earlier PA11-PA19 object/value semantics already define the target
- braced initialization of bounded arrays with compile-time known size
- arrays of aggregate elements whose array members receive nested braced
  sub-lists, including zero-initialization of omitted member elements
- direct aggregate construction when the target aggregate type is already supported by the
  earlier object-model assignments
- ordinary function-call argument conversion through non-explicit converting constructors
  and conversion operators over the supported class subset
- explicit non-class functional casts between the supported integral and enum forms
- `reinterpret_cast` between the supported pointer and integer forms
- captureless lambda expressions plus the supported by-reference local and explicit/implicit `this`
  capture subset
- range-for statements over:
  - bounded arrays
  - supported class/member and ADL `begin` / `end` ranges whose iterator operations stay
    within the already-supported call/operator subset

Within this milestone, PA20 should produce valid LowIR for ordinary source programs over
that subset. That LowIR should be accepted by PA24 `lowir2native` for the supported cases.

To complete PA20, implement these goals:

1. `auto` variable deduction.
   The compiler should deduce the declared type from the initializer and lower the resulting
   variable just like an equivalent explicit declaration, including ordinary pointer and
   reference declarators such as `auto*`, `auto&`, and `auto&&`.

2. Direct braced initialization.
   Supported scalar and array declarations should lower cleanly from `{...}` source forms,
   not only from `=` initializer syntax.

3. Captureless lambda lowering.
   Captureless lambdas should become callable lowered entities with deterministic LowIR.
   A function-local static declared in a lambda has distinct storage for each
   lambda expression and each enclosing function-template specialization; a
   call through the function pointer a captureless closure converts to runs
   the same body, so it shares the call operator's statics (N3485 5.1.2p6).
   A lambda expression has one closure type (5.1.2p3) however many times its
   context is lowered: a lambda in a default member initializer is one
   closure in every constructor that initializes the member; one in an
   in-class default argument is one at every call that uses it; one in a
   constructor's or destructor's body is one in every entry that runs the
   body; and one in a class template's static data member initializer or a
   variable template's initializer is one per specialization.
   The call operator, and the static function the conversion to a function
   pointer returns, is an inline function (5.1.2p5) that states
   `inline_hint=yes`, and `unwind=no` when the lambda is declared `noexcept`
   or `throw()`; a lambda-expression called directly states its call
   operator's `unwind=no` on the call signature.

4. Range-for lowering.
   Range-for over arrays and supported user-defined `begin` / `end`
   ranges should lower into ordinary loop/control-flow structure in LowIR, including
   ordinary reference loop declarations such as `const int&` and `const auto&`. A
   materialized class prvalue used as the range remains alive through the loop and is
   destroyed when the complete range-for statement ends.

The test suite also exercises a small remaining ordinary-language closure cluster here:
direct braced-init expressions, direct aggregate construction, supported integral / enum
functional casts, and pointer / integer `reinterpret_cast`.

### Out Of Scope

The following are explicitly out of scope for PA20:

- capturing lambdas other than the supported by-reference local / `this`-capture subset
- `std::initializer_list` semantic interoperation
- RTTI and `typeid`
- `dynamic_cast`
- placeholder return-type declarations without a visible definition body
- `decltype(auto)` (C++14; not in `pa20.gram`)
- range-declarations that require unsupported user-defined iterator or reference semantics
- any PA20 feature path that depends on unsupported PA11-PA19 semantics

Inputs that rely on those features have undefined behaviour for this milestone.
The PA20 test suite therefore does not require those inputs to fail deterministically; it
only checks the defined PA20 feature subset above.

### Stage Handoff

The intended next stage is PA21, which finishes the remaining deferred advanced-language
corners over the current single-inheritance object model before PA22 tackles the remaining
ABI and inheritance closure work.

So PA20 should leave behind:

- a stable first-tier language-closure semantic layer
- LowIR lowering for the ordinary non-advanced C++11 forms added here
- explicit remaining deferrals only where PA21 really needs to take over

### Design Notes (Non-Normative)

PA20 should extend the existing semantic and lowering path, not replace it.

The same monotonic-extension rule applies here:

- PA20 should add its new behavior only when the source actually uses the supported PA20
  feature set
- it should not perturb PA19 outputs for programs that remain entirely within the PA19
  subset
- in practice, lambda helper synthesis and `auto` deduction should stay on-demand rather
  than eagerly changing the behavior of ordinary earlier programs that do not use those
  features
