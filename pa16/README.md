## CPPGM Programming Assignment 16 (`cppgm++ --emit-lowir`)

### Overview

PA16 extends PA15's first practical metaprogramming slice into a full language-level
constant-evaluation milestone. Its job is to make `constexpr` semantics a first-class part
of the compiler rather than leaving constant evaluation as only the small pragmatic subset
needed by PA15 template arguments and `static_assert`.

To complete PA16, implement these goals:

- `constexpr` function evaluation
- `constexpr` constructors, member functions, and variables
- constant initialization and object-valued constant evaluation
- a reusable constant-expression engine for both ordinary source semantics and later
  template machinery

PA16 still produces LowIR. It does not introduce a new output format.

### Prerequisites

You should complete Programming Assignment 15 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6-PA7 semantic foundation
- the PA10-PA15 LowIR lowering path
- the PA8 LowIR contract
- the PA15 metaprogramming and integral constant-expression machinery

### Starter Kit

The starter kit contains:

- a `cppgm++.cpp` assignment entry point, linked to the editable compiler source
  in `../dev/cppgm++.cpp`
- the standard assignment `Makefile` and harness scripts
- the grammar for this assignment, `pa16.gram` (no HTML explorer ships for
  PA16)
- a checked-in local test suite under `tests/`

Extend the `cppgm++` implementation you built in earlier assignments; the
initial scaffold is not reinstalled at this milestone.

Use the supplied reference tools to inspect example output. Tests compare
your compiler with the checked-in contract references.

### Input / Command-Line Arguments

The PA16 invocation is the unoptimized LowIR mode:

    $ cppgm++ --emit-lowir -O0 -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

Behaviour is undefined unless the command-line arguments match that shape, with
the same source-file ordering and `-o` relaxations as the earlier source-to-LowIR
milestones. Preserve the earlier dump modes. Native driver behavior and optimized
LowIR output are not required in PA16.

### Output Format

On success, `cppgm++` shall write LowIR text to `<outfile>` and exit
`EXIT_SUCCESS`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA16 extends the PA15 lowering
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

