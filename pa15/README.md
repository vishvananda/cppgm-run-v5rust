## CPPGM Programming Assignment 15 (`cppgm++ --emit-lowir`)

### Overview

PA15 extends PA14’s first-tier templates with the first practical compile-time
metaprogramming layer:

- integral non-type template parameters
- integral non-type template arguments
- type and non-type template parameter packs
- pack expansions in supported declaration, call, and instantiated body shapes
- explicit specialization of supported class templates and function templates
- integral constant-expression evaluation for template arguments
- `static_assert` over the supported integral constant-expression subset

### Prerequisites

You should complete Programming Assignment 14 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6 declarator/type model
- the PA7 call-resolution layer
- the PA10-PA14 LowIR lowering path
- the PA8 LowIR contract
- the PA14 template declaration, lookup, deduction, and instantiation machinery

The intended direction is:

- PA5 provides syntax
- PA6 provides scope/type lookup
- PA7 provides the procedural expression/call core
- PA10-PA14 lower the supported language subsets to LowIR
- PA15 extends the template layer with compile-time value arguments and explicit specialization

### Starter Kit

The starter kit contains:

- a `cppgm++.cpp` assignment entry point, linked to the editable compiler source
  in `../dev/cppgm++.cpp`
- the standard assignment `Makefile` and harness scripts
- the grammar for this assignment called `pa15.gram`
- an HTML grammar explorer of `pa15.gram` in the sub-directory `grammar/`
- a checked-in local test suite under `tests/`

Extend the `cppgm++` implementation you built in earlier assignments; the
initial scaffold is not reinstalled at this milestone.

Use the supplied reference tools to inspect example output. Tests compare
your compiler with the checked-in contract references.

### Input / Command-Line Arguments

The PA15 invocation is the unoptimized LowIR mode:

    $ cppgm++ --emit-lowir -O0 -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

Behaviour is undefined unless the command-line arguments match that shape, with
the same source-file ordering and `-o` relaxations as PA14. Preserve the earlier dump modes;
native driver behavior and optimized LowIR output are not required in PA15.

### Output Format

On success, `cppgm++` shall write LowIR text to `<outfile>` and exit
`EXIT_SUCCESS`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA15 extends the PA14 LowIR
subset only by making more of the source language lower into the already-defined LowIR
family. PA15 does not introduce a new output format.

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
It Enforces", is the one list). Exact textual LowIR matching is not a PA15
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

Tests compare your output with the checked-in references using the LowIR
comparison described in [the format guide](../pa8/lowir.md).

For each test case `x`:

- `cppgm++` is executed to produce `x.my`
- the exit status is recorded in `x.my.exit_status`
- `x.my` is compared against `x.ref`
- `x.my.exit_status` is compared against `x.ref.exit_status`

`make test` runs the checked-in local suite under `tests/`. The suite is split
by test role:

- `tests/spec/` contains N3485/spec-anchored PA15 metaprogramming tests. Each
  provided C++ language test in this directory starts with a leading comment of the
  form `// N3485 focus: 14.x.y [clause.name] ...` so a reviewer can find the
  governing text in `../doc/n3485.txt`.
- `tests/general/` contains broader metaprogramming tests that are useful for
  PA15 but are not one-rule spec probes.

The `make test` target runs both directories through the LowIR validator. For
successful tests, the validator checks the reference LowIR and your generated
LowIR for basic structural correctness, then compares the canonicalized LowIR
against the checked-in reference. For rejected tests, the exit status is the
checked result; exact diagnostic text is not checked.

PA15 is tested against generated LowIR text. That LowIR is intended to become
input for the later PA24 `lowir2native` backend, but that future native path is
not the PA15 grading contract.

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.t` runs one.

### Optional Student Test Ideas

When adding your own tests, useful PA15 themes include explicit specialization
ordering and visibility, integral non-type argument equivalence, type and
non-type parameter packs, `sizeof...`, pack expansions, dependent non-type
parameter types, and static data member specialization.

### PA15 Syntax Spec

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa15.gram`. The grammar defines accepted syntax only;
the PA15 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

As in the earlier assignments, that grammar defines accepted input syntax only. The output
format for `cppgm++` is specified by this README, PA8 `lowir.md`, and the checked-in
`.ref` files.

PA15 gives the following previously parsed forms semantic/code-generation
meaning:

- integral non-type template parameters such as `template<int N>`
- template parameter packs and pack expansions such as `template<class... Ts>`
  and `f(args...)`
- explicit specialization syntax such as `template<> int f<int>(int)` and
  `template<> struct Box<int> { ... }`

Passing PA14 is necessary but not sufficient for passing PA15: an input may be syntactically
valid for PA5-PA15 and still be outside the supported PA15 metaprogramming slice described
below.

