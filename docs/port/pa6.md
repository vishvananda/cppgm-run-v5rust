# PA6 port ledger

Stage base commit: `70e745588987998ee1e5bde80b3861541eab753d`
Last audited commit: INITIAL (PA6 has no audited implementation checkpoint yet).
Implementation does not advance this field; Ralph owns checkpoint/final audits.

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
