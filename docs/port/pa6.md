# PA6 port ledger

Stage base commit: `70e745588987998ee1e5bde80b3861541eab753d`
Last audited commit: `3140a2bbab5d8a42bfdba0259d0ee0a567439d4f` (accepted partial checkpoint audit).
Audit entries advance this field to the reviewed implementation revision;
Ralph retains stage advancement and goal ownership.

## Declaration checkpoint contract and ownership

This is a coherent partial PA6 implementation checkpoint, not PA6 completion
or advancement. The incoming clean checkout returned status 86 for
`--emit-types` in all 121 fixed cases. The preceding interrupted turn had no
retained implementation progress at that boundary; this continuation implements
the missing semantic layer instead of repeating the status report.

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, `docs/rust-clone-plan.md`,
`pa6/README.md` and `pa6/scopes-and-types.md`. Consulted the frozen C++ owners at
`/home/vishvananda/work/references/v5rust-cpp`, revision
`0fe8e3b4bde9929c2419fb7906028df83bb17de5`:

- `dev/cppgm++.cpp`: emit-mode selection and types source loop.
- `dev/src/sema/analyzer.cpp`: graph creation, dispatch, namespace reopening,
  aliases, using declarations and directives.
- `analyzer_types.cpp`, `type_table.cpp`, `lookup.cpp`: specifiers, declarators,
  cv/reference/array formation, canonical parameters and lookup.
- `analyzer_declarations.cpp`, `analyzer_classes.cpp`, `analyzer_bodies.cpp`:
  aliases/entities/definitions, class scopes/completion, parameter/body scopes,
  name-only expression traversal and implicit constructor demand.
- `semantic_dump.cpp`, `semantic_types.h`, `semantic_graph.h`,
  `entity_records.h`: source-facing dump and typed semantic records.
- `constant_evaluator.cpp`: integral/character literal conversion only;
  the general evaluator is not translated yet.

Rust owners are `dev/src/sema/{graph,types,lookup,specifiers,declarators,
declarations,analyzer,expressions,dump}.rs`. `dev/src/driver/ast.rs` and
`dev/bin/cppgm.rs` share the existing source loop through a typed `EmitMode`.
The source map records each partial C++ responsibility separately.

The driver parses each unit once. Semantic analysis borrows the original flat
AST, including structured names and decoded phase-7 literal facts. Private
semantic stores issue checked compact `TypeId`, `ScopeId`, `EntityId` and
`BindingId` identities. Scope/name hash indexes are separate from source-order
binding/child lists. Dense arrays retain resolved names, declarator types,
type-id facts and declaration bindings against original AST IDs. Canonical
function types adjust parameters recursively; source types remain available to
the dump. A variable's declarations share its completed entity type.

Type interning/canonical memos belong to one unit. Canonicalization and array
qualification use explicit worklists; semantic expression and dump traversals
also avoid native recursion along long chains. Lookup allocates only for the
visited namespace edges, never all unit scopes, and caches no invalidatable
lookup result. Member bodies wait for outermost class completion. Evaluated
empty class construction demands one implicit constructor scope; unevaluated
operands and class-object initializer contexts preserve the C++ suppression
rules. Later lowering, optimization, native generation and register allocation
are not implemented or modified by this checkpoint.

## Coherent behavior groups

Implemented and validated on the fixed cases and ownership regressions:

- `--emit-types`, per-file isolation, output wrappers and located diagnostics;
  missing semantic groups remain status 86 rather than language rejections.
- Namespace scopes/reopening, namespace aliases, unnamed/inline lookup edges,
  using directives/declarations, qualified and lexical type/value lookup,
  shadowing, directive cycles and nearest common namespace nomination.
- Fundamental specifier combinations; typedef/alias identity; cv qualification,
  pointers/references, alias reference collapsing; array/function declarator
  precedence and source versus canonical parameter adjustment, including nested
  function types, sole void and variadic lists.
- Positive integral/character literal bounds; incomplete-array completion by
  compatible redeclaration or parsed braced/string initializer; shared completed
  variable types, duplicate definitions and incomplete object rejection.
- Named classes/forwards and compatible class keys; elaborated tag lookup and
  nearest namespace/block declaration home; injected names, complete-class
  member bodies; function/block scopes with declared parameter types.
- Types-mode expression name checks and exact operand visitation order;
  evaluated versus unevaluated contexts, callee allowances and deduplicated
  implicit default-constructor scope demand.

First divergent boundary is now integral constants/enumeration/template/
anonymous-type/exception-specification analysis, rather than the driver.
Every remaining fixed failure exits 86 with its owning missing group;
`failure-clusters.json` groups exact identities by the first missing C++/Rust
responsibility. There is
no remaining normal diagnostic/output mismatch on the supported fixed slice.
The entire assignment remains incomplete: fixture coverage alone does not prove
all declaration invariants, lookup forms or syntax contexts are translated.

Additional production dependencies beyond these fixed failures remain: full
entity owner/linkage facts, default arguments/prototype scopes, block extern
association, overload and incompatible function redeclaration rules, exhaustive
specifier/member declaration checks, qualified injected-name rules, complete
class layout, inheritance, special members/bit-fields, member initializer
completion events, selection/iteration/handler/ambiguous statement scopes,
runtime new-array bounds and extended/predefined semantic types. Do not use
this checkpoint as evidence of PA7 expression typing or later code generation.

## Declaration checkpoint evidence

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/port/pa6-declarations/`.
Candidate implementation commit: `8096793b39602b581016324c8143b1163e923213`.
Candidate release `cppgm` SHA-256: `c34fc6acae6544f405605e20cb601c4dce56e76dc861f1caf207087ebc6ecbcd`.
Frozen C++ tool SHA-256: `9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.
The final checkout/commit receipt is `handoff.json` in that artifact directory.

The fixed PA6 manifest still executes 121 identities. Differential progression
was 0/121 at turn start, then 59/121, 65/121 and 66/121. All intermediate passing
identities remain passing: failures reduced from 121 to 55, without adding or
removing a manifest case. The protected preservation gate reports 0/0 because
there were no turn-start PA6 passes; the intermediate identity comparison is
also recorded in `stage-final-summary.json`.

| Command | Result | Evidence under artifact root unless absolute |
| --- | --- | --- |
| `make build` | PASS | release binary/hash; build elapsed time retained in invocation evidence |
| `make test-pa6` (required `clone_check.py stage pa6`) | FAIL, 66/121 matching, 55 missing groups | `stage-final.log`, `stage-final-summary.json`; raw per-case traces at `/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa6/<case-id>/trace.jsonl` |
| `make course-pa6` | FAIL, 66/121, failure marker retained | `course-final.log`; `/home/vishvananda/work/private/v5rust/artifacts/clone/course/pa6/test/{counts.txt,course-result.json}` |
| `clone_check.py preflight pa6` | PASS | `preflight.log` |
| `clone_check.py prior pa6` | PASS, PA1–PA5 differential and course lanes | `prior-final.log`; `/home/vishvananda/work/private/v5rust/artifacts/clone/course/paN/test` |
| `clone_check.py preserve pa6` | PASS, 0/0 turn-start PA6 identities | `preserve.log` |
| `make lint` (`clone_quality.py`) | PASS: rustfmt, strict Clippy, debug/release unit suites, source audit | `quality-final.log`; seven semantic ownership/invariant regressions added |
| `make audit-rust` | PASS, 97 files, 0 findings | `audit-final.log` |

