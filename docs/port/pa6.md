# PA6 port ledger

Stage base commit: `70e745588987998ee1e5bde80b3861541eab753d`
Last audited commit: `743155c1cd0d29b13aa35346a67a4ab5f95cd92e` (PA6 declaration follow-up source/quality/prior-lane review; original full-stage acceptance `5851cab157eacc66f410f3275b0c06bd2f8d1c41`; PA7 acceptance remains pending).
Audit entries advance this field to the reviewed implementation revision;
Ralph retains stage advancement and goal ownership.

## PA7 audit follow-up to PA6 declarations — 2026-10-13

Reviewed clean entry `6ef88b8823f646a16bb17f0725394a355c48b635` and the
resulting implementation revision `743155c1cd0d29b13aa35346a67a4ab5f95cd92e`. This supplements
the prior PA6 acceptance review; it does not replace its stage base or Ralph's
stage ownership. The PA7 independent source audit found earlier-owned storage,
language linkage, block entity adoption, using-signature conflict, typedef and
builtin exception declaration mismatches. Rust `sema/linkage.rs`, declaration
and builtin/exception owners now implement the C++ control flow and failure
order, with namespace/name-indexed block obligations, retained first-declaration
facts, shared variable IDs and independent block-function defaults.

28 required regressions belong in PA6's original executed types lane; their
exact paths and new frozen reference generation are recorded in `fixtures.json`
at `/home/vishvananda/work/private/v5rust/artifacts/pa7-declaration-audit-20261013/`.
No existing fixture/reference or acceptance manifest changed. The final
`clone_check.py prior pa7` reruns PA1–6 fixed gates and full course suites:
PA6 remains **121/121 fixed**, now **722/722 full course**, retaining all
694 preceding course IDs including multi-unit cases. Build, strict rustfmt/
Clippy/debug+release tests, file audit and safe-Rust audit pass. See the PA7
follow-up for exact ownership, resolved failures, commit hashes, artifact paths,
resource observations and the independently found frozen-oracle signal bugs;
those are neither accepted PA6 rejection fixtures nor altered policy.

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

## Template parameter ownership checkpoint

Implementation entry HEAD: `a96e8b976715a73c97d6f95719549df176e5dd89`
(clean, 116/121 fixed cases). Stage base and last-audited commit above are
unchanged; implementation does not advance the audit marker. The interrupted
continuation left no retained change or live tool handle; classify that
continuation as no progress and take the next independent owner group. This
checkpoint implements that group and remains a partial PA6 handoff.

Read the repository guide, clone contract, architecture specification, Rust
conversion plan, PA6 handout and scopes/types guide. Consulted frozen revision
`0fe8e3b4bde9929c2419fb7906028df83bb17de5` directly, particularly:

- `analyzer.cpp`: template placement, parameter creation, inner parameter
  visibility, default names, shadow diagnostics and pattern assertions.
- `class_templates.cpp`, `template_names.cpp`, `type_table.cpp`: positional
  canonical entities, typed head shapes, equivalent redeclarations, retained
  defaults, duplicate-default and class pack/default ordering checks.
- `analyzer_classes.cpp`, `analyzer_declarations.cpp`, `analyzer_types.cpp`:
  namespace publication versus source printing, injected versus bare template
  names, qualified namespace definition overlays, parameter shadow checks and
  pattern array bounds/enumerator values.
- `analyzer_bodies.cpp`, `analyzer_exceptions.cpp`, `semantic_dump.cpp`,
  `entity_records.h`, `template_records.h`, `semantic_types.h`: body context,
  constructor demand, later specification ownership and exact source views.

### Completed behavior and data flow

`dev/src/sema/templates.rs` owns private checked `HeadId`/`ShapeId` stores,
parameter scopes, canonical position and structural shape indexes, declaration
placement, shadow checks and head/default merging. A head borrows its original
AST clause and retains each default's syntax `TypeId` and declaration `ScopeId`;
it retains no tokens or rendered names. Defaults' syntax/scope fields have one
narrow dead-code expectation because instantiation is still a later consumer.
Published named parameters have distinct source `EntityId`s and types;
canonicalization maps those to shared positional entities, preserving the
source names of renamed declarations. Position keys include depth, pack state,
index and inner template shape. The C++ index's intentional low-16-bit
truncation is explicit; host store sizes and IDs are checked separately.

`graph.rs` owns these stores and entity facts. `types.rs` memoizes canonical
parameter composition through its existing DAG walk. `declarations.rs` and
`analyzer.rs` distinguish namespace/class lookup ownership from source print
ownership with typed `ValueScope` facts. `declarators.rs` retains its resolved
lexical lookup scope for bodies and specification owners. Qualified namespace
class/function template definitions use a hidden parent-linked parameter
scope overlay containing only their named parameters and leading into their
namespace; they do not copy all visible names. Declaration and lookup indexes
remain separate from source-order bindings/children.

Completed scope/declaration rules include named and unnamed type and
template-template parameters; hidden inner lists that do not leak names or
print scopes; duplicate and enclosing/nested shadow diagnostics; declaration
placement in namespace/class/local/C-linkage contexts; defaults seeing only
preceding parameters; template-template defaults naming actual templates;
pack defaults and class pack/default order; compatible class forwards,
renamed definitions and same-shape function redeclarations; exact mismatched
class-head/default diagnostics; bare versus injected template-name type checks;
and qualified namespace definition lookup/print order. Alias declarations keep
head identity so their bare names are diagnosed as templates.

`enumerations.rs` checks template parameter shadowing before publication and
walks template enumerator initializers for names while retaining the C++
pattern's sequential placeholder values. `constants.rs` preserves zero bounds
for nonconstant/nonpositive pattern bounds after required name checks.
`analyzer.rs` defers pattern assertions to name traversal; deferred bodies
restore their recorded template depth. `expressions.rs` suppresses implicit
constructor demand in patterns. `dump.rs` reads named facts for the exact
`template-parameters`, `typename` and `template-parameter` forms.

Three new unit tests inspect namespace publication versus print ownership,
inner-scope isolation, shared member/parameter type identity, distinct source
names versus positional canonical function identity, and inner-head shape in
canonical keys. The older missing-owner test now uses a non-type parameter,
which remains an explicit missing group. No AST is reparsed/copied, no semantic
phase uses text transport, and no backend, optimizer or allocator code changes.
No dependencies, unsafe/FFI, compiler fallback, environment reads, fixture
recognition or blanket lint suppression were added. Errors are structured;
a failed unit is discarded rather than consumed downstream.

### Validation and fixed-case progress

Evidence root:
`/home/vishvananda/work/private/v5rust/artifacts/port/pa6-templates/`.

| Required command/check | Result | Evidence relative to root |
| --- | --- | --- |
| `make build` | PASS | `build.log` |
| `make test-pa6` = required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6` | FAIL, **120/121**, four fewer fixed failures | `stage.log`, `incoming-stage.log`, `stage-summary.json`; raw invocation receipts under `artifacts/clone/cases/pa6/` |
| `make course-pa6` | FAIL, **173/174**, same remaining fixed case | `course.log`; `artifacts/clone/course/pa6/test/{course.log,counts.txt,course-result.json,trace.jsonl}` |
| `clone_check.py prior pa6` | PASS, PA1–PA5 differential and complete course lanes | `prior.log`; earlier `artifacts/clone/course/paN/test/` |
| `clone_check.py preflight pa6` | PASS | `preflight.log` |
| `clone_check.py preserve pa6` | PASS, 116/116 incoming passing identities | `preserve.log`; independent identity comparison in `stage-summary.json` |
| `make lint` = required `clone_quality.py` | PASS, rustfmt, strict Clippy, debug/release unit suites and source audit | `quality.log` |
| `make audit-rust` | PASS, 106 implementation files, zero findings | `audit.log` |
| New owning-lane regressions | PASS, 23/23 exact protected matches and course outcomes | `regression-trace.jsonl`, `regression-summary.json`, `regression-references.json` |
| Independent oracle probes | 96/100 exact matches; four explicit missing later owners | `probe.log`, `probe-summary.json`, `probes.jsonl`; raw mismatch streams/files linked by receipts |
| Wide parameter probe | PASS, 65,538 parameters, both exits 0 and exact dumps | `wide.log`, `wide-summary.json`, `wide-trace.jsonl` |
| Supported scaling and stack qualification | PASS | `measurements.json`, `measurement-probes.jsonl`, `nested-summary.json`, `measure.log` |

The fixed manifest retains exactly the same 121 case identities. No incoming
pass was lost, no case was removed and added tests do not count as progress.
These four fixed cases changed from status 86 to exact oracle outcomes:

```text
tests/general/200-template-template-parameter.t
tests/spec/200-template-parameter-scope.t
tests/spec/300-template-template-inner-parameter-scope-bad.t
tests/spec/300-using-declaration-template-id-bad.t
```

The only remaining fixed failure, relative to `pa6/`, is
`tests/general/300-declaration-forms-valid.t`. Its earliest missing owner is
explicit constructor/special-member declarations; candidate exit 86 remains
an actual failure, including its course failure marker. Protected raw evidence:
`artifacts/clone/cases/pa6/cc1c75b61a08d0b55e01/{trace.jsonl,course.log}`.
The new `pa6/tests/general/500-template-*.t` inputs and their new oracle-only
reference pairs supplement the existing executed course lane. Frozen inputs,
reference files, manifests and checker policy were not modified.

### Remaining dependency boundary and architecture review

The next fixed-stage dependency is the explicit constructor in `holder`, its
parameter/member initializer/body scopes and subsequent use. Consult
`analyzer_declarations.cpp`'s special member/function declaration owners and
`analyzer_classes.cpp`'s member-completion owners before translating it.
Template parameter scopes are no longer a fixed-case blocker.

Broader probes deliberately retain real missing owners, without a rejection
waiver: `inner-qualify.cpp` needs dependent qualified name/type facts;
`function-different-shape.cpp` and `function-template-ordinary.cpp` need indexed
ordinary/template overload sets; `function-noexcept.cpp` needs template
specification ownership/equivalence and demand. All four now exit 86 at those
owners rather than emit an incorrect ordinary diagnostic. Non-type parameters,
pack expansion/substitution, qualified members of specialized/dependent classes,
full alias/variable template semantics, deduction, specialization and
instantiation remain outside this completed group. Earlier ledger dependencies
for full declaration/specifier, statement, layout and expression semantics
still apply. No full PA6, template-to-ELF or PA34 proof is claimed.

Architecture review should trace scope versus declaration versus print owner,
source parameter versus positional identity, inner-list visibility, default
binding before declaration, qualified overlay lifetime, and deferred body
context/constructor suppression. The new indexes visit only queried heads,
parameters and lexical parents; no whole-program scan, speculative
instantiation, global invalidation or backend lifetime change was introduced.
Ralph owns checkpoint/final audits and advancement.

### Measured resource costs

Final candidate cppgm SHA-256: `643f7b4e20a9835c400b084a06eea7b90587ac9afb26a9d431f4f942aff3c09b`.
Frozen C++ tool SHA-256: `9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

