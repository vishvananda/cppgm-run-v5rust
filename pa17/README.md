## CPPGM Programming Assignment 17 (`cppgm++ --emit-lowir`)

### Overview

PA17 is the first half of template completion. Its job is to finish the
template declaration and specialization model so the compiler knows:

- what template entities exist
- how template-template parameters, member templates, and friend templates are
  represented
- what specializations exist
- which specialization is selected
- which declarations/definitions own the selected specialization

PA17 still produces LowIR. It does not introduce a new output format.

### Prerequisites

You should complete Programming Assignment 16 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6-PA7 semantic foundation
- the PA10-PA16 LowIR lowering path
- the PA8 LowIR contract
- the PA14-PA15 template and metaprogramming machinery
- the PA16 full constant-evaluation layer

### Starter Kit

The starter kit contains:

- a `cppgm++.cpp` assignment entry point, linked to the editable compiler source
  in `../dev/cppgm++.cpp`
- the standard assignment `Makefile` and harness scripts
- the PA17 specialization/entity test suite under `tests/`

Extend the `cppgm++` implementation you built in earlier assignments; the
initial scaffold is not reinstalled at this milestone.

Use the supplied reference tools to inspect example output. Tests compare
your compiler with the checked-in contract references.

### Input / Command-Line Arguments

The PA17 invocation is the unoptimized LowIR mode:

    $ cppgm++ --emit-lowir -O0 -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

Behaviour is undefined unless the command-line arguments match that shape, with
the same source-file ordering and `-o` relaxations as the earlier source-to-LowIR
milestones. Preserve PA7's dump mode. Native driver behavior and optimized
LowIR output are not required in PA17.

### Dependent qualified types

Within a retained template definition, a qualified member type whose qualifier
depends on a template parameter requires the C++11 `typename` introducer.  This
is checked when the template is defined, even if no specialization is requested.
A type member already established in the current instantiation retains the
standard exception; a different specialization of the same class template does
not.  The exception applies to parameter types after an out-of-class member's
qualified declarator, but not to a dependent return type written before that
declarator.  A dependent template name used as a template-template
argument uses the `template` introducer and is not misclassified as a member
type requiring `typename`.  In a type position, a dependent member template-id
requires both introducers: `typename` establishes that the result is a type,
while `template` establishes that the qualified member is a template.  A
template-id whose argument contains a pack expression such as
`sizeof...(Pack)` is dependent even when a later substitution gives it the
same specialization as a fixed template-id.  Because a dependent
qualified-id without `typename` does not name a type, a declarator whose
parenthesized operand is only such a name declares a variable with a
direct-initializer rather than a function with an unnamed parameter.

### Qualified template-name hiding

Function and variable templates declared directly in a class participate in
the ordinary member-name lookup namespace and hide same-named declarations
inherited from a base class.  This lookup rule applies before overload ranking:
a hidden base non-template must not be admitted merely because the derived
declaration has no concrete binding until template deduction.  Naming a
variable template without its required template arguments is invalid; lookup
does not fall through to a hidden base data member.

### Output Format

On success, `cppgm++` shall write LowIR text to `<outfile>` and exit
`EXIT_SUCCESS`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA17 extends the
PA16 lowering surface only by making more of the C++ source language lower into
the already-defined LowIR family.

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
part of the assignment contract. Exact textual LowIR matching is not a PA17
grading requirement.

### Error Handling

If an error occurs during preprocessing, tokenization, parsing, semantic
analysis, or LowIR generation, `cppgm++` shall `EXIT_FAILURE`.

The output file is not required to be meaningful on failure.
Diagnostics are not part of the grading contract.

### Standard Output / Error

Standard output and standard error are ignored for automated testing of
`cppgm++`.

You are free to use them for debugging, tracing, or diagnostic messages.

### Testing

PA17 tests live under `tests/`. The suite is split by test role:

- `tests/spec/` contains N3485/spec-anchored specialization/entity tests. Most
  carry a `// N3485 focus: 14.x.y [clause.name] ...` comment near the top,
  naming the governing text in `../doc/n3485.txt`.
- `tests/general/` contains broader cross-feature and realistic
  template-entity examples that are useful for PA17 but are not one-rule spec
  probes.