The PA6 course target internally reports exit 0 while retaining a failure marker;
the protected course checker correctly returns failure. This is not an
oracle/harness defect and is not treated as an expected-failure exception.
Full PA6 success is still required for stage advancement.

Raw failures retain stdout/stderr and both generated outputs in each trace's
`evidence` directory; deterministic bytes are compared without rewriting them.
`probes.jsonl` contains seven protected adapter probes (with earlier measurement
runs also retained), and `measure.py` reproduces the supported workloads.
A fixed 1,000-nested-class input under a 1 MiB initial stack qualifies at a
shared 16 MiB stack with both exits 0, identical streams and identical dump
hashes. Judge the receipt's `match` and `stack_qualification.passed`;
`nested-summary.json` and its qualification evidence retain the raw attempts.
No native cutoff or diagnostic position was calibrated or used as a language
result. The genuine typed byte-budget guard remains active.

Source responsibility review: semantic modules follow graph/type/lookup/
declarator/declaration/expression/dump ownership rather than C++ class files or
mechanical chunks. No dependencies, unsafe/FFI code, compiler subprocess
fallbacks, source reparsing, fixture recognition, environment reads under
`dev/src/`, protected fixtures/checks or oracle edits were introduced. Typed
front-end graph lifetime is one source unit; downstream backend lifetimes and
optimizer behavior are unchanged. Audit the incomplete groups explicitly
before increasing the supported contract.

## Declaration checkpoint hot-path costs

`measurements.json` records three timed runs per implementation at 1,000,
4,000 and 16,000 declaration/function groups, both flat and namespace-scoped.
Each group contains an alias, pointer variable, reference parameter, local
array and return. The protected adapter verifies full output identity before
measurement. Inputs, dumps and executables have SHA-256 receipts. Wall times
include process startup and type-dump I/O; RSS comes from `/usr/bin/time`.
These are recorded costs, not equality or PA35 optimization requirements.

| Shape | Groups | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| flat | 1000 | 17.33 / 9288 | 45.04 / 8736 |
| flat | 4000 | 64.71 / 21988 | 123.90 / 19224 |
| flat | 16000 | 235.85 / 69636 | 454.27 / 66768 |
| namespaces | 1000 | 17.86 / 9336 | 43.55 / 8748 |
| namespaces | 4000 | 63.28 / 22020 | 130.64 / 20620 |
| namespaces | 16000 | 252.80 / 72376 | 488.50 / 72088 |

## Integral constant and enumeration checkpoint

Implementation entry HEAD: `c2c98818abfecf5ecc41b10fafcffd7a88511932`.
The stage base and last-audited fields above remain unchanged. The preceding
retained goal turn is classified as progress: the declaration implementation
and validation commits are present, with 66 fixed-manifest identities passing.
This entry is the next coherent partial PA6 implementation handoff; it does
not declare PA6 complete or advance an audited checkpoint. `handoff.json`
under the artifact root below records the committed implementation HEAD.

Consulted the frozen `constant_evaluator.{cpp,h}`, `integral_arithmetic.{cpp,h}`,
`analyzer_classes.cpp` enumeration/naming entries, `analyzer.cpp` assertion
entry, `analyzer_types.cpp` bounds/parameter-object/prototype entries,
`analyzer_declarations.cpp` object publication/conflict rules, `layout.cpp`
scalar/array/enum queries and `semantic_dump.cpp` declaration/value spellings.
The source map assigns these responsibilities to
`dev/src/sema/{constants,integral,queries,enumerations}.rs` and the existing
analyzer, declarator, declaration, graph, type and dump modules.

Completed related behavior groups and their production invariants:

- Typed 64-bit integral, boolean and character values: target sign extension,
  conversion, promotion/rank/common type, signed undefined-operation rejection,
  unsigned wrap, division/remainder and shifts including CWG 1457's sign-bit
  left shift. Host arithmetic cannot define target overflow.
- Short-circuit logical operands retain unevaluated type/name checking while
  suppressing optional arithmetic nonconstancy; conditional arms acquire their
  common type. Explicit integral/enum casts and decoded literal string-element
  reads consume the existing AST and phase-7 byte facts.
- Object constants use explicit `None`, `Evaluating`, `Value(Constant)` state
  on the owning entity. Nonvolatile const integral/enum objects and eligible
  const references record their initializer's value at declaration time;
  reading a referenced variable reuses that entity's stored constant.
  `constexpr` qualification already belongs to declaration formation.
- Array bounds and assertions consume typed values with distinct optional
  nonconstancy versus fatal semantic diagnostics. Located rejection spelling
  and ordering match, including overflow and scoped-enum conversion rules.
- Named/scoped/fixed/opaque enums, underlying-type agreement, qualified
  completion, sequential enumerators, widening/default values, representability
  and range-selected underlying types. An enumerator has its integral type
  during the definition and the finished enum identity after the brace.
  Unscoped qualification indexes only that enum's enumerators, even where a
  value of the same name exists elsewhere. Aliases and using bindings converge
  on the same entity and value; class/enum/value-kind conflicts are checked.
- Anonymous enum labels count only declarations that need generated names;
  unnamed type bindings stay hidden unless the declaration gives them a name.
  Qualified scoped definitions retain their original typed declaration-label
  owner for enum dumps and imported enumerator spellings.
- `sizeof`/`alignof` scalar, pointer, reference, array and enum queries and
  declared-entity/parenthesized `decltype`. Scalar queries use declared types,
  never object construction. Array multiplication records overflow while
  descending and diagnoses an incomplete element before size overflow.
  Hidden earlier-parameter scopes are created on demand for a type that may
  name values; arrays/functions adjust to pointers while parameter object cv
  and function source types remain distinct from canonical signatures.

Architecture/source responsibility review: constants retain compact `TypeId`
and `EntityId` identities; enum metadata names its owner scope. Dense constant
facts remain indexed by original `ExprId`; dense enum-declaration facts are
accessed through private binding-store APIs. The sparse qualified display map
borrows structured AST names and is presentation-only. The analyzer borrows
cursor-owned decoded literal bytes only for its unit lifetime; the finished
graph owns small values and never retains those bytes. A reusable binary-spine
worklist has nested base/truncate ownership, including failure cleanup. No
query allocates or scans data for unrelated declarations. No negative query
cache or global invalidation was added. Four new unit regressions exercise
numeric boundaries, enum/using identity, source-versus-object parameter types
and decoded string signedness; the preexisting missing-group test now targets
the still-unimplemented template owner.

