## CPPGM Programming Assignment 19 (`cppgm++ --emit-lowir`)

### Overview

PA19 is the template integration assignment. Now that you have implemented the
individual template features in PA14, PA15, PA17, and PA18, this PA checks that
they work together in realistic combinations. These tests are intentionally
complicated: they are meant to uncover edge cases where a feature works alone
but loses typed information, chooses the wrong specialization, instantiates too
early, or fails during lowering when combined with another template mechanism.

PA19 does not introduce a new isolated template feature. It is a composition
surface over the earlier template assignments.

PA19 still produces LowIR. It does not introduce a new output format.

### Prerequisites

You should complete Programming Assignment 18 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6-PA7 semantic foundation
- the PA10-PA18 LowIR lowering path
- the PA8 LowIR contract
- the PA14-PA18 template machinery
- the PA16 constant-evaluation layer

### Starter Kit

The starter kit contains:

- a `cppgm++.cpp` assignment entry point, linked to the editable compiler source
  in `../dev/cppgm++.cpp`
- the standard assignment `Makefile` and harness scripts
- the PA19 template-integration test suite under `tests/`

Extend the `cppgm++` implementation you built in earlier assignments; the
initial scaffold is not reinstalled at this milestone.

Use the supplied reference tools to inspect example output. Tests compare
your compiler with the checked-in contract references.

### Input / Command-Line Arguments

The PA19 invocation is the unoptimized LowIR mode:

    $ cppgm++ --emit-lowir -O0 -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

Behaviour is undefined unless the command-line arguments match that shape, with
the same source-file ordering and `-o` relaxations as the earlier source-to-LowIR
milestones. Preserve PA7's dump mode. Native driver behavior and optimized
LowIR output are not required in PA19.

### Output Format

On success, `cppgm++` shall write LowIR text to `<outfile>` and exit
`EXIT_SUCCESS`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA19 extends the
PA18 lowering surface only by requiring previously introduced template features
to compose through the already-defined LowIR family.

LowIR top-level declaration/definition order is a presentation convention, not
a dependency order. Reference outputs and canonical dumps use the order defined
in `../pa8/lowir.md`: `declare global`, `declare function`, `global`, then
`function`, but the relaxed LowIR comparison canonicalizes top-level entries
before comparison. Your output must still be repeatable for the same inputs;
`../pa8/lowir.md` defines the canonical reference presentation and notes where
internal LowIR symbol names are only a presentation tie-breaker.

Your output must also preserve order-sensitive LowIR regions when they are
present: instruction order inside blocks, item order inside structured globals,
vtable slot order, and action order inside generated initialization,
finalization, constructor, destructor, and cleanup bodies.

The test harness checks that the generated LowIR is well formed and matches the
checked-in `.ref` files after canonicalizing presentation details that are not
part of the assignment contract. Exact textual LowIR matching is not a PA19
grading requirement.

### Error Handling

If an error occurs during preprocessing, tokenization, parsing, semantic
analysis, or LowIR generation, `cppgm++` shall `EXIT_FAILURE`.

The output file is not required to be meaningful on failure. Diagnostics are not
part of the grading contract.

### Standard Output / Error

Standard output and standard error are ignored for automated testing of
`cppgm++`.

You are free to use them for debugging, tracing, or diagnostic messages.

### Testing

PA19 tests live under `tests/`. The suite is split by test role:

- `tests/spec/` contains N3485/spec-anchored integration cases whose important
  behavior is still tied to a specific standard template rule.
- `tests/general/` contains broader generic-program examples that combine
  multiple template mechanisms.

The `make test` target runs both directories through the LowIR validator. For
successful tests, the validator checks the reference LowIR and your generated
LowIR for basic structural correctness, then compares the canonicalized LowIR
against the checked-in reference. For rejected tests, the exit status is the
checked result; exact diagnostic text is not checked.

The clusters are organized by feature combination:

- `100`: dependent-name and entity interactions that do not fit a narrower
  later cluster
- `200`: deduction, partial ordering, non-deduced context, and braced-init
  deduction combinations