Warm A/A calibration precedes paired ABBA and BAAB blocks; retain every
wall-time/RSS observation and binary/input/output hash in `measurements.json`.
Every workload first passes the protected comparator, and every measurement
checks its full dump hash. Timing includes process startup and output I/O;
peak RSS is from `/usr/bin/time`. These are host-noise-sensitive compiler
resource observations, not runtime equality or PA35 code-generation targets.
The nine workloads combine type/default/member/function scopes, hidden inner
heads and canonical shapes, and repeated namespace publication/head merging.

| Shape | Groups | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| template-type-members | 1000 | 34.47 / 17008 | 69.56 / 15612 |
| template-type-members | 4000 | 132.51 / 51444 | 256.57 / 52936 |
| template-type-members | 16000 | 529.41 / 187868 | 1059.67 / 203732 |
| template-template-heads | 1000 | 14.46 / 10068 | 37.23 / 8744 |
| template-template-heads | 4000 | 51.85 / 24096 | 101.01 / 23360 |
| template-template-heads | 16000 | 170.28 / 77520 | 346.15 / 83588 |
| template-redeclarations | 1000 | 21.91 / 12144 | 51.06 / 10696 |
| template-redeclarations | 4000 | 69.92 / 33488 | 151.92 / 34344 |
| template-redeclarations | 16000 | 334.07 / 116920 | 647.90 / 127380 |

A fixed 600-nested-template-class input starts with a 1 MiB stack. The
protected receipt reports `match: true`, `stack_qualification.passed: true`,
a shared 16 MiB retry, both exits 0 and identical dump SHA-256
`24b2ce77040c55c74e861e026e856a7fec86d515c55883f709699f744533c86b`.
`nested-summary.json` links retained initial and retry attempts. The genuine
byte-budget guard was preserved; native exhaustion depths and diagnostic
positions were not calibrated. Filesystem usage is approximately 26 GiB of
the supplied 100 GiB ceiling. The wide 65,538-parameter probe independently
checks the target index-width boundary without a resource-cutoff comparison.


## Explicit constructor and static-member definition checkpoint (2026-10-12)

Implementation entry HEAD: `45d7cc47963df5098814bd240d38aa33a705e784` (clean, 120/121 fixed cases).
Stage base remains `70e745588987998ee1e5bde80b3861541eab753d`; last-audited
commit remains `3140a2bbab5d8a42bfdba0259d0ee0a567439d4f`. Implementation does
not advance the audit marker or stage. The preceding accepted implementation
turn is progress: its retained template ownership commit raised 116/121 to
120/121. The incoming required progress failure correctly identified the one
unchanged fixed failure; no live background work was inferred from a state file.
This entry records a coherent implementation handoff with all current fixed
and full course lanes green. Ralph owns the checkpoint/final audit and advancement.

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the complete Rust clone plan,
PA6 handout/scopes guide and stage ledger. Consulted frozen revision
`0fe8e3b4bde9929c2419fb7906028df83bb17de5` in
`/home/vishvananda/work/references/v5rust-cpp`, especially:

- `analyzer_declarations.cpp`: `special_member`, `declared_name`,
  `declare_function`, `redeclare_function`, `new_function`, `default_definition`,
  `declare_variable`, `earlier_variable` and `check_variable_specifiers`.
- `analyzer_bodies.cpp`: body parameter publication, duplicate names,
  types-mode constructor initializer traversal and default-name demand.
- `analyzer_classes.cpp`: class completion/deferred-body order and
  `define_implicit_default_constructor` suppression.
- `analyzer_types.cpp`, `analyzer_copy.cpp`, `analyzer_initialization.cpp`:
  suffix diagnostics, copy/move classification and the semantic versus types
  mode distinction for defaults/initialization.
- `semantic_dump.cpp`, graph/entity records and the already parsed special
  member/function-body AST: source labels, ownership and output order.

### Completed behavior and typed data flow

`dev/src/sema/constructors.rs` owns the ordinary explicit constructor
entry, class/name checks, signature publication/redeclarations, defaulted
constructor recognition and body demand. A private graph index keyed by
`(EntityId, canonical TypeId)` distinguishes constructor overload declarations
and gives qualified definitions direct access to their earlier entity.
Constructors have unnamed ordinary bindings with the class name on their entity
for presentation; they cannot hide or replace the injected class type. The
class retains a user-constructor `BindingId`, independently of its implicit
constructor scope. These declarations suppress implicit default-scope demand
for evaluated constructions, including deleted/defaulted/copy declarations.

Source function types remain on each binding, while canonical parameter
adjustment selects the shared function entity. Qualified definitions preserve
source parameter names/cv/arrays/function forms and use their recorded lexical
scope. Ordinary constructor declarations, multiple distinct signatures,
compatible qualified definitions, delete/default state, constexpr agreement,
class redeclaration/redefinition, by-value self-parameter and cv/ref diagnostics
follow the C++ owner. Special specifier formation returns an optional written
TypeId separately from its declarator base; constructors consume that typed
fact rather than walking the specifiers again to infer whether a type was given.
The ordinary function suffix now rejects a trailing return type without auto,
including on constructors, before resolving that result type.

The C++ types mode deliberately does not record/evaluate function default
arguments or pure specifiers. Its by-value self diagnostic therefore applies
only to a sole class-value parameter; a later parameter's written default is
not consulted. Defaulted copy/move recognition accepts cv-qualified class
references, including volatile, and does not demand semantic construction
plans. These distinctions are translated behavior, not a language-policy change.

`analyzer.rs` retains the function `EntityId` in its existing completion queue;
function entities and body scopes have bidirectional typed ownership. Parameter
bindings retain source types, named duplicates reject before initializer/body
traversal, and unnamed parameters add no lookup name. Mem-initializer expressions
run in complete-class function scope in source order before the compound body.
Types mode does not resolve their target names or build subobject initialization
plans. Ordinary specifications use the existing entity-owned exception state
and completion events, preserving specification-before-body diagnostics.

`data_members.rs` owns class-variable qualification/name/constexpr checks and
static in-class initializer constraints. `graph.rs` records a typed
`DataMemberKind::Instance/Static` fact. `declarations.rs` lets qualified static
member definitions reuse their declaration's entity, preserves completed array
and constant facts, records definition/redefinition and rejects non-static
out-of-class definitions, repeated in-class initializers and forbidden static
on the out-of-class definition. Qualified definitions never append an object
storage slot. Their initializer uses the declarator's resolved lookup scope.
The existing layout owner still walks only ordered instance-member dependencies
when demanded; static definitions do not affect offsets or object size.

Two new unit regressions inspect constructor signature/source-type identity,
injected-name lookup, distinct overload entities, body owner links, implicit
scope suppression, static definition identity and exclusion from object storage.
Twenty-seven new inputs/reference pairs under `pa6/tests/general/600-*.t` are
executed by the existing course lane. Their references were generated only by
the frozen C++ tool; `regression-references.json` records each input/reference
hash and oracle streams/status. Existing fixtures, references, fixed manifests,
checker policy, oracle sources and Ralph state were unchanged.

Architecture review: one borrowed AST, compact checked IDs, private typed
store APIs, source order separate from indexed identity, no owning name text
or syntax copies, and one completion event for each queued body remain.
Constructor queries visit only the requested class/signature. No whole-unit
scan, speculative instantiation, global invalidation, new dependency, unsafe,
FFI, compiler subprocess fallback, phase environment read, mechanical split
or blanket lint suppression was introduced. All errors use Result/structured
errors; failed units are discarded. Source audit passes with 108 implementation
files and no findings. Optimizer, code generator, allocator and backend lifetime
code were not changed; this frontend checkpoint makes no object/PA34 proof claim.

### Validation, fixed progress and evidence

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/port/pa6-constructors/`.
Final Rust cppgm SHA-256: `f47ff0a9796e5a39d6a044616be65ef7b11f6450cfe6fac473159e02dd3997d8`.
Frozen C++ cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command/check | Result | Evidence relative to root unless absolute |
| --- | --- | --- |
| `make build` | PASS, 9.82 s release build | `build.log` |
| `make test-pa6` = required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6` | PASS, **121/121** | `stage.log`, `stage-summary.json`; raw receipts under `artifacts/clone/cases/pa6/` |
| `make course-pa6` | PASS, **201/201**, no failed marker | `course.log`; `/home/vishvananda/work/private/v5rust/artifacts/clone/course/pa6/test/{counts.txt,course-result.json,trace.jsonl}` |
| `clone_check.py prior pa6` | PASS, PA1–PA5 differential and complete course lanes | `prior.log`; earlier `artifacts/clone/course/paN/test/` |
| `clone_check.py preflight pa6` | PASS | `preflight.log` |
| `clone_check.py preserve pa6` | PASS, all 120 incoming passing identities | `preserve.log`, independent set proof in `stage-summary.json` |
| `make lint` = required `clone_quality.py` | PASS, rustfmt, strict Clippy, debug/release unit suites and source audit | `quality.log` |
| `make audit-rust` | PASS, 108 files, zero findings | `audit.log` |
| Independent protected probes | 131/135 exact; four explicit next-owner boundaries | `probe.log`, `probe-summary.json`, `probes.jsonl`; every mismatch links raw candidate/oracle diagnostics/files |
| Final new regressions and suffix probes | PASS, 27/27 regressions plus 3/3 trailing-return probes | `final-extra-summary.json`, `final-extra-trace.jsonl`, oracle creation receipts in `regression-references.json` |
| Supported scaling and stack qualification | PASS | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `nested-summary.json` |

The fixed manifest has exactly the same 121 case identities; no incoming pass
was lost or case removed. Its sole incoming failure,
`tests/general/300-declaration-forms-valid.t`, now matches exit, streams and
complete dump bytes. It first needed explicit constructor declarations, then
reached the qualified static data-member ownership rule in the same declaration
owner. Raw case evidence:
`/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa6/cc1c75b61a08d0b55e01/trace.jsonl`.
Remaining fixed failing case IDs: **none**. Failure count decreased **1 to 0**;
new tests did not count as progress. `stage-summary.json` retains the incoming,
gained, preserved and current identity sets. Ralph reruns stageProgress and
its audits; fixture counts alone do not certify the entire semantic implementation.
No oracle or harness defect was found.

### Resource measurements and next dependency boundary