Class layouts, complete-class exception specifications, default/prototype
placeholder environments, templates, anonymous classes/unions and special
members remain separate dependencies. General expression/member typing,
interpreted pointer/floating/wide/object values and instantiation semantics
remain later owners. This checkpoint does not certify those paths. No code
generation, optimizer or register allocator behavior was changed; no unsafe,
FFI, subprocess implementation, source reparsing, environment reads in phases,
new dependencies or fixture/checker/oracle/Ralph-state edits were introduced.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/port/pa6-constants/`.
Candidate release `cppgm` SHA-256:
`598ef71a954ad9496efd7681bb1ba8834b4ecd0c6bbc23d6e95f73a0ecc8bcd5`.
The frozen C++ SHA-256 remains
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

The fixed manifest remains exactly 121 identities. Passing cases increased
from 66 to 105, with failures reduced from 55 to 16 and all 66 incoming passes
preserved. `stage-start.log` and `stage-final-summary.json` record the exact
incoming, new, preserved and remaining identities. Added unit/probe coverage
is not used to count stage progress. `failure-clusters.json` records each
remaining case's earliest missing owner; every remaining invocation exits 86,
with raw diagnostic/output evidence retained rather than treating that as a
successful language rejection.

| Command | Result | Evidence under this checkpoint artifact root |
| --- | --- | --- |
| `make build` | PASS | `build.log`, 9.63 s release build; binary hash above |
| `make test-pa6` (`python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6`) | FAIL, 105/121 exact cases, 16 missing groups | `stage-final.log`, `stage-final-summary.json`; raw traces at `/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa6/<case-id>/trace.jsonl`, whose receipts link both outputs/streams |
| `make course-pa6` | FAIL, 105/121, retained course failure marker | `course-final.log`; `/home/vishvananda/work/private/v5rust/artifacts/clone/course/pa6/test/{counts.txt,course-result.json}` |
| `clone_check.py prior pa6` | PASS, PA1–PA5 differential and course lanes | `prior-final.log`; earlier course artifacts under `artifacts/clone/course/paN/test` |
| `clone_check.py preflight pa6` | PASS | `preflight.log` |
| `clone_check.py preserve pa6` | PASS, 66/66 incoming identities | `preserve.log`; independent identity proof in `stage-final-summary.json` |
| `make lint` (`clone_quality.py`) | PASS, rustfmt, strict Clippy, debug/release unit suites and source audit | `quality-final.log` |
| `make audit-rust` | PASS, 101 files, 0 findings | `audit-final.log` |
| `python3 <artifact-root>/probe.py` | PASS, 64/64 exact protected adapter probes | `probe-final.log`, `probe-summary.json`, `probes.jsonl`; intermediate divergence evidence retained in `mismatches/` |
| `python3 <artifact-root>/measure.py` | PASS, supported workload parity and protected nested-stack receipt | `measure-final.log`, `measurements.json`, `nested-summary.json`, `probes.jsonl` |

No oracle or harness defect was found. The course's retained failure marker
continues to make its protected wrapper fail despite the inner make outcome.
Stage completion still requires full course and differential success.

Resource evidence: a fixed 1,000-nested-class workload starts with a 1 MiB
stack and qualifies at a shared 16 MiB stack. Both implementations succeed
with identical dump/stream bytes; `nested-summary.json` reports `match: true`
and `stack_qualification.passed: true`, linking retained raw attempts. No
native exhaustion depth or diagnostic location was calibrated. The genuine
byte-budget guard remains active. Long integral binary spines of 1,000,
4,000 and 16,000 terms match without native recursion along the left spine.

The final warm measurement run uses A/A calibration followed by ABBA and
BAAB blocks, retaining every observation, input/binary/output hash and RSS.
Each sample's dump hash is checked. Wall time includes startup and dump I/O;
RSS comes from `/usr/bin/time`. The workloads exercise wrapped constant
initializers, scoped enums, const references, array bounds and assertions in
flat/namespace forms, plus the single-expression spine. These are observed
compiler costs, not equality or PA35 optimization requirements.

| Shape | Groups/terms | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| constants | 1000 | 29.78 / 10712 | 65.01 / 9288 |
| constants | 4000 | 83.04 / 24788 | 195.03 / 28284 |
| constants | 16000 | 292.89 / 80728 | 681.16 / 102284 |
| namespaces | 1000 | 27.11 / 10696 | 63.37 / 9284 |
| namespaces | 4000 | 71.70 / 24872 | 170.89 / 28344 |
| namespaces | 16000 | 332.68 / 82044 | 658.13 / 104256 |
| spine | 1000 | 4.60 / 5856 | 18.19 / 8756 |
| spine | 4000 | 5.67 / 6560 | 18.00 / 8736 |
| spine | 16000 | 9.89 / 7172 | 32.28 / 8744 |

## Remaining exact fixed-manifest case IDs

All paths below are relative to `pa6/`; full identities and raw trace ownership
are retained in `stage-final-summary.json`.

```text
tests/general/100-namespace-nonstatic-anonymous-union-bad.t
tests/general/200-anonymous-struct-declares-nothing-bad.t
tests/general/200-template-template-parameter.t
tests/general/300-complete-class-exception-specification.t
tests/general/300-declaration-forms-valid.t
tests/general/300-dynamic-exception-specification-different-set-bad.t
tests/general/300-dynamic-exception-specification-set.t
tests/general/300-noexcept-function-pointer-declarator.t
tests/general/300-unused-nonconstant-exception-specification-bad.t
tests/general/300-unused-unknown-exception-name-bad.t
tests/spec/200-anonymous-type-typedefs.t
tests/spec/200-namespace-anonymous-class-array-object.t
tests/spec/200-namespace-anonymous-union-injected-members.t
tests/spec/200-template-parameter-scope.t
tests/spec/300-template-template-inner-parameter-scope-bad.t
tests/spec/300-using-declaration-template-id-bad.t
```

## Handoff

The next boundary is ordinary exception specifications: retain typed declaration
facts, validate unused specifications, preserve dynamic exception-set identity
and schedule member specifications on complete-class events. Size/call queries
there require class layout and the next expression-typing slice. Independent
remaining groups are anonymous class/union naming and injection, template type
and template-template parameter environments, and special-member declarations.
`300-declaration-forms-valid.t` now stops at its constructor owner, before its
constant-reference controls; do not count the missing constructor as a
language rejection.

Independent audit questions: review optional nonconstancy versus semantic
failure through unselected operands; enum types before/after the closing brace;
underlying-range selection and overflow; aliases/imports and unscoped enum-only
qualification; hidden anonymous labels; exact object declaration point and
const-reference state; parameter object/source/canonical type distinctions;
query demand and array diagnostic order; and nested worklist failure cleanup.
Use the protected stack qualification receipt and the complete earlier-stage
ladder. The stage base and last-audited commit are unchanged; Ralph owns audit,
checkpoint acceptance and advancement.

## Ordinary exception-specification checkpoint

Implementation entry HEAD: `13ad51d2442932776c30ae44095188981dfc1164`.
Stage base and last-audited fields above remain unchanged. The preceding goal
turn is classified as progress: its committed integral/enumeration group
raised the fixed manifest to 105/121. This is a coherent partial PA6 handoff;
Ralph still owns checkpoint/final audit and stage advancement.

Read the repository contract, specification, clone plan and PA6 handout/guide.
Consulted frozen `analyzer_exceptions.cpp`, `analyzer_exception_sets.cpp`,
`analyzer_types.cpp`, `analyzer_declarations.cpp`, `analyzer_classes.cpp`,
`analyzer_calls.cpp`, `constant_evaluator.cpp`, `layout.cpp`, `analyzer.h` and
`lookup.cpp`. Rust owners are `sema/exceptions.rs`, `sema/layout.rs` and the
existing analyzer/declaration/declarator/graph/lookup/query/constant modules.
The source map records the translated responsibilities and pending interfaces.

Completed behavior and invariants for this slice:

- Function suffix exception qualifiers remain separate from canonical types.
  Ordinary functions retain typed `Absent`, `Pending(SpecificationId)`,
  `Evaluating` and `Value(ExceptionSet)` state on their shared entity.
  Redeclarations compare the first specification's result; bare `noexcept`,
  `throw()` and true operands converge, while false operands allow all.
- Dynamic specifications adjust top-level cv, arrays and function types,
  sort/deduplicate canonical `TypeId` sets and compare set identity, with
  exact void/incomplete-class/rvalue-reference rejection. Nonempty member
  specifications resolve at outermost class completion. Unused integral
  noexcept operands are checked at unit end; a redeclaration can demand the
  first operand sooner. Prototype scopes use adjusted parameter object types
  while retaining declared cv, and add no dump scopes.
- Namespace lookup and directive/inline edges retain declaration ordinals;
  compatible redeclarations keep their earliest visible ordinal. Deferred
  queries limit only namespace scopes, leaving complete-class/prototype lookup
  available. This is direct indexed visibility, with no scope snapshots,
  history traversal, unit-sized query allocation or negative lookup cache.
  Saved lookup/evaluation contexts restore on every Result path.
- PA6's C++ types mode ignores specifications on function-pointer and nested
  parameter declarators. Typedefs and alias declarations reject them after
  type formation, preserving source diagnostic order. Same-signature member
  redeclarations reject in their owning declaration layer before exception
  comparison; qualified out-of-class definitions still match their owner.
- Size/alignment queries demand plain class/union layout once. The class owns
  ordered non-static member EntityIds and target byte offsets. Explicit
  NotStarted/InProgress/Value layout state and an iterative dependency stack
  handle member/array class chains, natural alignment/padding, union maximum
  size, empty-class size 1 and the C++ 2^60-byte limit. Static members and
  functions consume no object storage. Packing, attributes and virtual members
  retain an explicit missing layout boundary rather than acquiring a plain
  layout. No layout is requested merely to print a type.
- Unevaluated monomorphic zero-argument calls consume the declared result type
  without executing a body; dense facts retain the callee/selected entity and
  result TypeId against original ExprIds. Evaluated non-constexpr direct calls
  in integral noexcept operands reject exactly. Constexpr interpretation,
  argument/default/overload/member-object conversion and broader expression
  exception facts remain separate expression/interpreter dependencies.

Architecture review: one parsed AST and one graph per source unit remain.
Specification/prototype and layout worklists belong to the analyzer's unit
lifetime; the returned graph retains typed values and identities. Canonical
function types contain no exception state. Layout visits only the demanded
class's member/dependency edges and records the result on that class. Dense
call-target facts have private store APIs; later consumers read those facts
instead of repeating lookup. Four unit regressions check exception/type
identity separation, namespace versus class visibility, demand-only layout
and offsets, and missing layout/interpreter status 86 boundaries. Existing
stack byte-budget safety remains unchanged. No unsafe, FFI, dependencies,
compiler subprocess fallback, text reparsing, environment reads under phases,
fixture/oracle/checker/Ralph-state edits or code-generation/optimizer/register
allocator changes were introduced. Backend lifetime policies are untouched.

The integral operand/query slice is not a certificate for general PA7
expression typing or interpretation, templates, exception overrides, function
pointer semantic exception facts, complete class attributes/bases/bit-fields,
overloads/default arguments or special members. These remain explicit owners
for the full compiler port. Fixture counts alone do not establish completeness.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/port/pa6-exceptions/`.
Candidate release `cppgm` SHA-256: `b821f50bc7d87ffe941f5e00b21c1e191ff64390932a298ccfd24e4e50dc235b`.
Frozen C++ SHA-256 remains
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.
`handoff.json` records the final committed HEAD and binary receipt.

