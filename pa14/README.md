## CPPGM Programming Assignment 14 (`cppgm++ --emit-lowir`)

### Overview

PA14 adds the first usable template tier on top of the completed PA13 procedural/object/
polymorphic compiler. It extends PA13 with:

- function templates
- class templates
- type template parameters
- default type template arguments
- dependent names and current-instantiation lookup in supported template bodies
- basic function-template argument deduction for direct calls
- on-demand template instantiation for the supported class/function cases
- template-backed operator overloads where the non-template PA7-PA13 machinery
  already exists

### Prerequisites

You should complete Programming Assignment 13 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6 declarator/type model
- the PA7 call-resolution layer
- the PA10-PA13 LowIR lowering path
- the PA8 LowIR contract
- the PA11-PA13 class metadata, constructor/destructor machinery, and polymorphic lowering

The intended direction is:

- PA5 provides syntax
- PA6 provides scope/type lookup
- PA7 provides the procedural expression/call core
- PA10 lowers the procedural subset
- PA11 adds the basic non-virtual object model
- PA12 adds the non-polymorphic value-semantics layer
- PA13 extends that object model with scoped polymorphism
- PA14 adds first-tier templates on top of that existing semantic/codegen stack

### Starter Kit

The starter kit contains:

- a `cppgm++.cpp` assignment entry point, linked to the editable compiler source
  in `../dev/cppgm++.cpp`
- the standard assignment `Makefile` and harness scripts
- the grammar for this assignment called `pa14.gram`
- an HTML grammar explorer of `pa14.gram` in the sub-directory `grammar/`
- a checked-in local test suite under `tests/`

Extend the `cppgm++` implementation you built in earlier assignments; the
initial scaffold is not reinstalled at this milestone.

Use the supplied reference tools to inspect example output. Tests compare
your compiler with the checked-in contract references.

### Input / Command-Line Arguments

The PA14 invocation is the unoptimized LowIR mode:

    $ cppgm++ --emit-lowir -O0 -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

Behaviour is undefined unless the command-line arguments match that shape, with
the same source-file ordering and `-o` relaxations as PA13. Preserve the earlier dump modes;
native driver behavior and optimized LowIR output are not required in PA14.

### Nondependent template bases

A base class that does not depend on a template parameter is resolved and
validated when the class template is defined, even if no specialization of the
derived template is ever used.  It must already name a complete, non-union,
non-final class that is accessible from the class-template definition.  A
friend class-template declaration grants that access before any concrete
specialization exists.  Selecting a class-template specialization as a fixed
base also instantiates and validates that specialization's definition; an
error in its body cannot be hidden merely because no derived specialization is
used.  A genuinely dependent base remains deferred until a concrete
specialization is formed.

### Output Format

On success, `cppgm++` shall write LowIR text to `<outfile>` and exit
`EXIT_SUCCESS`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA14 extends the PA13
template-free object/polymorphism subset of that IR with the instantiated lowering needed by
this milestone.

PA14 writes a single concatenated LowIR program consisting of:

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

Template instantiation in PA14 should produce ordinary instantiated declarations which then
lower through the existing PA10-PA13 LowIR conventions.