Warm A/A calibration followed by ABBA and BAAB blocks records every elapsed
compile time/RSS sample and input/output/binary digest. Each input first passes
the protected comparator; every measured invocation checks its complete dump
hash. Times include process startup and type-dump I/O; RSS is `/usr/bin/time`
peak KiB. These are compiler cost observations under host noise, not required
C++/Rust runtime equality or PA35 generated-code optimization targets.
Constructor bodies combine references, copy declarations, later type aliases
and suppression. Qualified definitions combine adjusted signatures, static
member definitions and demanded plain class layout. Signature fanout uses
100/400/1,600 distinct array-reference signatures in one class, exercising the
index without unrelated program scans.

| Shape | Groups/signatures | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| constructor-bodies | 1000 | 37.55 / 15672 | 87.85 / 14980 |
| constructor-bodies | 4000 | 142.44 / 45668 | 300.01 / 50160 |
| constructor-bodies | 16000 | 555.20 / 166200 | 1097.88 / 190528 |
| qualified-definitions | 1000 | 31.00 / 13572 | 70.37 / 12860 |
| qualified-definitions | 4000 | 109.08 / 35360 | 237.27 / 40284 |
| qualified-definitions | 16000 | 424.87 / 122836 | 909.16 / 152068 |
| constructor-signatures | 100 | 4.77 / 5800 | 15.84 / 8748 |
| constructor-signatures | 400 | 6.97 / 5828 | 19.71 / 8836 |
| constructor-signatures | 1600 | 13.33 / 8600 | 33.13 / 8752 |

A fixed 1,000-nested-class input with explicit constructor bodies starts with
a 1 MiB stack. The protected receipt reports `match: true`,
`stack_qualification.passed: true`, a shared **16 MiB** retry, both exits 0
and identical dump SHA-256
`358e13699e3a55684b1df6ffbdfcc655bbd6464778b603b871de3c55f5a4b6ee`.
`nested-summary.json` links retained raw initial/retry attempts. The genuine
byte-budget guard remains active; native exhaustion cutoffs and diagnostic
positions were not calibrated. Filesystem usage is approximately 26 GiB of the
supplied 100 GiB ceiling.

This completed group covers ordinary explicit constructor declaration/body
views and qualified static member definition ownership in types mode. Next
boundaries remain separately owned: constructor templates need head/shape
matching; destructors/conversion functions need typed special name/signature
keys and presentation; function-try handlers need statement/exception scopes.
The precise independent probe IDs are `constructor-template`,
`destructor-boundary`, `conversion-boundary`, and `handler-boundary` under the
evidence root. All four still exit 86 against a successful oracle; they are
real unfinished groups, not waived rejections. Consult `declared_name`,
`special_member`, copy/destructor owners and body handler traversal before
porting them. Template specialization/instantiation, non-type parameters,
full ordinary overload resolution, member initialization/construction plans,
inheritance/extended layout, full statements and PA7 expression semantics
retain their prior dependency boundaries. No full template-to-ELF or PA34
claim is made. Stage base and last-audited commits stay unchanged.


## Full-stage acceptance audit: control/declaration findings (2026-10-12)

Audit target: `pa6 full-stage`, phase `audit`. Incoming clean implementation
revision was `7e93f70` (explicit constructors/static members), following stage
base `70e745588987998ee1e5bde80b3861541eab753d`. The previous goal turn is
progress: that retained commit and its fixed/course receipts establish 121/121
fixed cases. The interrupted tool notice did not establish a live process;
this audit began with a clean checkout and fresh command handles.
Reviewed implementation revisions are `18b0cd7e6b6c0455eb2cc5dfa03156d1bac9f7d7`
and `1194a1aa75c2d54bc02db2c8753df6c71e0b010f`, plus the earlier ownership
checkpoints since the prior audit marker. **Full-stage acceptance remains
unproven.** Green fixtures do not close the remaining declaration owners below;
Ralph's existing full-stage goal remains active and stage state is unchanged.

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the complete PA6 handout and
scopes/types guide, clone plan, source map and historical stage ledger.
Consulted the frozen C++ owners directly at revision
`0fe8e3b4bde9929c2419fb7906028df83bb17de5`, including `cppgm++.cpp`,
`analyzer.cpp`, `analyzer_bodies.cpp`, `analyzer_handlers.cpp`,
`analyzer_range_for.cpp`, `analyzer_declarations.cpp`, `analyzer_types.cpp`,
`analyzer_exceptions.cpp`, `semantic_dump.cpp`, graph/entity/type records,
`type_table.cpp`, lookup and template-head owners. The review follows the
Types branch, separating its name/scope/declaration responsibilities from
later expression conversion, template instantiation and control-flow semantics.

### Findings fixed and source responsibilities

1. Selection/iteration/handler statements and function-try bodies returned
   status 86 although C++ Types mode builds their block scopes and declarations.
   `sema/statements.rs` now owns if/switch/while/do/for, declared-type range-for,
   conditions, guarded substatements, try/catch and original-AST statement
   selection. Condition declarations reuse the ordinary declaration/type
   owner. Range names precede the range variable's type lookup and publication;
   catch bindings use their own enclosing block, followed by their compound
   body. Catch-all ordering and constexpr function-try rejection match the
   owning C++ diagnostics. Return/throw/control-flow validation remains the
   later semantic mode's responsibility, exactly as in C++ Types mode.
2. Case labels silently skipped their operand names. Their expression walk now
   precedes the labeled statement, preserving source-order failures without
   imposing a later constant-expression case-conversion rule.
3. Outermost function, controlled-statement and handler blocks allowed variables
   to redeclare their parameters/condition/exception names. Typed guarded-block
   metadata checks the immediate construct's value index. Nested blocks still
   permit shadowing; type-name behavior follows the C++ variable-only check.
4. Function declarations ignored constexpr disagreement, and return-type-only
   differences reached an unfinished overload owner. Canonical adjusted
   parameter/cv/ref/varargs comparisons now select the exact ordinary
   redeclaration diagnostic. Constexpr agreement remains an entity fact outside
   canonical function type identity. Ordinary definition diagnostics use the
   function declaration's entry location, rather than its later declarator-id.
5. C++11 constexpr non-static member functions lacked their implicit const
   function qualifier. A typed function-specifier operation composes that type
   once; static member declarations/qualified definitions retain their static
   type. Source types and canonical adjusted signatures remain distinct.
6. A static function declaration following external linkage was accepted.
   Optional typed function properties now retain linkage/static facts. The
   supplemental revision preserves internal linkage inherited through unnamed
   namespaces, including nested named namespaces, while C-linkage functions
   retain the C++ external-linkage rule. No rendered name or output spelling
   participates in linkage or type identity.
7. Function bodies now explicitly reset and restore enclosing class depth,
   following the C++ local-class completion boundary. Constexpr function-try
   rejection occurs before mem-initializer names. Function-definition analysis
   moved into its owning helper to keep the declaration dispatcher within the
   existing source/function limits, without adding lint allowances.

Rust owners are `sema/{statements,analyzer,declarations,graph,lookup,types}.rs`;
`syntax/ast.rs` removes only dead-code expectations on facts now consumed.
The source map records statement/body/handler/range and declaration ownership,
plus the wider unported responsibilities. Fifty-eight new oracle-derived
`pa6/tests/general/700-audit-*.t` cases and reference pairs run in the existing
course lane. Existing frozen fixtures/references/checker/oracle policy were
unchanged. Reference creation streams/hashes and all exact regression receipts
are retained separately. One graph ownership test checks local condition/catch
identity, parent edges, guarded blocks and absence of leakage.

Architecture review: one borrowed parsed AST and unit-local graph; checked
compact IDs and immutable canonical type records; dense selected-reading facts
refer to original StmtIds, never a replacement semantic syntax tree. Scoped
lookup follows indexed names/edges; the prefix helper borrows a bounded original
component slice. Selection is retained as a fact, not reused as a cache across
changed instantiation environments. No new semantic result cache or incomplete
cache key is introduced. Block/catch/range binding publication and print order
remain distinct. Demand, layout/constant/exception state and per-unit ownership
are preserved. Stack checks protect recursive statement/compound traversal;
long expression spines retain the existing iterative walk. Errors are structured
Results; failed units are discarded. No unsafe/FFI, dependencies, compiler
subprocess fallback, fixture recognition, phase environment reads, parser
replay, mechanical split, blanket lint expectation or backend policy change
was introduced. Optimizer/code generator/allocator and per-function backend
lifetimes remain untouched.