The unchanged fixed manifest contains 121 identities. Incoming 105 passes are
all retained; six existing failures now pass, increasing the total to 111 and
reducing failures from 16 to 10. `stage-final-summary.json` records exact
incoming/new/preserved/remaining identities and proves no lost pass or manifest
change. This meets the failure-reduction criterion; Ralph executes its own
stageProgress audit. Each remaining failure exits 86, with its earliest owner
in `failure-clusters.json` and raw receipt/evidence paths in
`remaining-evidence.json`. No incomplete stub is counted as a language rejection.

| Command | Result | Evidence under this artifact root unless absolute |
| --- | --- | --- |
| `make build` | PASS, full release build 9.84 s; final incremental build 0.95 s | `build.log`, `build-final.log`, binary hash above |
| `make test-pa6` (`python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6`) | FAIL, 111/121 exact cases; 10 missing groups | `stage-final.log`, `stage-final-summary.json`; `/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa6/<case-id>/trace.jsonl` and each receipt's raw output/stream evidence |
| `make course-pa6` | FAIL, 111/121, retained course failure marker | `course-final.log`; `/home/vishvananda/work/private/v5rust/artifacts/clone/course/pa6/test/{counts.txt,course-result.json}` |
| `clone_check.py prior pa6` | PASS, PA1–PA5 differential and course lanes | `prior-final.log`; prior course artifacts under `artifacts/clone/course/paN/test` |
| `clone_check.py preflight pa6` | PASS | `preflight.log` |
| `clone_check.py preserve pa6` | PASS, 105/105 incoming identities | `preserve.log`; independent identity comparison in `stage-final-summary.json` |
| `make lint` (`clone_quality.py`) | PASS: rustfmt, strict Clippy, debug/release unit suites, source audit | `quality-final.log` |
| `make audit-rust` | PASS, 103 files, 0 findings | `audit-final.log` |
| `python3 <artifact-root>/probe.py` | PASS, 67/67 exact oracle probes | `probe-final.log`, `probe-summary.json`, `probes.jsonl`; intermediate raw mismatches retained under `mismatches/` |
| `python3 <artifact-root>/measure.py` | PASS: nine supported scaling inputs and protected nested-stack receipt | `measure-final.log`, `measurements.json`, `measurement-probes.jsonl`, `nested-summary.json` |

No oracle or harness defect was found. The course wrapper correctly fails
while its incomplete suite retains a failure marker. Full PA6 differential
and course success remain requirements for advancement. Early scratch probes
were relocated from checkout `artifacts/` to the external artifact root;
`relocation.json` maps their original evidence prefix, without rewriting raw
streams. The final probe/measurement receipts use the external paths directly.