The `make test` target runs both directories through the LowIR validator. For
successful tests, the validator checks the reference LowIR and your generated
LowIR for basic structural correctness, then compares the canonicalized LowIR
against the checked-in reference. For rejected tests, the exit status is the
checked result; exact diagnostic text is not checked.

This split assignment intentionally focuses on the specialization/entity half of
template completion:

- class partial specialization and specialization selection
- alias and variable template entity modeling
- explicit specialization and explicit-instantiation ownership
- the declaration/instantiation behavior required to make that model coherent

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.t` runs one.

### PA17 Syntax Boundary

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa17.gram`. The grammar defines accepted syntax only;
the PA17 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

### Optional Student Test Ideas

When adding your own tests, useful PA17 themes include alias/variable template
entities, template-template parameters, member templates, friend templates,
class partial specialization selection, explicit-instantiation ownership,
constructor/member-template specialization ownership, and partial ordering
boundaries.

### Assignment Boundary

PA17 owns the template declaration graph and specialization model over the
implemented language surface, including:

- alias templates
- variable templates (a C++14 feature, which the course accepts in C++11, as
  clang and g++ do with a warning); each specialization is its own object, its
  symbol carrying the template arguments (`_Z5widthIiE`,
  `_ZN5store5countIcEE`, `_ZN5table5countIiEE`); a static data member template
  may be defined outside its class, an ordinary class or a class template, and
  the definition's initializer is analyzed in the class's scope
- template-template parameters and template-template argument matching
- member templates, including templated member operators and templated call
  operators
- access checking for member class templates, alias templates, and nested type
  paths, including inherited access and template-template arguments
- friend templates in the supported class-template/function-template subset, including a
  friend type template-id whose non-type argument is a dependent constant expression
- class partial specialization
- partial-specialization ordering and specialization selection
- current-specialization identity in the supported class-template and
  specialization cases
- explicit-instantiation declarations and definitions over the supported surface
- an explicit-instantiation declaration for a class specialization suppresses
  non-inline instantiation but keeps a locally used in-class inline or defaulted
  member definition available
- integration with PA15 explicit specialization declarations/definitions when
  they interact with the PA17 specialization graph
- collection/ownership behavior for constructor/member-template specializations
  and namespace-scope friend-template declarations
- an explicit definition of a member template for a concrete class-template
  specialization replaces the definition instantiated from the primary and may
  not itself be defined more than once
- a member template called before its out-of-class definition is still
  instantiated from that definition (14.6.4.1), whatever the order of the call
  and the definition
- a block-scope static in a function-template or member-template
  specialization whose substituted initializer is a constant expression is
  constant-initialized data with no guard, as in an ordinary function (PA16),
  even when the initializer names other specializations' functions
- the dependent-name and instantiation behavior strictly required to make the
  specialization model work

### Out Of Scope

The following are explicitly out of scope for PA17:

- full function-template deduction over the intended language surface
- function-template partial ordering
- SFINAE and substitution-failure completion
- dependent-call and instantiation-timing behavior (PA18)
- initializer-list template behavior
- hosted/vendor-only extensions that happen to use templates
- post-C++11 template-language features other than the variable templates above
- broad multi-feature integration cases whose main assertion is that several
  completed template features compose

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stage is PA18, which finishes deduction, substitution, and
SFINAE over the now-complete specialization model.

So PA17 should leave behind:

- a stable template declaration/specialization graph
- deterministic specialization selection
- specialization ownership that lowers through the ordinary LowIR path
- no remaining "template entity model later" gap before full template
  completion

### Design Notes (Non-Normative)

Extend the existing semantic template model for aliases, variable templates,
partial specializations and higher-order arguments. Substitute through semantic
types, values, dependent expressions and entity identities, retaining checks
whose validity depends on supplied arguments. An already known result type
does not prove that substitution is valid, including when an alias discards
an argument from its result.

Use canonical argument keys, PA16 constant values and template-entity bindings.
Keep specialization selection separate from the demanded declaration,
initializer or body. Retain deterministic partial-specialization selection and
explicit links to the class or namespace owning each generated entity.