### Fresh gates, exact preservation and evidence

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/pa6-full-audit/`.
Final candidate cppgm SHA-256:
`6d83bcfd8b4d279c41b99de907e625b6b1713cfadc0636fd4e3770a79b13ec78`.
Frozen C++ cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command/check | Result | Evidence relative to root unless absolute |
| --- | --- | --- |
| `make build` | PASS | `build.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6` | PASS, 121/121 unchanged fixed IDs | `stage.log`, `stage-incoming.log`, `stage-summary.json`, `fixed-traces/`; original receipts under `artifacts/clone/cases/pa6/` |
| `clone_check.py course pa6` (original full `make course-pa6` lane) | PASS, 259/259, no failed marker | `course.log`, `course-result.json`, `course-counts.txt`, `course-trace.jsonl`; `artifacts/clone/course/pa6/test/` |
| `clone_check.py prior pa6` | PASS, PA1–PA5 fixed and full course lanes | `prior.log`; `artifacts/clone/course/paN/test/` |
| `clone_check.py preflight pa6` | PASS, protected inputs/oracle/routing | `preflight.log` |
| `clone_check.py preserve pa6` | PASS, 121/121 incoming passing IDs | `preserve.log`; unchanged stage identity set in `stage-summary.json` |
| `make lint` = `python3 /home/vishvananda/work/v5rust-control/clone_quality.py` | PASS, rustfmt, strict Clippy, debug/release unit suites and source audit | `quality.log` |
| `make audit-rust` | PASS, 109 files, zero findings | `audit.log` |
| `make test-harness` | PASS | `harness.log` |
| New owning-lane regressions | PASS, 55+3 exact comparisons and original course outcomes | `regression-summary.json`, `extra-regression-summary.json`, corresponding traces and reference-creation receipts |
| Independent control/declaration probes | 58/61 exact; three explicit later/unported dependency probes retained | `probe-final.log`, `probe-summary.json`, `probes.jsonl` and each mismatch's raw evidence |
| Reviewed legacy groups | 64/64 constants, 67/67 exceptions, 87/87 anonymous types, 96/100 templates, 132/135 constructors | `legacy-summary.json`, `legacy-*/probe.log`, `legacy-*/probes.jsonl`; unfinished probe receipts retained |
| Supported scaling and protected stack qualification | PASS | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `nested-summary.json` |

The exact fixed-stage failing-case list is empty. New regressions supplement
coverage, never redefine the fixed manifest or count as a fixed-stage gain.
PA33's implementation-wide ownership census/debug/backend release obligations
and PA34 object/tool fixed-point equality remain their later milestones; this
PA6 audit imposes no PA35 generated-code optimization targets.

### Resource observations and unresolved acceptance boundary

Every supported scaling input first matches the protected comparator. Warm A/A
calibration then ABBA/BAAB retains every time/RSS observation and binary/input/
output hash; each measured dump hash is checked. Wall time includes startup
and dump I/O, RSS comes from `/usr/bin/time`. Validation overlapped this run,
so timings include host contention and are observations, not a performance
threshold or a C++/Rust runtime-equality requirement.

| Shape | Groups | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| control-scopes | 1000 | 39.77 / 14972 | 91.15 / 16376 |
| control-scopes | 4000 | 142.00 / 40788 | 282.65 / 56648 |
| control-scopes | 16000 | 541.80 / 146024 | 1106.58 / 216088 |
| handler-scopes | 1000 | 19.72 / 10768 | 49.45 / 10736 |
| handler-scopes | 4000 | 69.14 / 25528 | 156.39 / 31196 |
| handler-scopes | 16000 | 274.20 / 85824 | 570.67 / 116196 |

The 800-nested-if probe starts with a 1 MiB stack. Its protected receipt reports
`match: true`, `stack_qualification.passed: true`, a shared 16 MiB retry,
both exits 0 and identical full dump SHA-256
`3e4ffc6e91b056252b549d420706366bb8632e38e521e3bc59732020a47e2a57`.
`nested-summary.json` links preserved initial and retry attempts. The genuine
byte-budget safety guard remains active. Native exhaustion cutoffs and
positions were neither required to match nor calibrated. Storage usage is
approximately 27 GiB of the supplied 100 GiB ceiling (`du` could not read the
filesystem's inaccessible lost+found directory).

Full acceptance is still withheld for concrete successful-oracle declaration
paths: `constructor-template`, `destructor-boundary`, `conversion-boundary`
in `legacy-constructors/`. Each still exits 86 at its existing typed ownership
boundary, versus C++ exit 0. They are not waived language rejections. The former
`handler-boundary` now matches. The next independent dependency is typed
special-member names/signature/label ownership and constructor template-head
matching: consult `declared_name`, `special_member`, `declare_function`,
`default_definition`, implicit exception ownership and dump labels directly.

Additional independent later-boundary probes remain recorded, without shrinking
the acceptance claim: `range-auto-boundary` needs placeholder deduction;
template `inner-qualify`, `function-different-shape`,
`function-template-ordinary` and `function-noexcept` need dependent qualified
facts, ordinary/template overload indexing, or template exception ownership.
The handout excludes expression typing, overload resolution and template
instantiation/disambiguation; their owning later semantics remain distinct
from the outstanding PA6 declaration responsibilities. Remaining Missing sites
still require that ownership classification before a full-stage signoff.
This entry proves a coherent reviewed control/declaration increment and clean
green gates; it does not prove all stage-owned C++ responsibilities complete,
complete Ralph's goal, advance PA6, change acceptance policy or edit Ralph state.


## Full-stage audit continuation: special-member ownership (2026-10-12)

Incoming clean HEAD was `cf84f715cc03b3f893c43113f5dcb78a7d6261a6`.
Stage base remains `70e745588987998ee1e5bde80b3861541eab753d`. The preceding
turn is progress: its retained control/declaration commits, source review and
fresh fixed/course/quality receipts are authoritative. This continuation
reviewed and committed `dab1205c35f289a525d537ba323fa920b9de5092`; it does not
advance PA6 or complete Ralph's existing full-stage audit goal. There is no
fixed-manifest failure, but the remaining expression-key dependency below still
prevents a full completeness claim.

Consulted AGENTS.md, CLONE_CONTRACT.md, the PA6 handout/scopes guide, current
ledger and source map, with the previously read spec/clone plan as the normative
architecture. Read frozen C++ revision
`0fe8e3b4bde9929c2419fb7906028df83bb17de5` directly: `declared_name`,
`special_member`, `declare_function`, `redeclare_function`, `default_definition`,
`declare_typedef` and member overload checks in `analyzer_declarations.cpp`;
body/completion controllers; `analyzer_types.cpp`'s parameter/prototype and
declarator owners; `analyzer_exceptions.cpp`'s pending/fixed template and omitted
destructor-specification branches; `template_dependence.cpp`,
`template_fixed_expressions.cpp`, `template_placeholders.cpp`'s name/size keys;
`analyzer_expressions.cpp`'s parameter-object and size typing;
`constant_evaluator.cpp`, construction/destructor owners, semantic dump,
entity/type records and syntax render views.

### Completed related behavior and audit fixes

- `sema/special_members.rs` replaces the constructor-only owner. A private
  graph index uses class EntityId, MemberKind, canonical function TypeId and
  optional interned ShapeId. Source parameter types remain on each binding.
  Constructors, destructors and conversion functions cannot collide; identical
  parameter signatures with different template heads or ordinary/template
  categories retain distinct entities. Qualified definitions reuse the correct
  entity/head and merge defaults by the existing template owner. No ordinary
  overload search or instantiation is substituted for this declaration index.
- Named type/template-template constructor and conversion template scopes,
  renamed qualified definitions, class member-template contexts, canonical
  parameter adjustment, shape/default disagreements, class redeclaration and
  virtual-template/destructor-template diagnostics are translated. These close
  the incoming `constructor-template`, `destructor-boundary` and
  `conversion-boundary` probes; all 135 original constructor-owner probes now
  match the frozen tool.
- A conversion result and its suffixes consume one typed DeclaratorEnvironment;
  no second lookup overlay is built for the same declaration. Name/type checks,
  qualified namespace versus enclosure checks, static/virtual checks and later
  suffix failures retain the owning C++ order and exact locations. A legacy
  qualified-class probe exposed a changed diagnostic; its namespace-scope
  diagnostic is restored and added to the executed lane.
- Every special declaration's label borrows its original AST Name. A sparse
  BindingId label map preserves per-declaration alias spellings; the first
  entity's name remains the function-scope label. The shared syntax renderer
  writes only the last component and removes conversion-label spaces exactly
  as C++ does. No rendered name/type is used for semantic equality, indexing
  or phase transport.
- Conversion parameter rejection, cv/ref types, constexpr's C++11 const
  qualification, adjusted compatible definitions, reference/cv overload
  exclusions, delete/default and redefinition rules match Types mode. Existing
  variants are queried through the type interner without publishing speculative
  types; conversion parameter lists are empty, so these bounded qualification
  probes clone no parameter allocation. Defaulted destructor bodies do not
  demand later construction or cleanup plans.
- The audit found additional ordinary declarations with written result types:
  C++ Types mode accepts `int ~C()` and `int C()`. They now use the same typed
  member factory rather than reach an internal-error path or hide the injected
  class name. Non-template lookup signatures omit the result, while compatible
  redeclarations compare it explicitly. Only the type-less special-member
  entry sets the C++ user-constructor and copy-by-value facts; the written-type
  entry preserves C++'s implicit-constructor-scope demand. Invalid special
  typedef names and conversion functions with written return types follow
  their exact owning diagnostics. These are frozen mode behaviors, not new
  language rules or fixture-specific corrections.
- Types mode does not synthesize subobject destructor facts. Its first omitted
  destructor specification allows exceptions; a later omitted ordinary
  declaration compares with implicit true, just as the frozen owner does.
  Bare template noexcept/empty throw and literal true specifications retain
  normal typed exception facts outside canonical function type identity.
- `sema/template_exceptions.rs` retains a first noexcept pattern's original
  ExprId, parameter clause, source TypeId, ScopeId and namespace ordinal. It
  shares source-ordered unit-end validation with ordinary specifications.
  Class-template members and operands naming their own/dependent parameters
  wait for instantiation; fixed operands are validated in hidden prototypes.
  The inherited 32-name/64-expression-depth own-name probe uses an iterative
  AST worklist. Unbound/dependent names remain deferred as in the C++ fixedness
  check, rather than receive an ordinary-mode rejection. Source binding types
  remain distinct from adjusted parameter object types. A source function's
  sizeof key failure precedes operand typing; array values read the adjusted
  typed query. New forced-size/division and function/decltype probes verify the
  distinction, not just an otherwise unobservable boolean result.

Two new unit tests inspect kind/head-shape/signature isolation, distinct entities
and shared compatible conversion definitions, original per-binding labels versus
first-entity body labels, and retained template-specification environments without
premature prototype demand. The source map assigns all new responsibilities and
keeps the wider pending owners explicit. There are 110 primary plus one additional
new `pa6/tests/general/800-special-*.t` inputs with oracle-generated reference
pairs, executed by the original course target. Frozen inputs/reference files,
oracle sources/binaries, protected policy and Ralph state are unchanged.

Architecture review: one parsed AST, compact checked IDs, owned unit-local graph
and immutable canonical type records; no syntax duplication or text reparsing.
Declarators retain their original syntax name alongside semantic name identity.
Head/default and declaration/print/lookup ownership remain separate. The new
index and qualification queries touch only the requested member/signature and
its finite qualification variants. Template probes allocate only bounded own-name
storage and visited type dependencies; they never scan unrelated unit bindings.
No semantic lookup result cache, incomplete memo key or global invalidation is
introduced. Failed queries restore lookup limit, evaluation mode and template
depth; failed units are discarded. Function bodies retain explicit completion
and stack lifetimes. The new modules own coherent algorithms, not mechanical
fragments, and pass the independent file/function/nesting audit. No unsafe/FFI,
new dependency, compiler subprocess fallback, phase environment read, blanket
lint allowance or fixture recognition is added. Optimizer, allocator, code
emission and per-function backend policy/lifetimes remain untouched.

### Fresh validation and preservation

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/pa6-special-members/`.
Final candidate cppgm SHA-256:
`be7d39b6401c8dd7eee3ab213ebf243e6800f6cd77172167c4063c08dc7dd539`.
Frozen C++ cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command/check | Result | Evidence relative to root unless absolute |
| --- | --- | --- |
| `make build` | PASS | `build.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6` | PASS, 121/121 unchanged fixed cases | `stage.log`, `stage-summary.json`, `fixed-traces/`; original `artifacts/clone/cases/pa6/` receipts |
| `clone_check.py course pa6` (original full course target) | PASS, 370/370, no failed marker | `course.log`, `course-course-result.json`, `course-counts.txt`, `course-trace.jsonl`; `artifacts/clone/course/pa6/test/` |
| `clone_check.py prior pa6` | PASS, PA1–PA5 differential and full original course lanes | `prior.log`; `artifacts/clone/course/paN/test/` |
| `clone_check.py preflight pa6` | PASS | `preflight.log` |
| `clone_check.py preserve pa6` | PASS, 121/121 incoming identities | `preserve.log`; unchanged stage identity set |
| `make lint` = required `clone_quality.py` | PASS, rustfmt, strict Clippy, debug/release unit suites and source audit | `quality-final.log` |
| `make audit-rust` | PASS, 110 files, zero findings | `audit.log` |
| New owning-lane regressions | PASS, 110+1 exact protected comparisons and course outcomes | `regression-summary.json`, `extra-regression.log`, corresponding traces and reference-generation receipts |
| Expanded independent probes | 110/111 exact; one explicit remaining expression-key boundary | `probe-final.log`, `probe-summary.json`, `probes.jsonl`; initial/intermediate mismatch streams retained |
| Legacy ownership probes | 64/64 constants, 67/67 exceptions, 87/87 anonymous types, 97/100 templates, 135/135 constructors | `legacy-summary.json`, `legacy-*/probe.log`, `legacy-*/probes.jsonl` |
| Supported resource/scaling and stack probes | PASS | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `nested-summary.json` |