The nested workload contains 1,000 classes and an innermost exception
specification querying the completed outer class. With a 1 MiB initial stack,
the protected adapter qualifies both tools at the shared 16 MiB stack:
`match: true`, `stack_qualification.passed: true`, both exits 0 and identical
streams/output hashes. `nested-summary.json` links preserved raw attempts.
No native exhaustion cutoff or diagnostic position was calibrated.

## Exception checkpoint measured resource costs

Warm A/A calibration then ABBA and BAAB blocks retain every timing/RSS
observation and input/output/binary hash; each sample verifies its dump hash.
Wall time includes startup and dump I/O, RSS comes from `/usr/bin/time`.
The exception workload combines parameter-size noexcept operands and
redeclared duplicate/reordered dynamic sets. The class workload combines
padded members/arrays, complete-class size specifications, later throw aliases,
later static call-result queries and repeated layout reads. The final workload
is a demanded chain of plain member classes, exercising iterative layout.
These are compiler costs, not byte equality or PA35 optimization requirements.

| Shape | Groups | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| exceptions | 1000 | 23.53 / 11468 | 57.37 / 9300 |
| exceptions | 4000 | 83.60 / 26252 | 183.38 / 27604 |
| exceptions | 16000 | 320.70 / 87288 | 713.16 / 101956 |
| classes | 1000 | 41.21 / 16344 | 86.27 / 14212 |
| classes | 4000 | 151.85 / 46116 | 293.66 / 46276 |
| classes | 16000 | 633.93 / 167612 | 1168.31 / 175664 |
| layout-chain | 1000 | 10.61 / 7248 | 25.84 / 8748 |
| layout-chain | 4000 | 26.63 / 12860 | 60.00 / 11412 |
| layout-chain | 16000 | 106.30 / 36580 | 190.93 / 36768 |

## Current remaining exact fixed-manifest IDs