The test harness checks that the generated LowIR is well formed and matches the
checked-in `.ref` files after canonicalizing presentation details that are not
part of the assignment contract: the names of temporaries, slots, blocks and
parameters, the order of slot declarations and of top-level entries, and the
spelling of literals (`../pa8/lowir.md`, "What The Comparison Absorbs And What
It Enforces", is the one list). Exact textual LowIR matching is not a PA14
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

- `tests/spec/` contains N3485/spec-anchored first-tier template tests. Each
  provided C++ language test in this directory starts with a leading comment of the
  form `// N3485 focus: 14.x.y [clause.name] ...` so a reviewer can find the
  governing text in `../doc/n3485.txt`.
- `tests/general/` contains broader first-tier generic-program tests that are
  useful for PA14 but are not one-rule spec probes.

The `make test` target runs both directories through the LowIR validator. For
successful tests, the validator checks the reference LowIR and your generated
LowIR for basic structural correctness, then compares the canonicalized LowIR
against the checked-in reference. For rejected tests, the exit status is the
checked result; exact diagnostic text is not checked.

PA14 is tested against the generated LowIR text. That LowIR is intended to
become input for the later PA24 `lowir2native` backend, but that future native
path is not the PA14 grading contract.

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.t` runs one.

### Optional Student Test Ideas

When adding your own tests, useful PA14 themes include class/function template
instantiation, default type template arguments, dependent versus non-dependent
lookup, current-instantiation names, and direct function-template calls. Keep
any such tests within the PA14 boundary below; parameter packs,
template-template parameters, member/friend templates, non-type template
arguments, partial specialization, full deduction, and SFINAE behavior belong
to later assignments.

### PA14 Syntax Spec

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa14.gram`. The grammar defines accepted syntax only;
the PA14 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

As in the earlier assignments, that grammar defines accepted input syntax only. The output
format for `cppgm++` is specified by this README, PA8 `lowir.md`, and the checked-in
`.ref` files.

Template declarations, template-parameter clauses, common template-id syntax,
and later-template syntax such as non-type template parameters were already
preserved by PA5; PA14 is the first milestone that gives a supported
type-parameter subset of template syntax semantic/code-generation meaning.

Passing PA13 is necessary but not sufficient for passing PA14: an input may be syntactically
valid for PA5-PA13 and code-generation-valid for PA13 and still be outside the PA14
template slice described below.

A checked-in HTML grammar explorer for that grammar lives in `grammar/`. Treat
`pa14.gram` as the source of truth.

`pa14.gram` uses the same token vocabulary and the same extended BNF operators as
`../shared/source.gram`.

If this README and `pa14.gram` appear to disagree about source syntax, treat `pa14.gram`
as authoritative. If this README and PA8 `lowir.md` appear to disagree about LowIR syntax,
treat `lowir.md` as authoritative. If they disagree about the PA14 lowering slice, treat the
`Assignment Boundary` and `Out Of Scope` sections below as authoritative.

### Assignment Boundary

PA14 supports the following in addition to the PA13 subset:

- class templates whose parameters are:
  - type parameters
- function templates whose parameters are:
  - type parameters
- default template arguments for the supported type-parameter forms, including
  defaults that refer to earlier parameters in the same template head
- dependent type/value names in the supported declaration and expression forms
- current-instantiation lookup in the supported class-template cases
- `typename` and `template` disambiguators where they are needed by the PA14
  dependent-name subset
- explicit template-id use for supported class templates and function templates
- explicit type arguments using the ordinary PA6 declarator/type forms,
  including function types
- ordinary lookup and using-declaration behavior for supported templates,
  including repeated using-declarations of the same template and preservation
  of an ordinarily visible function template when hidden non-template friends
  share its name, and replay of a dependent-base using-declaration that names an
  enumerator used by another enumerator; ordinary lookup must retain ambiguity
  when distinct declarations are introduced by using-directives
- basic template argument deduction for direct supported function-template calls
  from ordinary argument types, without function-template partial ordering or
  SFINAE
- on-demand instantiation of the supported class-template and function-template cases:
  a member's body is instantiated only when it is odr-used (14.7.1p1-p2), so an
  unused conversion function's body and a dependent defaulted special member
  stay uninstantiated, and a program whose out-of-line destructor follows the
  class-template use that needs it is accepted
- a static data member named only in an unevaluated operand, such as the operand
  of `sizeof`, does not demand storage or instantiate its out-of-class definition;
  an evaluated value use or address use continues to demand the required definition
- instantiated specializations reuse supported PA12 rvalue-reference return
  paths and PA13 virtual-destructor lifetime without changing their value
  category or object-lifetime behavior
- distinct local-class identities for separate function-template
  specializations
- declaration-owned template-parameter scopes across nested instantiations and
  out-of-class member-definition bodies, including definition-time rejection
  of a declaration that redeclares an active template parameter even when the
  member is never instantiated
- definition-time semantic checking of unused supported function-template and
  qualified inline member bodies, including ordinary block and condition
  scopes and the distinction between type names and value names; class bit-field
  members remain valid value expressions in those bodies
- qualified class-template-ids in function declarators are parsed in their own
  declaration context rather than being captured by an unrelated function
  template with the same unqualified name
- class-template instantiation preserves the class-scope rule that a name which
  has become a typedef-name cannot be redefined by another typedef declaration
- compatible function-template declarations and definitions in either order
- a specialization of an `inline` function template, or of a member defined in
  its class template, is an inline function and states `inline_hint=yes`; one
  of a function template not declared inline does not (explicit
  specializations, which PA15 adds, follow their own declarations)
- a template's `noexcept(e)` is evaluated for each specialization, which states
  `unwind=no` when its `e` is true (`f<int>` of
  `template<class X> int f(X) noexcept(sizeof(X) == 4)`, not `f<char>`)
- a nested class with a fixed base does ordinary base lookup, and a local
  class in a function template classifies its own base as dependent or not on
  its own, whatever the enclosing template's bases are
- out-of-class definitions of nested classes declared inside the supported class templates,
  when those nested classes stay within the already supported PA11-PA13 class/value/
  polymorphic machinery
- distinct nested types from different class-template specializations remain distinct in
  overload resolution, and a dependent local type alias used by `new` resolves to the
  concrete specialization before constructor selection
- ordinary PA5 function declarator forms, including trailing return types, on the supported
  function-template cases
- instantiated specialization names that then participate in the ordinary PA11-PA13
  class/method/codegen machinery
- template-backed overload participation where the non-template PA7-PA13 machinery already
  exists, including function-template operator overloads
- ordinary member typedef hiding and injected-name lookup during re-entrant
  class-template instantiation

Within this milestone, PA14 should produce valid LowIR for ordinary generic code over the
supported PA13 subset. That LowIR is intended to be accepted by the later PA24
`lowir2native` backend for the supported cases.

### Out Of Scope

The following are explicitly out of scope for PA14:

- template-template parameters
- template parameter packs and pack expansions
- member templates and friend templates
- semantic support for non-type template parameters and non-type template
  arguments
- partial specialization
- explicit specialization
- full standard two-phase lookup
- function-template partial ordering
- substitution-failure candidate dropping and SFINAE
- full `constexpr` evaluation
- alias templates and variable templates
- hosted/vendor-only template traits and intrinsics
- template-aware virtual dispatch beyond ordinary instantiated class reuse
- templates whose definitions rely on unsupported PA11-PA13 class/value/polymorphic features

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stage is PA15, which adds the first practical metaprogramming layer on top
of the basic PA14 template machinery:

- integral non-type template parameters and arguments
- explicit specialization of supported class/function templates
- integral constant-expression template arguments
- `static_assert`-style metaprogramming support

So PA14 should leave behind a clean first-tier instantiation layer rather than trying to
solve the full template language at once. Partial specialization, SFINAE
metaprogramming, and full `constexpr` evaluation remain later work.

### Design Notes (Non-Normative)

The important point is to add templates as an extension of the existing PA11-PA13 language
behavior rather than building a separate generic-only compiler with different rules. Whether
that reuse happens through shared code, shared data structures, or a careful
reimplementation is up to you.

The same monotonic-extension rule applies here:

- PA14 should add template behavior only when the source actually uses the supported
  template feature set
- it should not perturb PA13 outputs for programs that remain entirely within the PA13
  subset
- in practice, template lookup and instantiation should stay on-demand rather than eagerly
  changing the behavior of ordinary earlier-milestone programs that do not use the PA14
  template subset

Useful intermediate representations include:

- explicit template declarations stored separately from ordinary instantiated declarations
- template-parameter scopes that can be rebound during instantiation
- instantiated class/function records that reuse the ordinary PA11-PA13 metadata/lowering
- typed template arguments and bindings rather than source-text template replay
- a clear separation between:
  - parsing template syntax
  - collecting template declarations
  - deducing or resolving template arguments
  - instantiating ordinary specialized declarations

Instantiation decides much of the front end's speed on real code, where one
header instantiates thousands of specializations. Cloning a demanded
dependent body and checking the clone is a sound way to instantiate; so is
sharing the non-dependent parts between instantiations, provided a shared
node costs the same to read as an ordinary one. What does not scale is
re-parsing or replaying source text per instantiation, or reaching every
shared node through an extra lookup on each read.

Find a specialization through a table keyed by its template and its
canonical argument list, with each argument an interned type or constant,
so that asking for `vector<int>` again is one probe. Scanning a template's
specializations and comparing their arguments one by one grows with the
number of instantiations, which is what a standard library header has most
of. If expressions carry their analyzed facts on the nodes, as PA5
suggests, each instantiation needs its own copy of the dependent body; that
is a sound trade, but it puts the copy on the front end's hottest path, so
keep it a plain walk over the nodes.