The fixed-manifest failure list is empty and all earlier passes are preserved.
Supplementary case counts are not used as a completeness proof or as a
fixed-manifest gain. No harness code changed, so the previous harness audit
remains relevant; the original checker and all current compiler lanes ran
unchanged. PA33-wide release/ownership and PA34 object/tool proof obligations
remain at their due stages; no PA35 optimization objective is imposed.

### Measured resource costs and next audit boundary

Nine fixed supported inputs first pass the protected byte comparator. A/A
calibration followed by ABBA and BAAB retains every wall-time/RSS observation,
input/output/binary hash, and verifies each measured dump hash. Startup and
output I/O are included; peak RSS comes from `/usr/bin/time`. Some validation
ran concurrently, so these are noisy compiler resource observations, separate
from exact generated output and not a runtime-equality or optimization gate.

| Shape | Groups | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| template-special-bodies | 1000 | 46.56 / 17712 | 105.18 / 17760 |
| template-special-bodies | 4000 | 156.67 / 53004 | 338.07 / 60656 |
| template-special-bodies | 16000 | 653.10 / 192308 | 1362.82 / 232308 |
| conversion-variants | 1000 | 36.73 / 14940 | 78.76 / 13492 |
| conversion-variants | 4000 | 122.21 / 42432 | 280.01 / 43732 |
| conversion-variants | 16000 | 505.40 / 154232 | 1092.22 / 170228 |
| template-constructor-shapes | 1000 | 28.48 / 14916 | 64.32 / 14216 |
| template-constructor-shapes | 4000 | 104.85 / 44156 | 213.62 / 47344 |
| template-constructor-shapes | 16000 | 433.13 / 158560 | 838.35 / 179616 |

A fixed 700-nested-class input combines member templates, constructors,
destructors and conversion bodies. From a 1 MiB initial stack, the protected
receipt reports `match: true`, `stack_qualification.passed: true`, a shared
16 MiB retry, both exits 0 and identical complete dump SHA-256
`82273175354e29be683206c441bd91d47a6eb1f583bf968daf26561ee098a54d`.
`nested-summary.json` links preserved initial/retry evidence. The genuine
byte-budget guard is unchanged. Native exhaustion cutoffs and diagnostic
positions were not calibrated or required to match.

Full acceptance is still withheld. The precise expanded probe
`constructor-template-expression-redeclaration-boundary.cpp` redeclares
`noexcept(sizeof(T)==4)` under a renamed head; C++ succeeds and Rust exits 86
at `template exception expression equivalence`. The first declaration is now
retained correctly, but expression equivalence needs the reusable typed
`decltype_words`/`name_words`/`size_words` responsibility in
`template_placeholders.cpp`. Do not replace the missing key with rendered
text, literal expression comparison, evaluation alone or a generic rejection.
The next independent owner is that expression-key interface and its complete
parameter/type/name context, followed by the remaining source-responsibility
classification before full signoff. There are also real pending broader
probes `inner-qualify`, `function-different-shape` and
`function-template-ordinary`; their dependent qualified and ordinary/template
overload owners are unchanged. Generic operator/special-name interfaces and
other Missing sites still require review against the handout's due boundary.
This committed increment closes the three incoming basic declaration probes
and preserves every gate, but does not prove the complete original objective.
Ralph's existing full-stage audit goal remains active; no state or acceptance
policy was edited.


## Template expression keys and alias-template declaration audit

Reviewed incoming documentation revision `26ffd7f00c9f8614346da413f6d41d8be7a0336a`
and implementation `dab1205c35f289a525d537ba323fa920b9de5092`, then audited
implementation `7f072acb50466fec690d20f600694e61fc1cb737`. The stage base remains
`70e745588987998ee1e5bde80b3861541eab753d`. This entry closes the incoming
constructor-template expression-equivalence boundary; full-stage acceptance
is still withheld pending the remaining responsibility review and fixes.

Read the repository contract, specification, clone plan, PA6 handout and
scope/type guide, ledger and source map in this ongoing audit. Consulted the
frozen C++ source at `0fe8e3b4bde9929c2419fb7906028df83bb17de5`, especially
`template_placeholders.cpp::decltype_words/name_words/parameter_word/resolution_words/size_words`,
`analyzer_exceptions.cpp::record_noexcept/check_fixed_template_noexcept`,
`analyzer_types.cpp::prototype_scope/declarator`,
`template_names.cpp::declare_alias_template`,
`class_templates.cpp::check_template_defaults`,
`analyzer.cpp::namespace_alias/alias_declaration`,
`lookup.cpp::DesignateOne`, and `analyzer_declarations.cpp::declare_typedef`.
Dependence and unevaluated-value rules were checked directly in
`template_dependence.cpp`, `template_packs.cpp` and `constant_evaluator.cpp`.
The source map records the translated slice and remaining owners separately.

### Findings fixed and completeness of this increment

- Template noexcept operands now compare typed structural keys in the two
  declarators' hidden prototype scopes. Both prototypes use the first entity's
  source parameter types, and source parameters key by clause position. Types
  use canonical IDs; ordinary declarations use entity IDs; free names use
  interned name IDs. Integral/character/boolean literal values normalize by
  type and value. Other literals keep lexical spelling. Parentheses,
  canonical operators, assignment/postfix distinctions, ordered children,
  call counts, casts, member spelling, allocation/deletion and initializer
  forms retain their C++ comparison roles. Equal evaluated results do not
  imply equivalent operands. Unsupported C++ key forms remain explicitly
  unknown; unported dependent qualified types, packs and non-type arguments
  remain explicit boundaries.
- Size-query keys preserve the source binding type before parameter adjustment.
  A fixed expression-form size query may key by its value, while a type-id
  query retains canonical type identity. Source function-type size failures
  and source array extents retain their reference control-flow positions.
  Both prototypes, comparison timing and first-specification facts remain
  separate from the later unit-end validation queue.
- The first array bound of a new-expression is removed through a borrowed
  declarator suffix view. Neither the AST nor the original declarator's dense
  type fact is overwritten by that view's query result.
- Namespace aliases now have distinct declaration EntityIds for name keys,
  while sharing target ScopeIds. Lookup still merges aliases naming the same
  namespace, as `DesignateOne` requires. The first fresh stage run exposed
  `tests/general/200-namespace-type-convergence.t` after identity separation;
  target-based lookup convergence restored that case. Its initial failure
  receipt and later exact passes are retained.
- Alias templates now publish hidden lookup bindings, independently of
  ordinary typedef presentation. Namespace redeclarations compare head shape
  and canonical aliased type, keep the first binding/head/defaults and return
  before rechecking new defaults, exactly as the frozen implementation does.
  Class duplicates, declaration conflicts, inline class/enum definitions,
  default ordering and source parameter adjustment have independent probes.
  The CPP-accepted repeated or different default behavior was preserved;
  frozen code and references were not corrected.
- Typedefs naming an injected class name reject before publication. Diagnostics
  use the root declarator start, including a leading pointer operator;
  ordinary `using C=C` and differently named aliases preserve their C++ results.
- Own-name collection's 32-name limit now counts the dependent named function
  parameters actually added, rather than limiting how many parameter
  declarations are examined. A dependent parameter after 40 ordinary
  parameters correctly suppresses premature source-function-size checking.

Rust owners are `sema/expression_keys.rs`, `sema/name_keys.rs`,
`sema/template_exceptions.rs`, `sema/templates.rs` and their existing
`exceptions`, `declarators`, `declarations`, `graph`, `lookup` and `analyzer`
interfaces. Keys are ephemeral and environment-local: no expression-ID-only
key cache can reuse facts from another prototype. A private sparse graph API
provides parameter positions without repeatedly scanning a clause. Published
IDs remain checked and unit-local; original source types remain distinct from
adjusted object/signature types. Syntax is parsed once and borrowed throughout;
there is no rendered-expression reparsing or C++ production fallback. Ordinary
and alias-template factories share the owning default-order algorithm while
retaining their different redeclaration policies. New code is safe Rust, uses
structured Results, stays within source/function/nesting limits and adds no
lint allowance. Backend/optimizer policies were not changed.

Added 121 oracle-derived cases in the original executed
`pa6/tests/general/850-expression-*.t` lane: 79 key/dependence cases, seven
injected-name typedef cases and 35 alias-template declaration cases. New refs
and statuses are documented by artifact receipts; existing frozen refs and
gate policy are unchanged. Supplementary counts are not a completeness proof
or a fixed-manifest gain.

### Validation and retained evidence

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa6-expression-keys/`.
The candidate cppgm++ SHA-256 is
`473d449cfeee37febe99710a680ad7cf8ea148211c05683fd1b03f94f3f92d05`;
the frozen C++ tool remains
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command or independent review | Result | Evidence under the artifact root |
| --- | --- | --- |
| `make build` | PASS | `build-final.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6` | 121/121 exact | `stage-final.log`, `stage-summary.json`, `fixed-traces/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py course pa6` | 491/491; no failure marker | `course-final.log`, `course-course-result.json`, `course-counts.txt`, `course-trace.jsonl` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa6` | PA1–PA5 exact gates and full original course lanes PASS | `prior-final.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa6` | PASS | `preflight-final.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa6` | 121/121 preserved | `preserve-final.log` |
| `make lint` / required `clone_quality.py` | fmt, strict Clippy, debug/release unit suites and file audit PASS | `quality-final.log` |
| `make audit-rust` | 112 files; zero findings | `audit-final.log` |
| Original special-member probes | 111/111 exact | `special-final.log`; original `pa6-special-members/probes.jsonl` |
| New key probes | 79/81 exact; two explicit dependent qualification boundaries | `probe-final.log`, `probe-summary.json`, `probes.jsonl` |
| Alias-template probes | 35/35 exact | `alias-probes-final.log`, `alias-summary.json`, `alias-probes.jsonl` |
| Legacy ownership probes | constants 64/64, exceptions 67/67, anonymous types 87/87, templates 97/100, constructors 135/135 | `legacy-summary.json`, `legacy-*.log`, `legacy-*.jsonl` |
| New executed regression inputs | 121/121 exact on the reviewed implementation and full course lane | `regression-references.json`, `typedef-references.json`, `extra-regression-references.json`, corresponding traces and `all-regressions-final-summary.json` |