Paths relative to `pa6/` (superseding the preceding checkpoint's 16-case list):

```text
tests/general/100-namespace-nonstatic-anonymous-union-bad.t
tests/general/200-anonymous-struct-declares-nothing-bad.t
tests/general/200-template-template-parameter.t
tests/general/300-declaration-forms-valid.t
tests/spec/200-anonymous-type-typedefs.t
tests/spec/200-namespace-anonymous-class-array-object.t
tests/spec/200-namespace-anonymous-union-injected-members.t
tests/spec/200-template-parameter-scope.t
tests/spec/300-template-template-inner-parameter-scope-bad.t
tests/spec/300-using-declaration-template-id-bad.t
```

## Exception checkpoint handoff

The ordinary integral exception-specification group and its plain-layout /
zero-argument result-query dependencies are complete for the documented
slice. The next independent fixed-manifest boundary is anonymous class/union
naming and declaration-point storage/member injection: consult
`analyzer_classes.cpp`, typedef name adoption and the original AST token spans.
Template type/template-template parameter environments form the other
independent group; the declaration-forms case still stops at its constructor
owner. Broader typed expressions, constexpr interpretation, exception overrides
and complete layout remain future dependencies, not substitutes for these
remaining declaration groups.

Review declaration versus canonical type exception facts; namespace ordinal
limits versus complete-class scope visibility; prototype object adjustment;
unused-specification validation and redeclaration demand order; dynamic type
adjustment/completeness/set comparison; demanded layout memo/offset ownership,
iterative dependency bounds and failure cleanup; and protected successful
stack qualification. Stage base/last-audited fields stay unchanged.


## Accepted partial checkpoint audit (2026-10-12)

Audit target: `pa6 full-stage`, phase `checkpointAudit`. Audit scope is all
three accepted checkpoints since the stage base, including fixes in reviewed
implementation commit `3140a2bbab5d8a42bfdba0259d0ee0a567439d4f`. Incoming HEAD was
`ba33bbc` and the checkout was clean. The prior checkpoint turn is classified
as progress: its retained implementation and gate evidence established 111/121
fixed cases. No interrupted/background work was assumed live without a tool
handle. This entry supersedes historical checkpoint claims where an audit
finding below corrected them; it certifies the reviewed partial checkpoint,
without declaring PA6 complete or advancing the stage.

Read the repository guide, clone contract, specification, clone plan, PA6
handout/guide and this ledger. Reviewed the complete implementation diff from
`70e745588987998ee1e5bde80b3861541eab753d` through the audited implementation
revision, plus the source map. Consulted the frozen C++ owners at
`/home/vishvananda/work/references/v5rust-cpp`, revision
`0fe8e3b4bde9929c2419fb7906028df83bb17de5`: `cppgm++.cpp`, semantic graph/entity
records and dump, `analyzer.cpp`, `analyzer_types.cpp`,
`analyzer_declarations.cpp`, `analyzer_classes.cpp`, `analyzer_bodies.cpp`,
`lookup.cpp`, `type_table.cpp`, `constant_evaluator.cpp`,
`integral_arithmetic.cpp`, `analyzer_exceptions.cpp`,
`analyzer_exception_sets.cpp`, `analyzer_expressions.cpp`,
`analyzer_conversions.cpp`, `analyzer_calls.cpp` and `layout.cpp`.
Rust owners are the existing `dev/src/sema/` graph, lookup, types, specifiers,
declarators, declarations, constants, integral, enumeration, query, exception,
layout, expression and dump modules, plus the new `exception_operands.rs`.
The driver/syntax interface diff remains a borrow of the single original AST.
The source map records both new responsibilities and pending wider owners.

### Findings and completed fixes

1. **Class completion ordering.** Dynamic member specifications were evaluated
   after deferred member bodies. With both `throw(Missing)` and a zero array
   bound in a member body, Rust reported the body error first. Class completion
   now resolves the specifications before draining bodies, matching the frozen
   owner. Regression: `400-audit-dynamic-order-body.t`.
2. **Type checking before noexcept evaluation.** Optional nonconstancy stopped
   traversal of a skipped floating arithmetic/comma operand before an unknown
   name; a nonconstant left operand also masked a later semantic error. Void
   calls could become boolean constants, and mixed void/integer conditional
   arms could produce an internal error. An iterative occurrence worklist now
   checks the supported scalar operand tree in C++ source order, with the
   conditional condition converted before its arms. Contextual bool, scoped
   enum operations and conditional incompatibility report exact diagnostics.
   Dense original-ExprId facts retain expression and arithmetic conversion
   types; constant evaluation consumes them instead of deciding conversions
   again. Nonselected arithmetic/calls remain unevaluated. Regressions cover
   skipped unknown names, name-before-nonconstancy, void results, scoped enum
   bool/comparison/condition rejection and incompatible conditional arms.
3. **Parameter source versus object type.** Hidden exception prototypes used
   adjusted types for both facts, changing `sizeof(decltype(a))` from 12 to 8
   for `int a[3]`, and accepting a source function type's size. Prototype
   bindings now retain declared types for decltype, while their entities retain
   adjusted expression types. Body/source dump behavior remains the frozen
   types-mode behavior. Regressions: both `proto-array-decltype` cases and
   `noexcept-function-param`; a unit test inspects both typed identities.
4. **Declaration publication point.** The saved namespace cutoff was taken
   after publishing the function and prototype bindings, letting a first
   declaration's specification see its own function. The cutoff is captured
   before publication; class visibility remains complete-class lookup.
   Regression: `400-audit-noexc-recursive.t` (exact undeclared-function error).
5. **Call target and failure facts.** Nonconstant calls through a variable or
   parameter reported the ordinary non-constexpr-function diagnostic. Dense
   call facts now retain the selected BindingId, from which EntityId and kind
   are read directly; evaluated indirect calls report the correct variable or
   parameter diagnostic. Queries/evaluation reuse the recorded call result and
   resolved name. The call-query parameter-vector clone was removed.
6. **Anonymous namespace indexing.** Reopening scanned every printed child of
   its owner, repeatedly visiting unrelated namespaces. A scope now records its
   anonymous namespace identity directly. Source order still comes from the
   existing child vector. Regression: `400-audit-namespace-reopening.t`; the
   dedicated 1,000/4,000/16,000 reopen workload matches the oracle.
7. **Type ownership and needless cloning.** The intern table duplicated every
   function parameter vector in its record and owning hash-map key. A
   randomized fingerprint index now names store-owned records, with dense
   collision links and exact structural comparison. A forced-collision unit
   test proves distinct identities and repeated interning reuse. Canonical
   traversal borrows dependency vectors on its discovery step and returns a
   completed root memo directly; no-op qualification/adjustment avoids rebuilding
   a type. Body/prototype parameter traversal copies only individual TypeIds,
   rather than entire parameter lists. No observable identity/order depends on
   hash iteration or fingerprints.

Seventeen required regressions and their new C++ reference pairs are in
`pa6/tests/general/400-audit-*.t`, discovered by the assignment's existing full
course lane. Existing frozen inputs, references, manifests and protected policy
were unchanged by this audit. `regression-reference-receipts.json` retains
creation diagnostics; `regression-summary.json` proves all 17 course invocations
also match the protected byte comparator. Three new unit tests exercise the
parameter ownership distinction, nested exception-query error cleanup and
interning collisions. Failed queries restore the occurrence worklist,
lookup cutoff and evaluation mode; fatal declaration failures discard their
unit graph rather than exposing partial states to a downstream consumer.

### Architecture and algorithm review

The completed groups remain general algorithms over typed syntax, never
filename recognition or embedded answers. Followed aliases, qualified scoped
and unscoped enum definitions/imports, array completion and nested function
parameters through checked IDs to entity-owned facts and source-facing dumps.
Enum values retain their integral type until completion and then the enum
identity; underlying selection and sequential widening follow the C++ candidate
order. Integral operations preserve LP64 conversion/promotion/rank, sign
extension, unsigned wrapping, signed undefined-operation rejection, shift
limits and sign-bit left shift. Existing numeric/enum/string/prototype probes
all still match, including deterministic diagnostics and emission order.

Ordinary exception states stay outside canonical function types. A first result
is memoized on its entity; redeclarations demand it, member throw sets wait for
the outermost class event and unused ordinary noexcept operands are checked at
unit end. Lookup follows lexical/indexed namespace edges with declaration
ordinals, without snapshots or invalidating unrelated facts. Plain class layout
walks only demanded member/dependency edges, detects cycles, retains offsets,
checks the 2^60-byte limit and reuses completed layout results. Printing a class
requires no layout or member-body demand. A failed layout rewinds active frames;
already successful dependency layouts remain memoized. Unsupported layout and
constexpr-call interpretation remain status 86 boundaries.

One source/cursor/flat AST owns each unit. Dense semantic facts name original
AST occurrences; the graph owns type/entity/scope stores and presentation order.
The operand traversal is a semantic consumer of that AST, not a second parser,
syntax copy or text transport. Fingerprint links allocate no per-collision node.
Constant, exception and layout worklists use nested base/truncate ownership.
No global semantic scans, polling retries, unsafe/FFI paths, production compiler
subprocesses, new dependencies, blanket lint allowances, mechanical file splits
or environment reads under `dev/src/` were added. Typed source limits pass the
independent audit. Backend/optimizer/register allocator code and lifetimes were
unchanged. No template-to-ELF or PA34 proof is claimed at this partial PA6
boundary; those consumers are not implemented here.

### Validation and preservation

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/port/pa6-audit/`.
Candidate release cppgm SHA-256: `f0ca712d4f0cc43c90ad3dffcab998c0f214eea9911233cf8294f2650c9c2865`.
Frozen C++ tool SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.
All final gate, probe and measurement receipts refer to this candidate binary.

| Command/check | Result | Evidence under artifact root unless absolute |
| --- | --- | --- |
| `make build` | PASS | `build.log`; final binary hash above |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6` (`make test-pa6`) | FAIL, 111/121 fixed identities, same 10 missing groups | `stage.log`, `stage-summary.json`, `remaining-evidence.json`; per-case raw traces under `/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa6/` |
| `make course-pa6` | FAIL, 128/138 including 17/17 new regressions, same 10 unfinished cases | `course.log`, `regression-summary.json`; `/home/vishvananda/work/private/v5rust/artifacts/clone/course/pa6/test/counts.txt`, `course-result.json`, `trace.jsonl` |
| `clone_check.py preflight pa6` | PASS | `preflight.log` |
| `clone_check.py prior pa6` | PASS, PA1–PA5 differential and full course lanes | `prior.log`; earlier course receipts under `/home/vishvananda/work/private/v5rust/artifacts/clone/course/paN/test/` |
| `make lint` / required `python3 /home/vishvananda/work/v5rust-control/clone_quality.py` | PASS, rustfmt, strict Clippy, debug/release unit suites, source audit | `quality.log` |
| `make audit-rust` | PASS, 104 files, zero findings | `audit.log` |
| `clone_check.py preserve pa6` | PASS, 111/111 | `preserve.log` |
| stageProgressPreserved | PASS, zero lost identities, exactly the same 121 fixed cases and 10 failures | `stage-start.log`, `stage-summary.json` |
| `make test-harness` | PASS | `harness.log` |
| Independent protected probes | PASS, 92/92 reviewed-scope probes plus 64/64 constant and 67/67 exception probes | `probe-final.log`, `extra-final.log`, `edge-final.log`, corresponding summaries and raw `probes.jsonl`; `legacy-constants/` and `legacy-exceptions/` |
| Protected resource/scaling probes | PASS, 12 supported inputs and nested-stack qualification | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `nested-summary.json` |

The general probe set also retains two explicitly unfinished dependency probes:
`function-return-cv` (incompatible function redeclarations/overload owner) and
`prototype-trailing` (deduced-type/trailing-return environment). They both remain
status 86, as the incoming ledger already documented. They are not counted
among the 223 matching reviewed-scope probes and are not new waived findings.
The fixed ten remaining case IDs are listed immediately above this audit entry
and again with exact receipts in `remaining-evidence.json`; all ten still exit
86. No normal C++ rejection, panic, signal or internal error substitutes for a
missing implementation. The course failure marker remains a real stage failure.
Fixture counts alone are not the basis for this partial audit acceptance.

### Measured costs and protected stack qualification

Each of 12 supported inputs first passes the protected adapter, then A/A
calibration, ABBA and BAAB blocks retain raw times/RSS and input/output/binary
hashes. Each timed sample's dump hash is verified. Wall time includes startup
and type-dump I/O; peak RSS comes from `/usr/bin/time`. These observations are
compiler costs, not required C++/Rust runtime equality or PA35 budgets.

| Shape | Groups | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| exceptions | 1000 | 23.30 / 11480 | 56.51 / 9984 |
| exceptions | 4000 | 83.15 / 26300 | 178.46 / 28304 |
| exceptions | 16000 | 342.94 / 87212 | 692.78 / 104100 |
| classes | 1000 | 44.07 / 16396 | 82.38 / 14312 |
| classes | 4000 | 159.75 / 46460 | 301.67 / 46504 |
| classes | 16000 | 621.32 / 167596 | 1134.71 / 177100 |
| layout-chain | 1000 | 9.98 / 7260 | 26.73 / 8744 |
| layout-chain | 4000 | 25.87 / 12864 | 55.43 / 11484 |
| layout-chain | 16000 | 88.07 / 36616 | 167.02 / 35432 |
| anonymous-reopen | 1000 | 9.30 / 6572 | 24.61 / 8756 |
| anonymous-reopen | 4000 | 23.74 / 9996 | 55.69 / 10796 |
| anonymous-reopen | 16000 | 67.39 / 24824 | 175.83 / 31664 |

The 1,000-nested-class workload starts with a 1 MiB stack and demands a size
query in a complete-class exception specification. Judge the protected receipt:
`match: true`, `stack_qualification.passed: true`, shared 16 MiB retry, both exits
0, identical streams and dump SHA-256
`45af63c10ac0d5d142f7f6decc3c570f5be44ec7808b69c9b6f950b64c358aec`.
`nested-summary.json` links the retained raw attempts and qualification evidence.
Initial native cutoff/diagnostic differences were not treated as language
mismatches; the genuine byte-budget guard is unchanged. Checkout/scratch usage
was approximately 26 GiB, below the supplied 100 GiB capacity ceiling.

The next boundary remains anonymous class/union naming/storage/injection,
template type/template-template parameter scopes and special-member declarations.
Broader expressions/conversions, interpreted pointer/floating/object/constexpr
values, templates/instantiation, inheritance/attributes/bit-fields and exception
overrides retain their owning future dependencies. This audit does not require
them to pass, reset a goal or edit Ralph state. The audit phase returns control
to Ralph with the reviewed implementation committed and the ledger's
last-audited revision advanced; full PA6 gates remain required for advancement.

## Anonymous type/union implementation checkpoint (2026-10-12)

Incoming clean HEAD: `42241603feadf96a3137e4669a39fbbd080edf12`.
Stage base remains `70e745588987998ee1e5bde80b3861541eab753d`; last-audited
commit remains `3140a2bbab5d8a42bfdba0259d0ee0a567439d4f`. This implementation
entry does not advance that field or PA6. The preceding goal turn is progress:
its committed checkpoint audit and retained gates establish the incoming
111/121 fixed identities. No interrupted process was assumed live from a state
file. This checkpoint completes the anonymous declaration/naming/injection
group for types mode, including its related declaration and layout invariants.
PA6 remains a partial assignment at 116/121 fixed cases.

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the complete clone plan, PA6
handout/scopes guide and current ledger. Consulted the frozen C++ revision
`0fe8e3b4bde9929c2419fb7906028df83bb17de5` directly, especially
`analyzer_classes.cpp` (`class_specifier`, `class_body`,
`anonymous_class_label`, `anonymous_union`, `name_unnamed_type`,
`define_implicit_default_constructor`), `analyzer_declarations.cpp`
(`simple_declaration`, `declare_typedef`), `analyzer.cpp`'s alias declaration,
`analyzer_types.cpp`, and `semantic_dump.cpp`. The source map records these
translated responsibilities and the still-pending wider owners.

### Completed behavior and typed ownership

- Unnamed classes share the existing class-body completion and deferred-body /
  exception events with named classes. A compact Class TypeId names its single
  EntityId, scope and ordered members. There is no extra syntax tree, class
  body parser, token replay or owning spelling copy.
- Specifiers retain the newly declared TypeId separately from the qualified
  base/use type. Only a defining declaration assigns an unnamed type's display
  name; reuse through an alias cannot rename it. The first unqualified simple
  declarator names a namespace/block type; member objects leave it unnamed,
  while a typedef may adopt its name. Derived/cv-qualified typedefs retain the
  underlying declaration identity. Alias declarations finalize anonymous enum
  ordinals and class labels before later uses.
- Class-owned `AnonymousClass::Pending/Labeled(TokenSpan)` retains original
  preprocessing token ordinals. A class-key-only anonymous declaration adds
  one checked token to its exclusive end after the semicolon. Leading
  specifiers leave the closing-brace end unchanged. Unnamed types in parameters,
  type-id queries/casts/new and function return definitions retain the C++ blank
  label until a naming declaration event, including the exact header space.
  Generated labels are presentation only, never hash/lookup/equality keys.
- Naming a placeholder changes its dump through the entity, without adding a
  tag or injected-class-name binding. The same correction removes the prior
  anonymous-enum naming routine's extra lookup publication. Typedef aliases
  remain aliases; an elaborated anonymous typedef is rejected at the C++ owner.
- Anonymous unions in global/named namespaces require static; an unnamed
  namespace or block does not. A declarator-less anonymous struct/class outside
  a class diagnoses that it declares nothing. Supported lone virtual/constexpr
  specifiers preserve the diagnostic before storage/injection checks.
- Injection walks only this anonymous class's printed bindings in source order,
  skips hidden unnamed type placeholders, checks member kinds and direct
  enclosing-scope conflicts, then adds bindings sharing each original EntityId
  and TypeId. It never scans unrelated declarations. Class-scope injection adds
  one hidden unnamed storage Variable EntityId to the enclosing class's member
  list, rather than treating each injected alternative as separate storage.
  Nested union/struct layout therefore follows the existing demand-only layout
  dependency walk and caches. Namespace/block types-mode injection does not
  demand semantic initialization/storage lowering.
- Types mode suppresses implicit constructor-scope demand for every anonymous
  class, including one named by a typedef, exactly as the C++ owner does.
  Normal named-class demand, source-order emission, semantic completion and
  layout lifetime rules are retained.

Rust owners: new `dev/src/sema/anonymous_types.rs` owns declaration-point
naming, label states, storage qualification and member injection;
`declarations.rs` owns shared class creation/body completion; `specifiers.rs`
retains declared-type and Virtuality facts; `analyzer.rs` routes simple/alias
and class-only declarations. `graph.rs` owns anonymous provenance and typed
storage identities; `enumerations.rs` owns anonymous enum naming without tag
publication; `expressions.rs` owns constructor demand; `dump.rs` borrows these
facts for exact output. `syntax/ast.rs` only removes now-unfulfilled dead-code
expectations from already-retained token spans. The parser/token ownership and
optimizer/code generator/register allocator are unchanged.

Thirteen new required inputs and newly created oracle reference pairs are in
`pa6/tests/general/400-anonymous-*.t`, executed by the existing full course
lane. Existing frozen inputs/references/manifests and protected policy were
unchanged. Two unit tests inspect shared injected member identity, one hidden
storage slot and its measured member offset, lack of tag publication, retained
anonymous identity and suppressed constructor demand. Strict debug/release
unit suites pass. No dependencies, unsafe/FFI, process fallback, environment
reads, fixture recognition, blanket lint allowances or mechanical splits were
added. Failed declarations return structured errors and discard their unit
rather than exposing partially injected state to a downstream phase.

### Validation and exact preservation

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/port/pa6-anonymous/`.
Candidate release cppgm SHA-256:
`cabb29c73e184dc1b227f0cc3494e058b73244726a4ee909bea0671107e78017`.
Frozen C++ tool SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command/check | Result | Evidence under artifact root unless absolute |
| --- | --- | --- |
| `make build` | PASS | `build.log`; binary hash above |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6` (`make test-pa6`) | FAIL, 116/121 fixed cases; five fewer failures | `stage.log`, `stage-summary.json`, `remaining-evidence.json`; raw traces under `/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa6/` |
| `make course-pa6` | FAIL, 146/151, same five unfinished fixed cases; 13/13 new regressions pass | `course.log`, `regression-summary.json`; `/home/vishvananda/work/private/v5rust/artifacts/clone/course/pa6/test/{counts.txt,course-result.json,trace.jsonl}` |
| `clone_check.py prior pa6` | PASS, PA1–PA5 differential and full course lanes | `prior.log`; earlier course artifacts under `artifacts/clone/course/paN/test/` |
| `clone_check.py preflight pa6` | PASS | `preflight.log` |
| `clone_check.py preserve pa6` | PASS, all 111 incoming passing identities | `preserve.log`; independent exact identity comparison in `stage-summary.json` |
| `make lint` / required `python3 /home/vishvananda/work/v5rust-control/clone_quality.py` | PASS, rustfmt, strict Clippy, debug/release unit suites and source audit | `quality.log` |
| `make audit-rust` | PASS, 105 files, zero findings | `audit.log` |
| Independent protected oracle probes | PASS, 87/87; new regressions also 13/13 exact matches | `probe-final.log`, `probe-summary.json`, `probes.jsonl`, `regression.log`, `regression-summary.json`, `regression-trace.jsonl`; oracle creation receipts in `regression-references.json` |
| Supported scaling/resource probes | PASS, nine scaling inputs plus deep anonymous-union stack qualification | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `nested-summary.json` |

The independent identity comparison uses the accepted incoming audit's stage
log and this checkpoint's complete stage log: exactly the same 121 fixed IDs,
zero lost passes and five gains. The gained IDs, relative to `pa6/`, are:

```text
tests/general/100-namespace-nonstatic-anonymous-union-bad.t
tests/general/200-anonymous-struct-declares-nothing-bad.t
tests/spec/200-anonymous-type-typedefs.t
tests/spec/200-namespace-anonymous-class-array-object.t
tests/spec/200-namespace-anonymous-union-injected-members.t
```

The remaining exact fixed IDs are:

```text
tests/general/200-template-template-parameter.t
tests/general/300-declaration-forms-valid.t
tests/spec/200-template-parameter-scope.t
tests/spec/300-template-template-inner-parameter-scope-bad.t
tests/spec/300-using-declaration-template-id-bad.t
```

All five still exit 86 at an explicit missing owner. `remaining-evidence.json`
links their protected raw candidate/oracle streams and artifacts. There is no
waiver for a normal rejection, panic, signal or missing implementation. Course
failure markers remain real failures. No oracle or harness defect was found.
Full PA6 success, rather than this coherent partial checkpoint, remains required
for advancement; Ralph owns the checkpoint/final audits and stage state.

### Resource observations and next boundary

Warm A/A calibration followed by ABBA and BAAB blocks retains each time/RSS
observation and input/output/binary hash. Every sample's dump hash is checked;
each input first passes the protected byte comparator. Wall time includes
startup and dump I/O; peak RSS comes from `/usr/bin/time`. These are compiler
cost observations, with ordinary host noise, not C++/Rust runtime equality or
PA35 generated-code requirements. Injection combines separate namespaces,
anonymous unions and qualified function-body lookup. Layout combines nested
anonymous unions, one hidden storage slot, padding and repeated layout reads.
Aliases combine anonymous class aliases, cv/pointer typedef adoption, reuse and
size queries. The following table is generated from `measurements.json`.

| Shape | Groups | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| union-injection | 1000 | 18.50 / 10040 | 41.61 / 8748 |
| union-injection | 4000 | 57.32 / 22724 | 114.95 / 23332 |
| union-injection | 16000 | 225.99 / 75776 | 434.70 / 83792 |
| anonymous-layout | 1000 | 28.13 / 12836 | 57.84 / 11456 |
| anonymous-layout | 4000 | 100.93 / 36800 | 192.14 / 36076 |
| anonymous-layout | 16000 | 407.97 / 129472 | 728.92 / 133704 |
| anonymous-alias | 1000 | 23.94 / 12148 | 58.76 / 10732 |
| anonymous-alias | 4000 | 82.85 / 31168 | 193.79 / 32556 |
| anonymous-alias | 16000 | 324.37 / 105448 | 665.20 / 120904 |

The 1,000-nested-anonymous-union workload starts at a 1 MiB stack and demands
its containing class's layout. Judge the protected receipt:
`match: true`, `stack_qualification.passed: true`, shared 16 MiB retry, both
exits 0, identical streams and dump SHA-256
`cbd73177643ddcaf84d2e89b9eda9e98089eae41ac418bd630d6c7e6734a5807`.
`nested-summary.json` links retained initial/retry attempts. The genuine
byte-budget guard remains unchanged; native exhaustion depths/diagnostic
positions were not calibrated. Checkout/scratch usage is approximately 26 GiB,
below the supplied 100 GiB capacity ceiling.

The next independent boundary is template type/template-template parameter
scopes and their declaration ownership/visibility (`analyzer.cpp`,
`analyzer_templates.cpp`, template records and type-table/dump variants).
`300-declaration-forms-valid.t` additionally stops at its constructor/special
member owner. Anonymous naming/injection is no longer its blocker. Broader
semantic storage/access paths and anonymous-union initialization lowering belong
to the later semantic/lowering owners, as do inheritance, attributes/packing,
bit-fields, full expressions/constexpr interpretation and template instantiation.
The review should trace declaration-point versus lookup publication, pending
versus assigned labels, shared injected member identity versus hidden storage,
source-order diagnostics, layout demand and anonymous constructor suppression.
No complete template-to-ELF or PA34 byte-equality proof is claimed here.