The test harness checks that the generated LowIR is well formed and matches the
checked-in `.ref` files after canonicalizing presentation details that are not
part of the assignment contract: the names of temporaries, slots, blocks and
parameters, the order of slot declarations and of top-level entries, and the
spelling of literals (`../pa8/lowir.md`, "What The Comparison Absorbs And What
It Enforces", is the one list). Exact textual LowIR matching is not a PA16
grading requirement, but every instruction of a body must match the
reference's after that canonicalization, in its order.

### Error Handling

If an error occurs during preprocessing, tokenization, parsing, semantic analysis, or LowIR
generation, `cppgm++` shall `EXIT_FAILURE`.

The output file is not required to be meaningful on failure.
Diagnostics are not part of the grading contract.

### Standard Output / Error

Standard output and standard error are ignored for automated testing of `cppgm++`.

You are free to use them for debugging, tracing, or diagnostic messages.

### Testing

PA16 tests live under `tests/`. The suite is split by test role:

- `tests/spec/` contains N3485/spec-anchored constant-evaluation tests. Each
  provided C++ language test in this directory starts with a leading comment of the
  form `// N3485 focus: 7.1.5 [dcl.constexpr] ...` or another exact governing
  clause so a reviewer can find the text in `../doc/n3485.txt`.
- `tests/general/` contains broader constexpr cross-feature and realistic
  constant-evaluation examples that are useful for PA16 but are not one-rule
  spec probes.

The `make test` target runs both directories through the LowIR validator. For
successful tests, the validator checks the reference LowIR and your generated
LowIR for basic structural correctness, then compares the canonicalized LowIR
against the checked-in reference. For rejected tests, the exit status is the
checked result; exact diagnostic text is not checked.

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.t` runs one.

### PA16 Syntax Boundary

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa16.gram`. The grammar defines accepted syntax only;
the PA16 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

### Optional Student Test Ideas

When adding your own tests, useful PA16 themes include C++11 `constexpr`
declaration validity, literal type requirements, constant initialization, core
constant-expression rejection, pointer/reference constant evaluation, and
aggregate/object-valued constant evaluation.

### Assignment Boundary

PA16 owns full `constexpr` / constant-evaluation semantics over the implemented language
surface inherited from PA15, including:

- full constant-expression evaluation for the implemented expression/type subset
- `constexpr` functions
- `constexpr` constructors and member functions
- `constexpr` variables and constant initialization
- floating-point `constexpr` evaluation over the implemented scalar language surface
- `noexcept` constant expressions over the supported call/expression subset
- object-valued, pointer-valued, and reference-valued constant evaluation where the earlier
  language/object-model milestones already define the underlying semantics
- reuse of the constant evaluator for ordinary language semantics, template arguments, and
  `static_assert`

The intent is no longer a pragmatic subset. By the end of PA16, `constexpr` should be a
complete compiler-owned semantic layer for the supported C++11 language surface, not a
collection of special cases.

More concretely, over the already-implemented language subset, PA16 should cover the full
C++11 `constexpr` forms that later template and library code expect:

- scalar, floating, `nullptr`, and enum constant expressions
- unary, arithmetic, comparison, bitwise, logical, conditional, cast, `sizeof`,
  `alignof`, `sizeof...`, and `noexcept` constant expressions where those operators are
  already part of the supported language surface
- `constexpr` free-function calls, including recursive calls and default arguments
- `constexpr` constructors, including member-initializer lists and base/member
  initialization for literal class types; access checks performed while
  evaluating those initializers use the constructor's class context, so a
  derived constructor may invoke an accessible protected base constructor
- `constexpr` member-function calls on constant objects
- `constexpr` function templates and `constexpr` member functions of class
  templates, including dependent return and parameter types: the literal-type
  requirements are checked on the template's declaration, and a
  specialization whose instantiated types are not literal is still a valid
  function, only not `constexpr` (7.1.5p6)
- constant object values, not just integral scalars:
  - aggregate/class values
  - array values
  - nested aggregate/array values
- member access on constant objects via `.`
- array and string-literal element access via `[]`
- `constexpr` variables whose initializers must be fully evaluated at compile time
- an automatic nonvolatile array of trivial scalar elements whose complete
  initializer is known at compile time is initialized from readonly constant
  data with one object copy; each automatic array still has distinct storage
- reference-valued constant evaluation and `const T &` / reference parameter passing where
  the implemented object model already defines the underlying semantics
- lookup and reuse of previously computed constant values, including qualified lookup and
  static data members
- function-local static objects over the supported LowIR subset:
  - constant initialization when the initializer is a constant expression
  - dynamic class-object local statics with the required guard/check behavior, including
    direct initialization from a class-prvalue factory call
  - a temporary bound to a local static reference persists for the
    reference's lifetime (12.2p5): it occupies static storage, not the
    initializing call's frame, and is destroyed at exit with the other
    static objects (thread storage for a `thread_local` reference)
  - one object per declaration wherever the body is lowered more than once: a
    constructor's or destructor's complete and base-subobject entries share
    their statics (N3485 6.7p4)
- ordinary namespace objects whose constant-initialization probe encounters a
  core constant-expression failure use dynamic initialization; only contexts
  that require a constant expression are rejected for that failure
- a `constexpr` object is const, so one that is statically initialized states
  `storage=readonly` when its class has no `mutable` member and no
  non-trivial destructor, as PA11's const objects do; the temporary that a
  constant-initialized reference binds is readonly by its own type
  (`const int& r = 3` binds a readonly `const int`, `int&& r = 3` a writable
  `int`)

A call that constant evaluation replaces by its value does not make the
function used in emitted code, so a `constexpr` function whose every call is
evaluated at compile time (in a `static_assert`, a template argument, an
array bound, an enumerator or a constant initializer) need not be emitted.
The comparison accepts output with or without such unreferenced definitions.

PA16 also owns the semantic validation side of C++11 `constexpr`, not just evaluation. In
particular, the compiler should enforce the C++11-facing rules that matter for the
supported language subset, such as:

- `constexpr` variables require a compile-time initializer and a literal type
- `constexpr` function return and parameter types must be literal types
- `constexpr` constructors must produce literal objects through valid base/member
  initialization
- invalid `constexpr` declarations should fail during semantic analysis instead of being
  accepted and only failing later during use
- an executed declaration whose initializer is not constant invalidates the enclosing
  constant evaluation even when the declared value is not read
- a member call on a temporary is constant only when construction of that temporary is
  itself a valid constant expression

The implementation may support a strict superset of C++11 evaluation rules internally,
such as local variables, assignment, and loops inside constexpr evaluation. That is fine
and often useful for later milestones, but it does not reduce the requirement that the
standard C++11 forms above be covered cleanly and intentionally.

Constant evaluation is bounded, as Annex B allows. Support at least 512
nested `constexpr` calls, Annex B's figure. An evaluation that nests deeper,
or does more work than your evaluator's own limit, is not a constant
expression: a context that requires one is rejected, and a namespace object
falls back to dynamic initialization as above. Runaway recursion must end in
a clean rejection, not a crash or a hang.

### Out Of Scope

The following are explicitly out of scope for PA16:

- template language features that still remain deferred to PA17
- post-C++11 constant-evaluation features
- hosted/vendor-only compatibility forms that are not part of the standard C++ language

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stages are PA17 and PA18, which finish the remaining standard template
language using the now-complete constant-evaluation engine:

- PA17: complete the template entity and specialization model
- PA18: complete deduction, substitution, and SFINAE over that model

So PA16 should leave behind:

- a stable constant-evaluation semantic layer
- ordinary lowered declarations that no longer depend on PA15-specific constant-expression
  shortcuts
- a clear boundary where the remaining template-language work can build on real `constexpr`
  support rather than special-casing it

Automatic scalar-array image eligibility depends on the evaluated initializer,
not on whether the declaration spells `constexpr` or `const`. PA10 establishes
the literal-initializer case; PA16 adds general constant evaluation. Readonly
images may be shared, while each automatic array retains its own writable
storage. Nonconstant initializers retain runtime evaluation, and volatile
arrays retain their required stores. Class construction and destruction stay
on their typed lifetime paths.

### Design Notes (Non-Normative)

The useful shape for PA16 is one typed constant-evaluation layer shared by
`constexpr`, template arguments, `static_assert`, constant initialization, and
ordinary semantic checks.

Useful intermediate representations include:

- typed constant values for scalars, enums, pointers, references, arrays, and
  class objects
- a distinction between checking whether a declaration is valid `constexpr` and
  evaluating an expression in a constant-evaluation context
- reusable evaluated-value storage for bindings whose constant value is needed
  by lookup, template arguments, and later LowIR lowering
- typed structural interning of identical automatic constant-data templates,
  using data-item kinds, values, symbol identities, addends, size, and alignment
  rather than rendered LowIR text
- local-static initialization metadata that records whether LowIR lowering can
  emit a constant initializer or must emit guarded dynamic initialization