- `300`: SFINAE, substitution, detector idiom, and no-eager instantiation
  combinations
- `400`: pack, member-template, template-template-parameter, alias-template, and
  variable-template compositions
- `500`: library-shaped end-to-end reducers without hosted or builtin-trait
  dependencies

When working through PA19 failures, keep the earlier template assignments
passing. A PA19 fix should not relax, special-case, or regress the basic
functionality already covered by PA14, PA15, PA17, and PA18. Before treating an
integration fix as complete, run the earlier template PAs and PA19 through the
report harness from the repository root:

    $ make test-report ACTIVE_TEST_REPORT_PAS='pa14 pa15 pa17 pa18 pa19'

PA19 ships no self-checking controls: unlike PA10–PA18 and PA20–PA23 it has
no `tests/controls/`, and every PA19 case is graded by its LowIR.

### PA19 Syntax Boundary

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa19.gram`. The grammar defines accepted syntax only;
the PA19 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

### Optional Student Test Ideas

When adding your own tests, useful PA19 themes include dependent alias expansion
inside deduction, member-template calls through dependent owners, pack expansion
through SFINAE helpers, detector idioms that rely on earlier alias or partial
specialization machinery, and function-template partial ordering where more than
one template feature is needed for the selected result.

### Assignment Boundary

PA19 owns integration among already-introduced template features, including:

- dependent names combined with alias, variable, partial-specialization, or
  deduction behavior
- imported constants retained as defaults in nested member templates when the
  enclosing template is instantiated
- function-template deduction combined with packs, non-deduced contexts,
  explicit arguments, conversion templates, or constructor templates
- SFINAE and substitution behavior combined with alias templates, partial
  specializations, member templates, packs, and detector idioms
- no-eager-instantiation timing in realistic dependent template bodies
- dependent function and constructor default arguments are materialized only after overload
  selection, so an invalid default on an unselected candidate does not reject the call
- library-shaped reductions that do not require hosted headers, vendor
  builtins, or later language features

The course includes the later
[CWG1315 resolution](https://cplusplus.github.io/CWG/issues/1315.html) for
dependent non-type arguments in partial specializations: an argument expression
may involve a specialization's template parameter when that parameter is
deducible outside non-deduced contexts. Do not reject the declaration solely
because the expression is more than a simple parameter name, as literal N3485
14.5.5p8 would require. Substitution still checks the dependent expression.
In `tests/general/300-incomplete-sizeof-partial-specialization-sfinae.t`,
`T` is deduced from the first argument; `sizeof(comparator<T>)` then fails
because the class is incomplete. That substitution failure removes the
`has_comparator` specialization instead of rejecting the program, and
`lazy<X>::value()` selects the fallback and returns zero. This selected
compatibility rule does not bring other post-C++11 features into scope.

### Out Of Scope

The following are explicitly out of scope for PA19:

- new isolated template features not already introduced by PA14, PA15, PA17, or
  PA18
- `std::initializer_list` library semantics and initializer-list overload
  machinery
- member-pointer template behavior that depends on later member-pointer support
- hosted/vendor-only extensions that happen to use templates
- post-C++11 template-language features
- backend/toolchain ownership that belongs to the later native and toolchain
  milestones

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stage is PA20, which extends the source-to-LowIR path with
the first broad ordinary-language closure slice.

So PA19 should leave behind:

- a complete standard template semantic layer whose individual features compose
  cleanly
- instantiated declarations that lower through the ordinary LowIR path without
  template subset special-casing
- no remaining "template features work alone but not together" gap before later
  backend, ABI, and hosted toolchain work

### Design Notes (Non-Normative)

The useful shape for PA19 is not another special-case layer. The same typed
template declarations, template arguments, substitution results, deduction
bindings, and deferred-instantiation records from PA14-PA18 should be threaded
through the combined cases.

When a PA19 test fails, first look for data that was lost between those
subsystems: an alias expansion that discarded a dependent owner, a pack binding
that became text-only, a substitution failure promoted into a hard diagnostic,
or an instantiation record forced before the template arguments were known.