The fixed-manifest failing case list is empty. Fresh receipts, rather than
old probe-summary files, produced the normalized legacy summaries. All prior
passing cases remain passing. No harness code changed. PA33's full
release/ownership census and PA34's complete object/tool fixed-point proof
remain their due obligations; no PA35 optimization objective was imposed.

### Resource observations and protected native-stack proof

Six supported fixed workloads pass byte comparison before measurement. A/A
calibration followed by ABBA and BAAB retains every wall-time/RSS observation,
input/output/binary hash, and verifies the dump hash after every measured
invocation. Startup and dump I/O are included; peak RSS uses `/usr/bin/time`.
Some quality validation overlapped these observations, so host noise remains;
these are compiler resource measurements, separate from output equality.
`measurements.json`, `measurement-probes.jsonl` and `measure-final.log` retain
results; `initial-resource-observations/` preserves the first run's log,
comparison receipts and raw RSS samples.

| Shape | Groups | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| prototype-expression-keys | 1000 | 44.38 / 16384 | 97.89 / 17768 |
| prototype-expression-keys | 4000 | 146.47 / 47896 | 316.75 / 61372 |
| prototype-expression-keys | 16000 | 572.13 / 171936 | 1252.22 / 234928 |
| alias-expression-keys | 1000 | 35.05 / 15680 | 87.37 / 16348 |
| alias-expression-keys | 4000 | 129.19 / 44152 | 286.31 / 56212 |
| alias-expression-keys | 16000 | 529.12 / 160952 | 1123.75 / 215668 |

The fixed 1200-unary-operator template-key probe starts at a 1 MiB native
stack. Its first Rust guard report was unlocated and therefore failed the
protected qualifier; that raw mismatch remains under
`mismatches/5ecadad088e54be690264984b8e9f258/`. The key boundary now supplies
its original operand location to the existing genuine byte-budget guard.
No guard threshold, physical recursion cutoff or C++ diagnostic position was
calibrated, and no protected policy changed. The final receipt is
`match: true`, `stack_qualification.passed: true`, with a shared 16 MiB retry,
both exits 0, no stream/artifact differences and complete dump SHA-256
`c7d6e2d7432b749126de257f296cb9f2da95ffb44218607d77372e68cf4969b8`.
`nested-summary.json` links all retained initial/retry evidence. The original
small-stack exits are retained rather than treated as successful compilation.

### Remaining acceptance boundary

The incoming exact constructor-template redeclaration now succeeds and all
111 special-member probes match. Full acceptance remains withheld:
`dependent-qualified-value` and `dependent-qualified-type` still exit 86
against C++ success. The legacy `inner-qualify`,
`function-different-shape` and `function-template-ordinary` probes remain
explicit unresolved boundaries; overload/instantiation features must be
classified against the handout rather than certified from these counts.

A separate 27-case responsibility census (`boundaries.py`, `boundary-final.log`,
`boundary-summary.json`, `boundaries.jsonl`) has five exact matches and 22
remaining mismatches. Exact unresolved IDs are:

- `function-template-throw-fixed`, `function-template-throw-dependent`,
  `constructor-template-throw-fixed`, `function-template-throw-redeclare`,
  `function-template-noexcept-qualified-value`;
- `operator-free-plus`, `operator-free-new`, `operator-free-delete`,
  `operator-member-plus`, `operator-member-subscript`, `operator-member-call`,
  `operator-literal`, `destructor-namespace-written-type`;
- `class-inheritance-simple`, `class-inheritance-unknown`, `class-bit-field`,
  `new-runtime-array-names`, `new-runtime-array-unknown`, `new-array-class-demand`,
  `member-pointer-declaration`, `function-trailing-return`,
  `variable-placeholder-auto`.

The newly found injected-name typedef mismatch is fixed and absent from that
list. The namespace written-type destructor currently reaches status 70,
which is not an acceptable matched rejection. The next dependency is the
shared declaration-name identity/factory interface for ordinary operator,
literal and nonmember special labels, then nonempty template throw-set
ownership and the remaining responsibility classification. Declaration
collection must be distinguished from later overload resolution, expression
typing, class layout and instantiation; do not simply declare all these
probes out of scope. Rust Missing paths, full source responsibility coverage
and architecture still need review before stage signoff. Ralph's existing
full-stage audit goal stays active; no Ralph state, goal or acceptance policy
was created, reset or advanced.


## Callable declaration identities and template throw-set audit

Reviewed incoming documentation `3832475785e817a0692f31e7a25d75014a9ca048`
and implementation `7f072acb50466fec690d20f600694e61fc1cb737`, then audited
implementation `b635af3fa3484170186c324c243bf339a17f86dd`. Stage base remains
`70e745588987998ee1e5bde80b3861541eab753d`. This increment closes the incoming
ordinary operator/literal/nonmember-label factory and nonempty template throw
boundaries. It is not a full-stage acceptance claim.

The ongoing audit read the contract, spec, clone plan, complete handout,
scope/type guide, ledger and source map. Direct frozen C++ review included
`analyzer_declarations.cpp::declared_name/declare_function/new_function/redeclare_function/declare_declarator/default_definition`,
`analyzer.cpp::predeclare_allocation/using_declaration`,
`analyzer_bodies.cpp::function_body/check_id_expression`,
`analyzer_types.cpp::lookup_name/declarator_target`,
`analyzer_exceptions.cpp::record_throw_specification/check_redeclared_exceptions/pending_noexcept`,
`analyzer_exception_sets.cpp::resolve_throw_specification`, and
`analyzer.h`'s analyzer defaults. Source revision remains
`0fe8e3b4bde9929c2419fb7906028df83bb17de5`; neither source nor gate policy changed.

### Reviewed behavior and fixes

- `function_names.rs` owns typed `ValueKey` and `SpecialName` identities for
  canonical operator token plus array flag, literal suffix NameId and shared
  destructor identity. Lookup never keys on rendered labels. The graph owns
  a private sparse `(ScopeId, SpecialName)` index, compatible redeclarations
  retain their first visibility ordinal, and qualified special-name lookup
  uses the existing inline/directive namespace walk. No unrelated declaration
  scan or incomplete result cache was introduced.
- Callable declarations and definitions use the same value factory, declaration
  checks and typed `Definition` context. Body/default/delete definitions retain
  redefinition and deleted-first-declaration ordering before linkage/constexpr
  checks. Generic function qualifiers and qualified static-member diagnostics
  now share their reference control-flow boundary. Defaults remain name-only
  analysis in Types mode; copy/move assignment defaultability uses result and
  parameter class/reference facts without synthesizing backend bodies.
- Source binding labels retain the original final AST component; function scopes
  retain the first entity label. Alternative operator spelling keys alike while
  its source label remains observable. Allocation declaration labels omit a
  space; predeclared global allocation entity scope labels contain it. Qualified
  target failures in definitions use declaration-start location, independently
  of declarator-id/root locations used by declaration diagnostics.
- Four standard global allocation/deallocation functions are language-required
  hidden predeclarations. Typed `AllocationOrigin::Predeclared/Declared` facts
  distinguish the first written declaration. Its noexcept operand is not
  evaluated by the reference; its dynamic throw types are still checked for
  completeness. Later specifications compare normally. Initial probes corrected
  the mistaken use of the native driver's default: Types mode's analyzer
  `exceptions_` default is **true**. No driver policy or environment read was
  added to the semantic phase. Class allocation operators retain implicit static
  function properties while their source constexpr declarator formation remains
  unchanged, including C++11 implicit const behavior.
- A function template's first dynamic throw specification records adjusted
  canonical types immediately, even in a class being defined. First rvalue
  references reject, while completeness checks are skipped for function templates.
  Ordinary class-template members retain instantiation deferral. Later template
  throw types are neither resolved nor compared: only the nonthrowing category
  is compared, and a pending template noexcept operand retains its deferral.
  `SpecificationMode` explicitly distinguishes ordinary, template compatibility
  and template expression work. Ordinary/template paths reuse the same type
  adjustment algorithm and preserve ordinary unit/class queue order.
- Special names remain original structured syntax, parse-once ownership and
  compact IDs are preserved, and the suffix-view architecture from the prior
  increment is unchanged. New code is safe Rust with Results, private store APIs
  and narrow typed state. No lint allowance, C++ implementation fallback,
  optimizer/backend policy change or fixture recognition was added.

Rust owners are `sema/function_names.rs` and the existing `declarations`,
`analyzer`, `declarators`, `graph`, `lookup`, `exceptions` and `dump` owners.
Lexical token enums now supply Hash consistently with their existing canonical
Eq identities; rendering remains a source-facing syntax view. Source map rows
record the reviewed slice and later expression/overload/initialization owners.