A checked-in HTML grammar explorer for that grammar lives in `grammar/`. Treat
`pa15.gram` as the source of truth.

`pa15.gram` uses the same token vocabulary and the same extended BNF operators as
`../shared/source.gram`.

If this README and `pa15.gram` appear to disagree about source syntax, treat `pa15.gram`
as authoritative. If this README and PA8 `lowir.md` appear to disagree about LowIR syntax,
treat `lowir.md` as authoritative. If they disagree about the PA15 lowering slice, treat the
`Assignment Boundary` and `Out Of Scope` sections below as authoritative.

### Assignment Boundary

PA15 supports the following in addition to the PA14 subset:

- class templates whose parameters may now include type parameter packs,
  integral non-type parameters, and integral non-type parameter packs
- function templates whose parameters may now include type parameter packs,
  integral non-type parameters, and integral non-type parameter packs when the
  arguments are supplied explicitly
- a translation unit may instantiate dozens of distinct argument partitions
  for a function template with multiple parameter packs; every partition keeps
  its own argument-to-pack boundaries when specializations are reused
- pack expansions in supported declarations, direct calls, and instantiated
  body shapes
- integral constant-expression template arguments over the supported subset:
  - literals, including ordinary character literals
  - keyword literals `true` / `false`
  - id-expressions naming supported constant bindings
  - parenthesized expressions
  - unary `+`, unary `-`, `!`, `~`
  - binary arithmetic, shifts, comparisons, equality, bitwise, and logical operators
  - conditional `?:`
  - `sizeof...(parameter-pack)`
  - `sizeof(type-id)` and `alignof(type-id)`
  - supported cast expressions that fold to integral constant values
- explicit specialization of supported class templates
- explicit specialization of supported function templates; an explicit
  specialization of a function template states `inline_hint=yes` only when it
  is itself declared `inline` or `constexpr`, whatever the template's
  declaration says (14.7.3p12)
- an explicit specialization declared after uses of its primary template
  takes effect for the uses that follow it (14.7.3p6), in the supported class
  and function template cases
- constant-valued template bindings over the supported subset, including class-scope
  `static const` / `static constexpr` members and other ordinary metaprogramming helper
  bindings that feed lookup, template arguments, or `static_assert`
- dependent qualified type/value lookups at the practical level needed by the supported
  metaprogramming subset
- `static_assert` declarations whose condition is in the supported integral constant subset,
  including conditions that remain template-dependent until instantiation
- inline virtual members required by a concrete class-template vtable are
  instantiated even without a direct source call; unrelated non-virtual member
  bodies remain demand-driven

Within this milestone, PA15 should produce valid LowIR for ordinary metaprogramming code
over the supported PA14 language subset. That LowIR is intended to be accepted
by the later PA24 `lowir2native` backend for the supported cases.

### Out Of Scope

The following are explicitly out of scope for PA15:

- partial specialization
- pointer, reference, member-pointer, class-type, and other non-integral
  non-type template parameters
- SFINAE and substitution-failure candidate dropping
- full standard-conforming two-phase lookup
- constexpr function evaluation
- function-template deduction of non-type arguments
- full function-template deduction and partial ordering
- alias templates and variable templates
- hosted/vendor-only template traits and intrinsics
- template metaprogramming that depends on unsupported PA10-PA14 language features

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stages are:

- PA16: complete the language-level constant-evaluation model over the existing LowIR path
- PA17 and PA18: finish the remaining template specialization, deduction, substitution, and
  SFINAE work on top of that constant-evaluation engine
- PA19: check that the individual template features from PA14, PA15, PA17, and
  PA18 compose without breaking their basic behavior
- PA24: retarget the settled LowIR language surface to the real native backend

So PA15 should leave behind:

- a stable template/metaprogramming semantic layer
- ordinary instantiated declarations ready for LowIR lowering
- no PA15-specific output representation beyond LowIR itself

### Design Notes (Non-Normative)

PA15 should extend the existing template machinery, not replace it.

The same monotonic-extension rule applies here:

- PA15 should add metaprogramming behavior only when the source actually uses the supported
  PA15 feature set
- it should not perturb PA14 outputs for programs that remain entirely within the PA14
  subset
- in practice, packs, non-type template arguments, explicit specialization, and
  `static_assert` should stay on-demand rather than eagerly changing the
  behavior of ordinary earlier programs that do not use those features

Useful intermediate representations include:

- template parameters that distinguish type, pack, and integral value slots
- template arguments that carry canonical constant values rather than only source text
- explicit-specialization tables that plug into the existing instantiation machinery
- a specialization lookup step that runs before instantiation so late visible
  specializations replace stale primary-template instantiations in the supported
  cases
- compile-time constant bindings that can be reused by both `static_assert` and template
  argument resolution
