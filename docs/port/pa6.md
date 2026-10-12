# PA6 port ledger

Stage base commit: `70e745588987998ee1e5bde80b3861541eab753d`
Last audited commit: INITIAL (PA6 has no audited implementation checkpoint yet).
Implementation does not advance this field; Ralph owns checkpoint/final audits.

## Contract and ownership

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

## Evidence

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/port/pa6-declarations/`.
Candidate implementation commit: recorded in `handoff.json` after the implementation commit.
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

## Supported hot-path costs

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

## Remaining exact fixed-manifest case IDs

All paths below are relative to `pa6/`; the JSON summary retains the full
`clone/pa6/` identities and matching trace ownership.

```text
tests/general/100-namespace-nonstatic-anonymous-union-bad.t
tests/general/200-adjusted-parameter-sizeof-bound.t
tests/general/200-anonymous-struct-declares-nothing-bad.t
tests/general/200-bad-opaque-unscoped-enum.t
tests/general/200-bad-sizeof-incomplete-class.t
tests/general/200-bad-static-assert.t
tests/general/200-class-constants-and-using.t
tests/general/200-class-scope-bad-static-assert.t
tests/general/200-constexpr-variable.t
tests/general/200-cv-alias-constant-array-bound.t
tests/general/200-qualified-decltype.t
tests/general/200-qualified-member-scoped-enum-definition.t
tests/general/200-qualified-scoped-enum-definition-lookup.t
tests/general/200-qualified-unscoped-enumerator.t
tests/general/200-scoped-enum-integral-comparison-bad.t
tests/general/200-scoped-enum-qualified-enumerator-collision.t
tests/general/200-scoped-enum-qualified-enumerator.t
tests/general/200-scoped-enum-unscoped-enumerator-initializer-bad.t
tests/general/200-scoped-enumerator-from-other-enumeration-bad.t
tests/general/200-scoped-enumerator-narrowing-bad.t
tests/general/200-sizeof-qualified-type-idexpr.t
tests/general/200-sizeof-type-like-id.t
tests/general/200-template-template-parameter.t
tests/general/200-unscoped-enum-qualified-enumerator.t
tests/general/200-using-declaration-values.t
tests/general/200-using-imported-scoped-enum.t
tests/general/300-complete-class-exception-specification.t
tests/general/300-cross-tu-constant-not-visible.t
tests/general/300-declaration-forms-valid.t
tests/general/300-dynamic-exception-specification-different-set-bad.t
tests/general/300-dynamic-exception-specification-set.t
tests/general/300-noexcept-function-pointer-declarator.t
tests/general/300-scoped-enum-cast-constant.t
tests/general/300-unused-nonconstant-exception-specification-bad.t
tests/general/300-unused-unknown-exception-name-bad.t
tests/spec/200-anonymous-type-typedefs.t
tests/spec/200-const-int-static-assert.t
tests/spec/200-constant-short-circuit.t
tests/spec/200-decltype.t
tests/spec/200-elaborated-enum-type-specifier.t
tests/spec/200-enum-scoped.t
tests/spec/200-enum-unscoped.t
tests/spec/200-namespace-anonymous-class-array-object.t
tests/spec/200-namespace-anonymous-union-injected-members.t
tests/spec/200-opaque-scoped-enum.t
tests/spec/200-parenthesized-enumerator-decltype.t
tests/spec/200-sizeof-alignof-bounds.t
tests/spec/200-template-parameter-scope.t
tests/spec/200-using-directive-values.t
tests/spec/300-elaborated-undeclared-enum-bad.t
tests/spec/300-enumerator-invalid-initializer-bad.t
tests/spec/300-opaque-enum-redecl-underlying-bad.t
tests/spec/300-signed-constant-overflow-bad.t
tests/spec/300-template-template-inner-parameter-scope-bad.t
tests/spec/300-using-declaration-template-id-bad.t
```

## Handoff

The next coherent group is the typed integral constant evaluator: retain
width/signedness and enum identity, perform checked signed versus explicit
unsigned target arithmetic, and record const/reference object values against
entities. Add sizeof/alignof/decltype type facts and enumerator/assertion
analysis on the same AST. Then complete anonymous type/union naming,
template-parameter scopes and ordinary exception specifications at their
owning events. Do not replace this boundary with diagnostic stubs.

Independent audit questions: verify source/canonical parameter distinction,
lookup nomination/shadowing and identity convergence, array/entity completion,
member-body event order, exact operand worklist order and class demand
suppression. Review the deferred member initializer and prototype/default
argument dependencies before extending the declaration surface. Recheck
qualified class/member and block-extern rules alongside entity-owner/linkage
facts. Confirm the protected nested-stack receipt and the complete earlier-stage
ladder, and leave last-audited commit and advancement to Ralph.