### Validation and artifacts

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa6-function-names/`.
Candidate cppgm++ SHA-256:
`4b5940f97494d41a7dca2894562792636258bd77b8182fe256e41b7cae812bbe`. Frozen C++ tool remains
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command/review | Result | Evidence under artifact root |
| --- | --- | --- |
| `make build` | PASS | `build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6` | 121/121 exact | `stage-final.log`, `stage-summary.json`, `fixed-traces/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py course pa6` | 648/648; no failure marker | `course.log`, `course-course-result.json`, `course-counts.txt`, `course-trace.jsonl` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa6` | PA1–PA5 exact gates and full original courses PASS | `prior.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa6` | PASS | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa6` | 121/121 preserved | `preserve.log` |
| `make lint` / required `clone_quality.py` | fmt, strict Clippy, debug/release suites and source audit PASS | `quality-final.log` |
| `make audit-rust` | 113 files; zero findings | `audit-current.log` |
| Callable declaration probes | 123 exact; four reproducible frozen-oracle crashes separately reported | `probe-final.log`, `probe-summary.json`, `probes.jsonl`, `oracle-crashes.json` |
| Template throw probes | 34/34 exact | `throw-probes-final.log`, `throw-summary.json`, `throw-probes.jsonl` |
| Earlier special-member probes | 111/111 exact | `special.log`; original `pa6-special-members/probes.jsonl` |
| Earlier expression-key probes | 79/81 exact; only the prior two dependent-qualified boundaries remain | `keys.log`; original `pa6-expression-keys/probes.jsonl` |
| Executed new regression lane | 157/157 exact | `regression-summary.json`, `regression-references.json`, `regressions.jsonl` |

Added 157 new oracle-derived `pa6/tests/general/875-function-name-*.t` inputs
and new refs/statuses in the original executed course lane. Existing frozen
inputs/refs are unchanged. The stage failing case list is empty; every prior
passing case remains passing. Supplementary counts are evidence, not the
completion criterion. No harness code changed. PA33-wide responsibility and
release/debug proof, PA34 object/tool fixed-point equality and the exclusion of
PA35 objectives remain as recorded in the contract.

### Resource observations

Six supported fixed workloads compare exactly before measurement. Warm A/A
calibration followed by ABBA/BAAB preserves individual wall/RSS observations,
input/output/binary hashes and verifies the full dump hash after each invocation.
Startup and dump I/O are included; some host validation activity may contribute
noise. These costs are separate from externally visible bytes, not a runtime
performance-equality or optimizer gate. `measure.py`, `measurements.json`,
`measure-trace.jsonl` and `measure.log` retain the experiment.

| Shape | Groups | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| operator-identities | 1000 | 31.90 / 13576 | 86.22 / 13488 |
| operator-identities | 4000 | 111.81 / 37496 | 261.46 / 43796 |
| operator-identities | 16000 | 500.90 / 133236 | 970.81 / 164060 |
| template-throw-sets | 1000 | 55.02 / 19140 | 120.33 / 19172 |
| template-throw-sets | 4000 | 185.95 / 57064 | 375.10 / 66100 |
| template-throw-sets | 16000 | 677.98 / 213268 | 1291.35 / 255868 |

A fixed 700-nested-class input combines operator bodies and function-template
throw sets. From a 1 MiB initial native stack, the protected comparator reports
`match: true`, `stack_qualification.passed: true`, a shared 16 MiB retry, both
exits 0 and no stream/artifact differences. Complete successful dump SHA-256:
`5e451d0b4f011a5ccb119ece1d4975885ccc20aac24ab7586f55ce7409e5e81a`.
`nested-summary.json` links preserved initial/retry evidence. Genuine guards
remain unchanged; native cutoff depths and diagnostic positions were not
calibrated or required to match.

### Baseline failures and remaining acceptance boundary

Four malformed namespace destructor redeclaration inputs reproducibly crash
frozen C++ with SIGSEGV (three direct replays each, empty streams):
`namespace-destructor-duplicate`, `namespace-destructor-first-true`,
`namespace-destructor-first-false`, `namespace-destructor-qualified`.
Their minimal sources, returns and raw comparator evidence are preserved in
`oracle-crashes.json` and `probes.jsonl` mismatch directories. The source is
outside valid C++ destructor placement; the reference's implicit-destructor
redeclared-specification path assumes a class owner there. Rust preserves
safe first-declaration/factory behavior and never reproduces that invalid
memory access. These are **baseline issues**, not matched rejections or
qualified native exhaustion. They were not added as oracle-success/rejection
fixtures; no frozen code, policy or reference was changed. Full clone signoff
must not silently count such probes as passes.

The earlier 27-case responsibility census now has 17 exact matches; all eight
incoming name/factory gaps and four template throw gaps are closed, including
`destructor-namespace-written-type` (previously Rust status 70). Ten exact
remaining IDs are:

- `function-template-noexcept-qualified-value`;
- `class-inheritance-simple`, `class-inheritance-unknown`, `class-bit-field`;
- `new-runtime-array-names`, `new-runtime-array-unknown`, `new-array-class-demand`;
- `member-pointer-declaration`, `function-trailing-return`,
  `variable-placeholder-auto`.

`boundaries.log` and the original `pa6-expression-keys/boundaries.jsonl` retain
their current receipts. The separate key cases `dependent-qualified-value`
and `dependent-qualified-type`, and legacy `inner-qualify`,
`function-different-shape`, `function-template-ordinary`, remain unresolved.
The next dependency is a typed dependent qualified-name/type interface, then
continued source responsibility classification and fixes for class/declarator
and name-only allocation work. Do not equate declaration collection with later
expression typing, overload resolution, layout or instantiation, or classify
all remaining probes away without direct handout/source evidence. Full-stage
acceptance and Ralph's existing goal remain open; no goal or Ralph state was
created, reset, paused, advanced or replaced.

## Full-stage acceptance closure: allocation names and stage boundaries

Audit target `pa6 full-stage`, phase `audit`; stage base remains
`70e745588987998ee1e5bde80b3861541eab753d`. Incoming clean ledger revision:
`83c4d4f7c7597766f0341986590fe9e5430dd0fb`; incoming implementation audit marker:
`b635af3fa3484170186c324c243bf339a17f86dd`. Reviewed and validated implementation:
`5851cab157eacc66f410f3275b0c06bd2f8d1c41`.

**PA6 acceptance: PASS for the cumulative PA1–PA6 handout contract.** This
closes the previously withheld stage audit after fixing the remaining due
allocation-name group and individually classifying the wider compiler census.
It does not certify the complete Types mode on inputs requiring later-stage
features. Those remain explicit implementation dependencies, with their actual
mismatches retained below. No fixed case, reference, baseline, control checker,
acceptance policy or Ralph state was changed. Ralph owns stage advancement.

The audit follows CLONE_CONTRACT.md's instruction not to force future-stage
capabilities into early gates. PA6 README's Input Contract distinguishes syntax
acceptance from its semantic slice; its Out Of Scope excludes overload sets,
expression typing, general queries, template-aware semantic disambiguation,
specializations/instantiations, non-type binding and full constant evaluation.
The type notation in `pa6/scopes-and-types.md` specifies the supported forms.
These existing boundaries, rather than the outcome of a probe, determine what
is due. The historical ledger entries remain checkpoint evidence; this entry
supersedes their open-boundary statements for PA6 acceptance only.

### Source responsibility closure and completeness reasoning

Read the repository instructions, clone contract, specification, clone plan,
full handout and scopes/types guide, source map and historical ledger. The
frozen source revision remains `0fe8e3b4bde9929c2419fb7906028df83bb17de5`.
The following groups trace the due C++ control flow to Rust owners, including
behavior beyond the initial 121 fixtures. The preceding audit entries record
individual diagnostic, arithmetic and ordering findings and their fixes.

| Due responsibility and reviewed C++ owner | Rust owner and acceptance reasoning |
| --- | --- |
| `cppgm++.cpp` types source loop | `dev/bin/cppgm.rs`, `driver/ast.rs`: flags/output handling, input order, wrappers, independent units, located failures and retained AST mode. One preprocessor/cursor/AST is consumed in process by Rust sema and dump. |
| `analyzer.cpp` dispatch, namespace/alias/using/declaration linkage | `analyzer.rs`, `lookup.rs`, `graph.rs`: reopened scopes share identity; aliases distinguish name entities from target scope; indexed using edges preserve source ordinals, common-ancestor nomination, inline lookup, cycles and later namespace additions. Special callable imports share the qualified lookup owner. |
| `analyzer_types.cpp`, `type_table.cpp` specifier/declarator formation | `specifiers.rs`, `declarators.rs`, `types.rs`: supported fundamental/cv combinations and invalid combinations; pointer/reference/array/function precedence; alias collapsing and array qualification; source parameters versus canonical signatures and adjusted parameter objects; sole void, varargs and qualifier rules. Matching uses typed identity, not rendered output. |
| `analyzer_declarations.cpp` declaration/entity matching | `declarations.rs`, `data_members.rs`, `function_names.rs`: publication order, compatible objects/functions, completed arrays, initialized references, void objects, qualified enclosing scopes, static/linkage/constexpr agreement, duplicate definitions, declared/source labels and explicit function body ownership. Unsupported overload construction is a separate later boundary. |
| `analyzer_classes.cpp` class/enum/anonymous declaration paths | `declarations.rs`, `enumerations.rs`, `anonymous_types.rs`, `special_members.rs`: class keys/forwards/injected names, scoped opaque enums, elaborated hidden tags, enum underlying selection, anonymous token-ordinal names/adoption/storage and member conflict ordering. Complete-class events precede member body/exception lookup. Types-mode special-member declaration facts do not demand semantic construction plans. |
| `analyzer.cpp::template_declaration/template_parameters`, `class_templates.cpp`, `template_names.cpp`, `template_placeholders.cpp` declaration-head slice | `templates.rs`, `expression_keys.rs`, `name_keys.rs`: owned type/template-template parameter scopes, positional canonical entities distinct from source names, same-shape heads/defaults/alias-template declarations and structural noexcept compatibility. Qualified dependent members and instantiated specializations remain outside PA6. |
| `constant_evaluator.cpp`, `integral_arithmetic.cpp`, enum selection | `constants.rs`, `integral.rs`, `enumerations.rs`: literal/character conversion, LP64 signedness/rank/promotion, sign extension/truncation, unsigned wrapping, checked signed arithmetic/division/shifts, short-circuit unselected operands, scoped-enum restrictions, const/constexpr/reference reads, positive bounds and static assertions. Failure order remains distinct from a nonconstant result. |
| `analyzer_types.cpp` type-forming queries and `layout.cpp` required size dependencies | `queries.rs`, `layout.rs`: lookup-based sizeof disambiguation, supported decltype entity/lvalue forms, scalar/array/enum/plain-class sizes, incomplete/cyclic/too-large diagnostics, ordered union/member placement and demand-only layout memos. Extended object/query interpretation remains later. |
| `analyzer_exceptions.cpp` ordinary/member exception states | `exceptions.rs`, `exception_operands.rs`, `template_exceptions.rs`: first/compatible redeclaration facts, namespace declaration-point visibility, complete-class deferred member specifications, prototype parameters, unused ordinary checks at unit end, dynamic throw formation, builtin allocator first-declaration policy and template pending-operand/category rules. Exceptions remain outside canonical function identity. |
| `analyzer_bodies.cpp`, `analyzer_handlers.cpp`, `analyzer_range_for.cpp` Types branch | `statements.rs`, `analyzer.rs`, `expressions.rs`: function/block/condition/loop/catch scope and guarded redeclaration rules, borrowed statement readings, complete-class deferred bodies, name-only operand order and suppression of unevaluated/initializer class demand. Semantic return/conversion/lifetime/lowering branches are future owners. |
| `semantic_dump.cpp`, graph/entity/type records | `dump.rs`, `graph.rs`, `types.rs`: deterministic ordered bindings/child scopes, source type spelling, completed shared entity types, hidden builtin/anonymous facts and declaration labels independent of first-entity body labels. No output normalization is used. |

The final finding was `analyzer_bodies.cpp::new_expression` (line 1742): a
bounded first root array suffix is a runtime expression, not a constant array
bound. Rust previously returned status 86 for every array allocation, including
valid PA6 name traversal. `declarators.rs::allocation_type` now reuses the
existing borrowed root-suffix view with that first suffix skipped; remaining
suffixes still form types normally. `expressions.rs` schedules the bound as an
ordinary operand, preserving placements/initializer LIFO visitation and empty,
evaluated class-constructor demand. Nested expression walks protect their
worklist base, so type formation does not consume pending outer operands.
Unbounded roots retain the original unknown-bound type; structural key formation
continues to report an unknown unbounded-array key. No AST copy, mutation,
text reparsing or fabricated type fact for the stripped view was introduced.

All 46 targeted probes match exit, stdout, stderr and successful dump bytes:
positive/nonconstant/negative/zero/floating first operands; inner constant,
nonconstant, invalid and void dimensions; placement/type/initializer diagnostic
precedence; aliases/qualified/class/incomplete types; nested and parenthesized
allocations; unevaluated operands and repeated constructor demand. New
`890-allocation-name-*.t` cases and oracle-derived reference pairs run in PA6's
original course lane. These exercise name-only behavior without asserting later
allocation typing, overload selection or native allocation lowering.

### Exact remaining census IDs and owning boundaries

Fresh receipts in `census-summary.json` retain every mismatch. They are not
counted as passes, converted into generic rejections or removed from evidence.
The 27-case census improves from 17 to 20 exact matches; its three due allocation
failures are closed. Each remaining ID has the following existing boundary:

| Remaining ID(s) | Source dependency and handout ownership |
| --- | --- |
| `class-inheritance-simple`, `class-inheritance-unknown` | C++ `analyzer_classes.cpp::base_classes` adds base lookup/relationships. PA6's required named class/forward scope model has no base relationship; PA11 README lines 15 and 273 explicitly introduce single inheritance. Both successful formation and unknown-base diagnosis belong there. |
| `class-bit-field` | C++ class bit-field declaration/width/storage facts; PA11 README lines 265–273 requires the bit-field model. It is absent from PA6's supported declaration/type notation. |
| `member-pointer-declaration` | C++ member-pointer type owner/member identity; PA6's type vocabulary has ordinary pointers/references, not member pointers. PA9 README lines 131–135 introduces the standalone ABI form; PA22 README lines 11 and 171 requires source member-pointer type formation. PA11 explicitly excludes member pointers. |
| `function-trailing-return` | C++ `analyzer_types.cpp` forms a placeholder, then replaces it using the trailing type and its prototype scope. The supplied `auto f()->int` is not return deduction, but still requires the placeholder declarator interface, absent from PA6's fundamental vocabulary. PA14 README lines 286–287 explicitly requires trailing-return declarator forms. It remains recorded there rather than being mislabeled as a PA20 deduction test. |
| `variable-placeholder-auto` | C++ auto initializer deduction, outside PA6 expression typing/type vocabulary; PA20 README lines 10 and 174–180 requires it. |
| `function-template-noexcept-qualified-value`, `dependent-qualified-value`, `dependent-qualified-type`, `inner-qualify` | C++ dependent qualified name/type and structural placeholder lookup, not ordinary lookup through an existing class/namespace/enum scope. `inner-qualify` is `typename TT::type` for an uninstantiated template-template parameter. PA14 README lines 227–230 and PA15 lines 228–231 explicitly introduce dependent names/qualified lookup. PA6 excludes template-aware semantics and instantiation. |
| `function-different-shape`, `function-template-ordinary` | Both introduce distinct functions with one ordinary name and require the C++ overload set/template-overload representation. PA6 explicitly excludes overload sets and resolution; PA7 and PA14 own those interfaces. Same-shape template redeclarations, which require no distinct overload set, already match. |

The production Missing-site review agrees with these boundaries: packs/non-type
binding/specialization/instantiation/dependent lookup are PA14–PA15 dependencies;
call conversions/default application and extended operand typing are PA7 or
later; constexpr function interpretation is PA16; virtual override/layout,
friend/mutable and extended/vector type semantics are later class/extension
features. PA6's explicitly required storage/function specifiers, ordinary
exception slice, supported integral constants and type-forming queries do not
enter those unfinished branches. No currently required PA6 case is accepted
through a stub or delegated compiler path.

The four malformed nonmember destructor redeclaration probes described in the
preceding entry remain reproducible frozen-oracle SIGSEGV baseline issues.
PA6's Input Contract excludes ill-formed inputs it is not required to diagnose;
nonmember destructors are not a required declaration form. Thus they do not
supply language-result requirements for this stage, and are not qualified stack
exhaustion or successful matches. Their raw crash evidence is retained at
`pa6-function-names/oracle-crashes.json`. The complete compiler port must retain
that baseline report; safe Rust must never reproduce the invalid memory access.

### Architecture, validation and resources

The audit traces the driver through one owned source/cursor/AST to unit-owned
semantic stores and iterative dump output. Checked compact IDs and private
store APIs retain typed names/types/entities/scopes. Canonical type interning
compares exact structural records after fingerprint lookup; source declarations
retain parameter and presentation facts separately. Lookup caches no mutable
namespace result. Exception, constant, layout and template facts remain under
their recorded unit/entity/head/environment owners; no new cache is introduced.
Transactional parser selection stays bounded and refers to original AST IDs.
Worklists restore nested bases and structured Results propagate errors. The new
allocation view borrows syntax and retains explicit demand. There is no unsafe
code, C++ fallback, dependency addition, fixture tailoring, production phase
process execution/environment read, blanket lint allowance or mechanical split.
Per-function backend ownership and existing optimizer/register-allocation
policies were not changed.

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa6-allocation-names/`.
Candidate cppgm++ SHA-256:
`d3accf86f9c26a3082ec4b05474cfe8aabc42694f1e852931df947428b8c8c2e`.
Frozen C++ tool SHA-256 remains
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Validation command/review | Result | Artifact |
| --- | --- | --- |
| `make build` | PASS | `build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa6` | 121/121 exact PASS | `stage.log`, `stage-summary.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py course pa6` | 694/694 PASS, exit 0, no failure marker | `course.log`, `course-result.json`, `course-counts.txt`, `course-trace.jsonl` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa6` | PA1–PA5 exact gates and original full courses PASS (58, 28, 20, 112, 296 course groups) | `prior.log`, `prior-course-results.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa6` | PASS | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa6` | 121/121 preserved | `preserve.log` |
| `make lint` / `clone_quality.py` | rustfmt, strict Clippy, debug/release tests and file audit PASS | `quality.log` |
| `make audit-rust` | 113 files, zero source/function/nesting findings | `audit.log` |
| New allocation probes | 46/46 strict matches | `probe.py`, `probe-summary.json`, `probes.jsonl`, `new-fixtures.json` |
| Historical responsibility probes | anonymous 87/87, constants 64/64, constructors 135/135, exceptions 67/67; templates 97/100 and boundary 20/27 with the above future dependencies | `review_census.py`, `census-summary.json`, `census/*/trace.jsonl` |

Fixed-surface parity, original full courses and earlier-stage preservation are
all required here; additional passing fixtures alone were not the completeness
argument. New references were generated only for new cases. No generated dumps,
logs, caches or Cargo outputs were committed.

Warm C++ AA calibration followed by ABBA/BAAB measurements retain individual
wall/RSS observations, source/binary/output hashes and exact comparator receipts
in `measure.py`, `measurements.json`, `measure-trace.jsonl` and `measure.log`.
Costs include startup and dump I/O; they are observations independent of dump
byte equality and do not impose PA35 optimization objectives.

| Shape | Groups | C++ median ms / peak RSS KiB | Rust median ms / peak RSS KiB |
| --- | ---: | ---: | ---: |
| runtime-array-names | 1000 | 18.01 / 9332 | 46.34 / 8744 |
| runtime-array-names | 4000 | 58.96 / 19896 | 138.29 / 25488 |
| runtime-array-names | 16000 | 226.83 / 64440 | 513.22 / 93112 |
| class-array-demand | 1000 | 19.65 / 10776 | 50.00 / 10008 |
| class-array-demand | 4000 | 69.53 / 24844 | 151.32 / 29676 |
| class-array-demand | 16000 | 287.18 / 84836 | 564.41 / 108684 |

The fixed 700-nested-class allocation-name probe begins at a 1 MiB native stack.
The protected adapter reports `match: true`, `stack_qualification.passed: true`:
both tools succeed at the same 16 MiB retry with no differences and identical
dump SHA-256 `9ac21793e89d0bb55fb5da61399eeb04deeb4fe690116bc90ff827250ad79fe6`.
`nested-summary.json` links the original and larger-stack evidence. No native
exhaustion cutoff or diagnostic position was reproduced/calibrated.

Next dependency boundary: PA7 expression typing/conversion/overload facts, then
the explicitly listed later declaration/template/class capabilities. The source
map remains partial for those C++ files' complete compiler responsibilities;
this is deliberate and is not a claim that the full compiler is finished.
The committed audit and `handoff.json` record a clean checkout. Ralph's existing
phase goal may now be completed; no goal was created/replaced and no Ralph state
or stage-advancement policy was edited.


## Qualified declarator ownership correction from the PA17 checkpoint audit (2026-10-17)

Reviewed implementation: `0c08c8a9be6695ed4509a18f521b2455fe894fb9`.
The PA17 audit's malformed outside-member owner probes exposed the missing
namespace/class-only qualifier check shared by ordinary declarators. Frozen
C++ `analyzer_types.cpp::declarator` rejects an enum scope before creating the
member lookup bridge or analyzing suffixes. Rust
`declarators.rs::declarator_environment` now does the same, preserving exact
rejection text, name location and output state. No enum member-declaration
behavior is newly accepted. Source responsibilities are updated in the map.

The earliest owning regression is
`tests/spec/300-enum-alias-declarator-owner-bad.t`, with its new
`EXIT_FAILURE` expectation sidecar; no existing reference was changed. The
protected exact `--emit-types` comparison passes, and the full PA6 course
executes the new input and passes **741/741**, with no failure marker. The
unchanged PA6 fixed manifest passes **121/121**, as part of all 2,825 earlier
fixed cases and all 16 earlier courses. The production build, strict Clippy,
432 library tests in both profiles and 320-file companion audit pass.

Primary audit and resource/ownership evidence are in the latest section of
`docs/port/pa17.md`. Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa17-checkpoint-audit-j`;
`regressions.jsonl`, `prior-pa6-results.json`, `prior-pa6-course/`,
`evidence-audit.json` and `handoff-commit.json` retain the results. This is an
earlier-stage shared-declarator correction within the PA17 partial audit;
PA6's historical stage base/audit records above are unchanged.
