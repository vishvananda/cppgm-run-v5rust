# PA7 port ledger

Stage base commit: `9a851dcedb1e56bbc6585c84c5990f6ef1a1c372`.
Current implementation group base: `9f47dc928ae7e5a28edfdf050b15a55a2a84d753` (clean call/conversion acceptance audit entry).
Last audited implementation commit: `01cdba770e15010228180780457d1d7f5b47faca` (call/conversion/association source, complete due lanes and resource evidence reviewed; full-stage acceptance pending resumed audit of the refreshed 883778fd baseline). Previous audit: `743155c1cd0d29b13aa35346a67a4ab5f95cd92e`.


## Owner-authorized oracle resolution — 2026-10-13

The frozen C++ reference is now `883778fdfa45d523cf9e135d15169af976f451a8`
(PR #68, CI passed and merged into main). The null-type operator-candidate bug
reported below is fixed. The sixteen retained reproducer sources all pass the
protected adapter's exact comparison; a crash is still an invalid outcome.

The protected PA7 gate retains all 198 earlier identities and adds the sixteen
new rejection regressions (214/214 passing); the full course lane passes 346/346
and completed PA1–PA6 differential and course lanes remain green. PA12 adds two target-directed operator
regressions in its owning lane (318 total). C++ source/tool hashes and contract
pins are refreshed together, while the Rust compiler source and ordinary
acceptance rules are unchanged. See `baseline.md` and
`artifacts/observer/baseline-883778fd/` for the update and verification evidence.
The earlier impasse record is historical; resume the PA7 audit against this
reference and verify its acceptance obligations rather than repeating the
old crashing-oracle disposition. Ralph still owns goal and phase advancement.

## Completion audit: frozen baseline impasse — 2026-10-13

Reviewed clean entry: `cfb8075f846846faff12c69188620b2a474adc06`.
The last audited implementation remains
`01cdba770e15010228180780457d1d7f5b47faca`; all nine reviewed Rust source
hashes and both compiler binary hashes match the preceding validation snapshot.
The preceding goal turn was **progress**, with six corrected behavior groups,
nineteen executed regressions and committed source/evidence. This review does
not replace those requirements with a fixture-only completion claim.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-baseline-disposition-20261013/`.
`completion-audit.json` reconciles current sources with the preceding complete
command/probe/resource evidence and records the remaining dependency.

- Fresh protected `preflight pa7`: PASS, exit 0 (`preflight.log`).
- Fresh required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7`:
  PASS, exit 0, all 198 identities (`stage-required.log`, `fixed-manifest.json`).
- Unchanged-source evidence retains original course 330/330, independently
  preserved 311/311 entry identities, PA1–PA6 fixed/full course lanes, rustfmt,
  strict Clippy, debug/release workspace tests and the authoritative file audit
  (136 files, zero findings), plus 2,414 exact supplementary probes. Their
  commands, receipts and resource costs remain in the immediately following
  checkpoint and `../pa7-conversion-audit-20261013/`; no new compiler code or
  regression was introduced in this completion review.
- Fresh protected comparisons reproduce all sixteen failures listed below:
  `oracle-issue-probes.jsonl` has sixteen rejected comparison receipts, each
  frozen-oracle exit -11. Direct revalidation at 16 and 64 MiB reproduces
  32/32 SIGSEGV attempts, no streams and empty generated outputs, retained
  in `oracle-crash-reproduction.json` and `oracle-issues.log`.

The actual pinned control flow confirms the failure before any language result:
`analyzer_expressions.cpp::MayOverload` admits an unknown operand type;
`combine` calls the operator search before `check_resolved`, and
`analyzer_operators.cpp::operator_candidates` passes the left unresolved
function's null type to `conversions.h::ClassOf`, which dereferences it.
These inputs have no declared overloaded operator or user-defined conversion;
they therefore cannot be dismissed merely by naming PA7's later class/operator
boundary. The comparator reports infrastructure failures, not valid rejections.
No supplementary failure is counted as a passing or expected-failure case.

CLONE_CONTRACT.md requires: “A reproducible oracle bug or unstable output is
a baseline issue to report, not permission to correct references in this run.”
Its stack qualification requires identical **successful** results at a common
larger stack. A SIGSEGV with no guard diagnostic and no successful larger-stack
result fails that qualification. The two genuine stack probes in the preceding
checkpoint remain successfully qualified at 16 MiB; their implementation
exhaustion cutoffs and diagnostic positions are not acceptance defects.

The same frozen baseline condition is recorded across three consecutive goal
turns: declaration/linkage audit (`743155c`), call/conversion audit (`01cdba7`)
and this completion review. Rust-owned normal-result findings and due lanes
have been resolved and validated; no authorized Rust change can supply the
missing C++ oracle outcomes. Emulating a crash, changing protected reference
inputs or accepting the failed receipts would violate the contract.

Full exact-clone acceptance remains **unproven**, and the existing goal's blocked
audit threshold is satisfied. The next dependency is an approved resolution of
the reference baseline that provides valid comparison outcomes for the same
sixteen sources. No fixture, reference, oracle, gate policy or Ralph state was
edited; the goal is neither replaced nor marked complete. PA33 implementation
census/release/debug obligations, PA34 object/tool equality and PA35 exclusions
retain their original boundaries.

## Call/conversion and association acceptance audit checkpoint — 2026-10-13

Reviewed entry: `9f47dc928ae7e5a28edfdf050b15a55a2a84d753`.
Reviewed implementation commit: `01cdba770e15010228180780457d1d7f5b47faca`.
Stage base remains `9a851dcedb1e56bbc6585c84c5990f6ef1a1c372`.
The preceding goal turn made authoritative progress in the declaration/linkage
implementation `743155c1cd0d29b13aa35346a67a4ab5f95cd92e` and its reviewed
handoff `9f47dc9`. This turn verified the clean checkout, preceding source
hashes, source ledger and current protected checks before extending the audit.
**Six Rust findings are resolved; full exact acceptance remains unproven on
the sixteen reproducible frozen-oracle crash probes listed below.** No goal,
Ralph state, protected policy, oracle source or existing reference was changed.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-conversion-audit-20261013/`.
Final Rust cppgm SHA-256: `09fd062faca2cebd2d54da1d468100e147bd4ac66082877ede44bcaf5b53a1b3`.
Frozen cppgm++ SHA-256: `9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

`checks-final.json`, `reviewed-source.json`, `responsibility-census.json`,
`preceding-source-review.json`, `course-preservation.json` and `handoff.json`
retain the source/build/validation evidence. Initial mismatches, final inputs,
raw streams and protected output receipts remain under that root. No generated
output, Cargo target, scratch script or log is committed.

### Source responsibilities and completeness reasoning

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, the complete PA7 handout,
`docs/rust-clone-plan.md`, source map and cumulative stage checkpoints. The
handout defines the due procedural slice; the fixed manifest and all original
course cases remain mandatory, including inherited extensions. Later class
calls, user conversions, arbitrary template bodies, floating object execution,
exceptions and backend work remain explicit future owners, not accepted Rust
rejection stubs. No PA33 release/census or PA34 object proof is claimed at PA7.

Consulted frozen C++ `analyzer_calls.cpp` directly, including `call`,
`result_category`, association/argument-dependent lookup, `indirect_call`,
`implicit_sequence`, `resolve`, `compare_candidates`, `best_viable`, default
visibility and `convert_arguments`; `conversions.cpp/.h`'s scalar/reference
formation, qualification, integral promotions and sequence comparison;
`analyzer_expressions.cpp`'s binding/function designation, unary/address typing,
`select_overloaded` and `resolve_overloaded`; and `semantics_dump.cpp/.h`'s
expression type, unary, id and call presentation. Also reviewed
`analysis_records.h`, `statement_facts.h`, `semantic_graph.h` and frontend source
composition against the Rust owners. The source map now identifies the due
analysis working records and correctly leaves range expansion with PA20.

The cumulative source census reconciles the following responsibilities with
this review and the earlier reviewed checkpoints, rather than treating green
fixtures as completeness proof. Thirteen of the preceding declaration audit's
fifteen source hashes remain unchanged; the changed operator and printer
owners are reviewed here. `responsibility-census.json` retains all 121 relevant
entry/type/syntax/sema source rows, including their explicitly partial future
responsibilities. This is stage ownership review, not PA33's complete-port census.

| C++ responsibility | Rust owner / evidence and retained boundary |
| --- | --- |
| `cppgm++.cpp` emit modes, per-input wrappers, output failures and diagnostics | `dev/bin/cppgm.rs`, `driver/ast.rs`; one source/syntax/graph owner per unit, command-line source order, shared AST/types/semantics modes; other compiler modes return 86 |
| Shared preprocessing, syntax, locations, name interning and rollback | `preprocess/`, `syntax/`, `support/source.rs`; PA1–PA6 exact/full lanes, original typed AST and prior rollback/storage audits retained |
| `analyzer.cpp`, `analyzer_declarations.cpp`, lookup and canonical type/declarator owners | `analyzer.rs`, `declarations.rs`, `linkage.rs`, `lookup.rs`, `graph.rs`, `types.rs`, `declarators.rs`; indexed source visibility, entity/denotation distinction, immutable type identities and original declaration facts; preceding declaration review preserved |
| `analyzer_bodies.cpp` procedural scopes/control/returns | `statements.rs`, `procedural_statements.rs`; original StmtIds, function-local context and typed condition/conversion facts; procedural recheck retains 411/411 due cases |
| Scalar expressions, casts and integral arithmetic | `scalar_expressions.rs`, `scalar_operators.rs`, `scalar_casts.rs`, `integral.rs`; target arithmetic and operand/failure order; preceding operator/address review plus 1,940 conversion and 198 promotion/array-reference probes |
| `analyzer_initialization.cpp` scalar/list/array and defaults | `initialization.rs`, `default_arguments.rs`, `declarations.rs`; original expression/list IDs and selected conversions, bound completion, zero tails, declaration-time visibility; 78/78 initialization rechecks |
| `conversions.cpp/.h`, ordinary calls and overloaded designation selection | `scalar_conversions.rs`, `procedural_calls.rs`, `function_resolution.rs`, `call_lookup.rs`; six findings below; 232/232 function, 24/24 depth and 20/20 failure-order probes |
| Integral/address constants and builtin queries | `constants.rs`, `constant_addresses.rs`, `constant_pointer_operations.rs`, `constant_mutations.rs`, `builtin_calls.rs`, `queries.rs`; short-circuit/unevaluated effects, symbolic identities and deliberate target arithmetic; 147/147 address and 86/86 query rechecks; general floating/frame execution is later |
| Inherited anonymous storage, direct designation, scalar construction, nonvirtual base/member-pointer and signature-template extensions | `anonymous_types.rs`, `data_members.rs`, `construction.rs`, `copy_initialization.rs`, `inheritance.rs`, `member_expressions.rs`, `function_templates.rs`, `function_demands.rs`; retain audited source/demand identities and the exact fixed cases, plus 70/70 signature-template rechecks; full class/operator/template/interpreter paths remain later owners |
| `semantic_dump.cpp/.h`, `semantics_dump.cpp/.h` | `dump.rs`, `semantics_dump.rs`; source then demand traversal, recorded result/operand/default/conversion facts, declared function result types and exact unresolved unary output; no semantic lookup, substitution or demand during printing |
| `analysis_records.h` working frames and candidates | `procedural_statements.rs`, `procedural_calls.rs`, `analyzer.rs`, `copy_initialization.rs`; ephemeral body/query owners, compact IDs, selected outcomes and cleared scratch; lambda/goto/deferred-access/hosted frames are later |

### Findings and translated behavior

- Genuine overloaded function arguments rank as exact target matches before the
  selected expression's actual scalar binding is applied. Rust previously
  filtered an overloaded address-to-function-reference candidate too soon,
  changing both ambiguity/selection and failure bytes. Each candidate now owns
  its ranking conversion, optional application conversion and selected BindingId.
  Selection publishes that binding once; an unavailable application fails at
  the argument after ranking, without rerunning lookup or target selection.
- Reference ranking retains typed `ReferenceSource::FunctionLvalue`, including
  a function lvalue used to initialize a temporary pointer-reference. Inferring
  this fact from the referenced target type incorrectly preferred rvalue pointer
  references and made function-reference/pointer-reference calls ambiguous.
  Comparison now consumes the recorded source fact. No lint waiver was added:
  the initial fourth boolean was replaced with a semantic enum.
- Ordinary unary function designations resolve before amp/star typing, retaining
  the actual function-to-pointer operand conversion, result type/category and
  direct function identity. They no longer pass through the genuine-overload
  immediate-unary rewrite. Direct calls consume the retained designation;
  parameter function references and indirect pointers retain their own paths.
- Genuine overload selection rewrites only its root and immediate operand as
  C++ does. A deeper unresolved unary node has no resolved expression type;
  the printer now emits that empty type from the retained pending set, while
  its id-expression still has its entity type. Printing performs no resolution.
- ADL deduplication was keyed by type alone despite the inherited depth budget.
  A deep first visit could suppress a later shallower visit to the same type,
  losing its associated namespace. The query now records minimum reached depth
  and revisits only when it strictly decreases, at most 65 visits per type.
  Namespace/function deduplication, inline edges, directive exclusion and
  block/qualified-call suppression are preserved. No class/body is newly demanded.
- A named non-function callee is checked once before its arguments are typed.
  Namespace/namespace-alias values and forbidden local/default callee names
  therefore win over competing bad argument names, matching frozen control
  flow. Function sets retain their separate call checking order. A cohesive
  `publish_named_callee` helper owns original facts and parenthesis aliases;
  the authoritative audit rejected the intermediate 248-code-line function,
  and the factored final source passes without a length/nesting allowance.

One ownership regression verifies unchanged AST expression count, function
lvalue result type/category, original operand decay, source reference-ranking
facts, the selected reference overload and retained direct call identity.
Nineteen minimal required regressions live in PA7's executed `tests/general/`
lane. Their new references were generated only from the frozen oracle after
exact adapter comparison; no existing fixture or reference was modified.

### Validation and preservation

All final commands below have exit 0 in `checks-final.json`. The fixed manifest
still contains exactly 198 identities. The full course lane contains the same
311 identities from the independently enumerated `9f47dc9` tree plus exactly
19 new regressions: 330/330 protected receipts, zero lost identities, exit 0,
no failed comparison counts and no `.test_failed` marker. `course-trace.jsonl`,
`course-result.json`, `course-preservation.json` and `prior-summary.json` retain
actual candidate routes and case-level evidence.

| Check | Final result | Artifact relative to root |
| --- | --- | --- |
| `make build` | locked release workspace PASS | `build-final.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | 198/198 PASS | `stage-required-final.log`, `fixed-manifest.json` |
| `make test-pa7` | 198/198 PASS | `stage-make-final.log` |
| `make course-pa7` | 330/330 PASS, every original/new case retained | `course-final.log`, `course-result.json`, `course-trace.jsonl` |
| `python3 .../clone_check.py prior pa7` | PA1–PA6 fixed and full original course lanes PASS | `prior-final.log`, `prior-summary.json`; `../clone/course/paN/test/` |
| `python3 .../clone_check.py preflight pa7` | oracle/protected/tool routing PASS | `preflight-final.log` |
| `python3 .../clone_check.py preserve pa7` | PASS; independent committed course identity comparison also PASS | `preserve-final.log`, `course-preservation.json` |
| `make lint` / protected `clone_quality.py` | rustfmt, strict all-feature Clippy, debug/release workspace tests PASS | `lint-final.log` |
| `make audit-rust` | 136 files, zero findings | `audit-final.log` |
| Final-binary conversion sweep | 1,940/1,940 exact exit/streams/output matches | `probe-final-binary.log`, `summary.json`, `probes.jsonl` |
| Final-binary function designations/ranking | 232/232 exact matches | `function-final-binary.log`, `function-summary.json`, `function-probes.jsonl` |
| Final-binary type-depth association | 24/24 exact matches, both parameter orders and function/address forms | `adl-final-binary.log`, `adl-depth-summary.json`, `adl-depth-probes.jsonl` |
| Final-binary extended integral promotions/array references | 198/198 exact matches | `promotion-final-binary.log`, `promotion-summary.json`, `promotion-probes.jsonl` |
| Final-binary competing call failures | 20/20 exact matches | `callee-final-binary.log`, `callee-summary.json`, `callee-probes.jsonl` |
| Prior independent source/probe groups | addresses 147/147, queries 86/86, signatures 70/70, initialization 78/78, declarations 112/112, procedural due 411/411 | `rechecks.json` and isolated `pa7-*-20261013/` |
| Protected resource observations | paired bytes equal and two shared larger-stack qualifications PASS | `measure-final.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

The prior procedural suite's `case-float-id-bad` still reaches its individually
recorded outside-PA7 floating-object execution boundary (86), not a matched
rejection. Source review and full PA7 gates distinguish that later dependency.
The first broad exploratory run overlapped the first rebuild; its observations
are retained as discovery evidence only. Acceptance evidence uses the final
fixed binary, source hashes and separately retained final sweeps. No oracle
crash, Rust stub, panic, timeout or failed receipt is counted as a passing case.

### Architecture, numeric rules and resources

One source buffer/syntax unit and one owned Graph live per translation unit.
Dense facts address original ExprId/StmtId/DeclaratorId records; parentheses
share those facts. Canonical TypeIds and indexed names drive comparison and
lookup. Candidate rejection remains compact optional data; only final input
errors render diagnostics. Call scratch resets contents on every return and
retains capacity, with nested argument calls finished before borrowing it.
Rank and application facts are distinct bounded per-candidate data, not copies
of expression subtrees. ADL's per-query `(TypeId, minimum depth)` visitation
state expires with the query; strictly improving depths bound rework without
unrelated scans or global invalidation. Persistent signature caches/demand
remain under their earlier typed owners and complete keys.

Target integral width/sign, promotion and wrap/truncation owners are unchanged;
wide character/fixed/range enum conversions and pointer/array qualification were
compared beyond fixtures. No host signed overflow, float approximation or native
pointer identity was added. Syntax, body and default ownership stay parse-once;
no semantic text transport, speculative class completion, unsafe code, new
dependency, C++ compiler subprocess or implementation environment read exists
in this change. Source/function/nesting checks remain authoritative alongside
rustfmt and strict Clippy. Optimizer, allocator, backend policies and per-function
lifetimes are unchanged. PA35 optimization objectives are not imposed.

Final warm observations use A/A calibration followed by paired ABBA/BAAB blocks,
with every source/binary hash, wall/RSS sample and output digest retained. These
measure complete process startup, parsing, semantics and dump I/O. Equality is
proved separately by protected receipts; timing/RSS equality is not required.

| Workload / calls | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| function-reference-ranking / 128 | 5.20 / 5836 | 17.32 / 8006 |
| shared-adl-depth / 128 | 6.08 / 6558 | 17.21 / 8000 |
| function-reference-ranking / 512 | 6.90 / 6528 | 19.45 / 8746 |
| shared-adl-depth / 512 | 7.61 / 6572 | 23.68 / 8000 |
| function-reference-ranking / 2048 | 10.94 / 7274 | 30.79 / 8368 |
| shared-adl-depth / 2048 | 14.29 / 7280 | 43.88 / 8002 |

The native probes start at 1 MiB and retain 1,500 function dereferences and 1,000
nested ordinary calls. Both adapter receipts have `match: true` and
`stack_qualification.passed: true`, with both tools succeeding with identical
streams/output at the same 16 MiB retry. Raw original attempts and retries are
retained under `stack-qualifications/`. No exhaustion cutoff, frame size or
native diagnostic position was calibrated; the genuine byte-budget guard is
unchanged. Shared storage remains below the 107,374,182,400-byte ceiling.

### Remaining exact audit failures and next dependency boundary

No frozen PA7 case, course identity or normal-result probe in this checkpoint
still fails. The independently revalidated frozen-oracle failures are exactly:

```text
binary-023 binary-051 binary-079 binary-107 binary-135 binary-163
binary-191 binary-219 binary-247 binary-275 binary-303 binary-331
binary-359 binary-387 binary-415 binary-443
```

`oracle-issue-probes.jsonl` preserves protected infrastructure-failure receipts.
`oracle-crash-reproduction.json` revalidates all sixteen sources at both 16 and
64 MiB: 32/32 SIGSEGV exits (-11), no diagnostic and empty output. The pinned
operator-candidate path dereferences an unresolved overloaded function's null
type while processing the enum operand (`analyzer_operators.cpp` and
`conversions.h::ClassOf`), as recorded in the preceding review. No successful
larger-stack outcome exists, so protected stack qualification cannot qualify
these as language results. They are baseline issues under CLONE_CONTRACT.md,
not Rust defects to reproduce or permission to change references/policy.

The next acceptance boundary is disposition of these frozen baseline issues and
final cumulative stage signoff using the retained responsibility census. The
stage-owned normal-result corrections and all due lanes are now verified, but
full exact-clone acceptance is not asserted while those comparisons have no
valid oracle outcome. Ralph's existing goal remains active; it has not been
reset, narrowed, marked complete or advanced. Later class/operator/template/
interpreter work and PA33/PA34 implementation-wide obligations retain their own
stage boundaries.

## Declaration/linkage and scalar failure-order audit checkpoint — 2026-10-13

Reviewed entry: `6ef88b8823f646a16bb17f0725394a355c48b635`.
Stage base remains `9a851dcedb1e56bbc6585c84c5990f6ef1a1c372`.
Reviewed implementation commit: `743155c1cd0d29b13aa35346a67a4ab5f95cd92e`.
`reviewed-source.json` retains all fifteen source hashes and staged audit diff
SHA-256 `cebb2df629dae111d3c028e270f580a391caf6c6be7ad79fcd3e1219b3ca06ec`.
**This closes the findings listed here; full-stage acceptance is still pending.**
The independent operator sweep found sixteen reproducible frozen-oracle crashes,
which are retained as baseline issues, not successful reference outcomes or
accepted rejection fixtures. Ralph's existing goal stays active. No goal,
Ralph state, acceptance policy, frozen source or existing reference was changed.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-declaration-audit-20261013/`.
Final Rust cppgm SHA-256:
`55d8c3e79c5f2d4f5e303f3f403c7e128f350e7d706a18a30e010611f0dd2c6e`.
Frozen cppgm++ SHA-256 remains:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.
`checks.json`, `reviewed-source.json`, `handoff.json`, all probe inputs/receipts,
initial divergence evidence and final resource observations belong to that root.

### Source responsibilities, completeness reasoning and fixes

Read the repository contract/specification and cumulative PA6/PA7 handouts and
ledgers. Consulted the pinned C++ implementation directly, especially
`analyzer_declarations.cpp::declare_declarator`, `declare_variable`,
`check_variable_specifiers`, `earlier_variable`, `new_variable`,
`earlier_function`, `redeclare_function`, `new_function`,
`note_block_linkage`, `agree_block_linkage`, `agree_language_linkage`;
`analyzer.cpp`'s linkage context and hidden-name walk;
`analyzer_builtins.cpp::builtin_function`; `constant_expressions.cpp`'s
function entity places; `analyzer_expressions.cpp::combine`, `arithmetic`,
`unary`, `increment`, `subscript`, `comparison`, `typed_value`,
`check_resolved`; `analyzer_conversions.cpp::conditional`; and
`semantics_dump.cpp/.h`'s source/statement/expression/conversion/call views.

Compared successful control flow as well as competing failures, declaration
order, first-declaration properties, default ownership, canonical numeric/types,
entity identity and presentation bytes. Earlier ownership/demand audits remain
in this ledger. No fixture count substitutes for this review.

- `linkage.rs` owns storage/language linkage obligations, using typed `Language`
  and `Specification` values, compact `ScopeId`/`NameId`/`EntityId` facts, and
  a unit-local namespace/name index. A single declaration in a linkage
  specification is effectively extern; declarations in braces retain written
  storage. Nested language/specification context restores after analysis,
  including errors. Written-extern reference-initializer rules retain their
  separate ordering.
- A block extern variable adopts an eligible earlier namespace variable,
  including an external variable hidden by an intervening local. A hidden
  internal one remains distinct. An adopted variable shares its constant,
  canonical entity and completed array bound, while each binding retains its
  source scope. Const array element qualification and reference exceptions
  follow C++ first-declaration internal-linkage rules.
- A block function keeps its own entity and defaults, records its canonical
  namespace denotation, and inherits eligible outer storage/language facts.
  Its conflicting return type fails at the declaration. The frozen constant
  interpreter compares function entity places without normalizing denotation;
  that distinction is preserved rather than rewriting constant pointer identity.
- An unmatched block function inside an unnamed namespace still has external
  linkage; internal namespace linkage belongs to the namespace entity itself.
  The ownership test separately checks the internal enclosing function and
  external unmatched declaration. This corrects a retained fact even though
  the semantic view does not print it; acceptance is not based on dump tests
  alone.
- A first namespace declaration validates prior unmatched block obligations,
  newest first, for the same namespace/name: kind, matching function signature
  and return type or compatible variable type, then internal and language
  linkage. Other namespaces and unrelated overload signatures are skipped.
  No retrospective entity unification, whole-unit search or negative cache is
  introduced.
- Existing variables check compatible type before static/linkage agreement,
  then redefinition. Existing functions preserve return/delete/redefinition/
  static/language/constexpr order. Qualified redeclarations skip language
  agreement as C++ does. Namespace signatures introduced by a using declaration
  diagnose a conflict; different signatures remain available overloads.
- Initialized or qualified typedef declarators fail before the constexpr-typedef
  rule. Block extern initializers and constexpr declarations without a definition
  retain the required errors and positions. Cohesive typedef, signature matching
  and linked-variable completion helpers preserve source responsibility limits.
- Reserved abort is created once as a hidden global typed entity before its
  first explicit ordinary global prototype or value use. Explicit declarations
  retain its builtin origin and implicit nonthrowing exception facts. Wrong
  return types, overload arity, ordinary shadows and declared builtin constant
  execution failures follow their respective C++ paths; builtin language and
  exception matching exemptions apply to the entity, not a spelling test in
  general declaration comparison.
- Scalar logical expressions convert the resolved left operand before resolving
  the right value. Arithmetic rejects void before pointer checks. Conditional
  then operands resolve as values before else typing. Subscripts resolve the
  base before typing the index; a scalar braced index fails at that index before
  visiting its list names. Successful result types/categories and recorded
  conversions are retained. Class operator resolution remains a distinct later
  owner, not an ordinary rejection stub.
- The dump review confirms source declaration order, procedural statement child
  order, canonical parameter versus declared result types, original default
  expression IDs, recorded conversions, folded values, omitted sizeof operands,
  commuted subscript presentation and iterative source-then-demand traversal.
  Printing borrows facts and performs no semantic lookup, typing, substitution
  or body demand. The previously unmapped dump header now has an explicit Rust
  interface owner in the source map.

### Architecture and Rust review

The new obligations are owned by one `Analyzer`, with complete typed
`(ScopeId, NameId)` keys and append-only source order; the graph owns persistent
first-declaration facts. There is no cache invalidation or unrelated name scan.
A private ordinal cursor walks indexed overload entries without copying the
candidate set, respecting declaration-point lookup limits. Canonical type
interning stays in the type owner. Compact entity IDs and original binding
scopes separate shared storage from source presentation; function defaults are
not inherited by copying a namespace function.

The ownership regression verifies shared block-variable storage/completed type,
separate source binding scopes, independent block-function defaults and
canonical denotation, restoration of C++ language context, retained builtin
exception facts, and unchanged AST expression count. The parsed unit is never
cloned, synthesized or reparsed. A braced class subscript remains explicit
unported operator demand instead of being disguised as a normal rejection.

Safe Rust, structured `Result` errors, host-size/ID checks, original typed
byte-budget guards and deliberate target arithmetic are preserved. No new
unsafe/dependency/blanket allowance/environment read or C++ subprocess path
was added. The new module owns a cohesive linkage algorithm, not a mechanical
split. Strict debug/release Clippy, rustfmt and the authoritative file audit
pass: 136 implementation files, zero findings; 187 library tests pass in each
profile, plus the existing binary tests. The optimizer, register allocator,
lowering/native policies and per-function backend ownership were not changed.
Oracle tools remain harness comparators/evaluators only; due frontend work runs
in Rust.

### Exact probes, retained cases and validation

The first declaration sweep matched 50/67; the seventeen mismatches and their
raw evidence are retained in `probe-initial.log` and `probes-initial.jsonl`.
The expanded sweep reached 109/112 and then **112/112** after qualified language
agreement and builtin value lookup were corrected (`probe-final-owned.log`).
The boundary sweep then exposed builtin exception redeclaration matching;
after that fix it is **142/143**, with only `builtin-noexcept-query` reaching
an explicit later semantic `noexcept` expression boundary (PA16; status 86).
This is recorded in `boundary-final.log`; it is neither a PA7 accepted negative
case nor a claim that general exception expression analysis is complete.

Rechecked prior independent groups before the final internal-linkage fact
correction (then verified that correction with the ownership test and all final
gates):
query 86/86, address 147/147, procedural due 411/411, function-template
70/70, initialization 78/78 (`rechecks.json` and the isolated `recheck-*`
receipts). The procedural suite's already recorded outside-PA7
`case-float-id-bad` remains its single floating-object boundary; it is not
counted as a passing rejection or silently omitted from the evidence.

Eight additional scalar failure-order divergences are resolved; thirteen
focused operator probes now match (`operator-probes-final.jsonl`). The wider
operator matrix is **545/561**, with all sixteen residuals caused by the frozen
oracle SIGSEGV described below, not differing normal language results.

Added **43** supplemental executed regressions, listed exactly in
`fixtures.json`: **28 in PA6** for the earlier declaration/storage/linkage/
exception responsibilities, **15 in PA7** for constant entity identity, builtin
value/constant use and scalar analysis failures. New references alone were
produced by the frozen oracle in the original course cwd. Existing fixtures,
references and protected fixed manifests are unchanged. Each course invocation
has a strict adapter match, including diagnostic/artifact comparison for errors.

| Command | Final result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS | `build-operator-final.log`, `build-final.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | PASS | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | PASS, 198/198 | `stage-required.log`, `fixed-manifest.json` |
| `make test-pa7` | PASS, 198/198 | `stage-make.log` |
| `make course-pa7` | PASS, 311/311 | `course-final.log`, `course-result.json`, `course-counts.txt` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | PASS, all PA1–6 fixed and full course lanes | `prior-final.log`, `prior-summary.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | PASS | `preserve.log` |
| `make lint` | PASS, rustfmt/strict Clippy/debug and release tests/file audit | `lint-final.log` |
| `make audit-rust` | PASS | `audit-final.log` |

Earlier fixed counts remain PA1 58, PA2 26, PA3 20, PA4 106, PA5 196, PA6 121.
Full courses remain 58/58, 28/28, 20/20, 112/112, 296/296, and now 722/722.
`preservation.json` retains all 198 preceding PA7 fixed IDs. The source-entry
course identity comparison in `course-preservation.json` retains all preceding
694 PA6 and 296 PA7 cases, including multi-translation-unit inputs; no prior
passing case is lost. Exact command times are recorded in `checks.json`.

An intermediate PA6 course snapshot copied the final new test before its new
reference/status pair existed. Its missing-reference failure is retained in
`pre-exception-validation/pa6-incomplete-fixture-snapshot.log`. It is superseded
by the complete final rerun above; no failed outcome is counted as acceptance.
Quality/probe reruns followed the builtin and scalar corrections.

### Resource observations, separate from dump equality

Measurements were made after validation processes ended. At a shared 16 MiB
stack, two warm samples per tool precede interleaved ABBA/BAAB samples, four
measured per tool and workload. Every sample succeeds with one exact dump hash;
12 independent protected receipts also match. `measurements.json` records
source/binary hashes, raw samples, bytes and RSS; `measurement-results.log`
records results. Times are observations, not an optimization/equality policy.

| Workload / functions | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| Indexed outer linkage / 128 | 6.51 / 6472 | 19.79 / 7996 |
| Indexed outer linkage / 512 | 11.29 / 7908 | 29.20 / 8728 |
| Indexed outer linkage / 2048 | 25.09 / 12822 | 61.05 / 11830 |
| New block obligations / 128 | 8.56 / 6474 | 22.14 / 8376 |
| New block obligations / 512 | 16.35 / 8570 | 40.49 / 8744 |
| New block obligations / 2048 | 36.71 / 16010 | 89.13 / 18510 |

The first workload repeatedly adopts namespace function/array facts; the second
creates distinct block obligations followed by their namespace declarations.
The direct indexes visit the relevant name set only. Source text/AST/name
storage remains unit-owned; no per-function backend state was extended.

At a 1 MiB initial stack, the 1000 nested linkage specifications and 1500 address
parentheses probes both qualify through the protected adapter: **both tools
succeed at 16 MiB with exact streams/files**, `match=true` and
`stack_qualification.passed=true`. `native-probes.jsonl` and its linked
`stack-qualifications/` retain initial and successful larger-stack attempts.
No guard depth, native cutoff or diagnostic position was reproduced/calibrated.

### Baseline issues and next audit boundary

The operator matrix residual IDs are exactly:

```text
binary-023 binary-051 binary-079 binary-107 binary-135 binary-163
binary-191 binary-219 binary-247 binary-275 binary-303 binary-331
binary-359 binary-387 binary-415 binary-443
```

Each has an unresolved overloaded function `(f)` as the left operand and an
unscoped enum parameter `(u)` as the right, for the listed arithmetic, bitwise,
logical or comparison operator. `operator-matrix-summary.json` preserves exact
sources, exit values, diagnostics and artifact evidence. Protected receipts
classify the oracle as infrastructure failure (exit -11), not normal rejection.
Independent direct runs of all sixteen at **both 16 and 64 MiB** reproduce
SIGSEGV, no diagnostic and an empty output file (32/32 attempts), recorded in
`oracle-crash-reproduction.json`. In the pinned source,
`analyzer_operators.cpp::operator_candidates` calls `ClassOf` on the left
unresolved overload's null type; `conversions.h::ClassOf` dereferences it.
This is a reproducible baseline bug, not stack exhaustion. There is no exact
successful larger-stack oracle result and no legitimate Rust crash to emulate.
References, oracle and protected policy are preserved.

No due corrected probe still fails with a normal C++ result. The standalone
`builtin-noexcept-query` remains an individually recorded PA16 dependency;
class/enum operator-function candidate resolution is a PA12+ owner, distinct
from PA7 builtin operators. This checkpoint does not certify those later paths.
Full PA7 acceptance still requires reconciling the baseline issues and finishing
its cumulative responsibility census; green fixture lanes alone do not close
Ralph's goal. Continue from this audited source revision and retained evidence,
not by resetting the goal or editing Ralph state.

## Type-query and discarded-value audit checkpoint — 2026-10-13

Reviewed clean entry/group base:
`223cb4a5c3b422323dcc8f672189f4be701c8065`.
The previous turn made authoritative progress in implementation commit
`6741933` and ledger commit `223cb4a`; their clean worktree, ledger and retained
handoff were inspected before continuing the existing phase goal.
Stage base remains `9a851dcedb1e56bbc6585c84c5990f6ef1a1c372`.
Reviewed resulting implementation commit:
`ee02b39f602ab94d4d286e97e1d39649d6b5643e`.
The staged code diff SHA-256 is
`2d41d207011b1c16ce937c8747945cd30f0497c6e9be5f0fc90784eb9da1a212`;
`reviewed-source.json` retains all eight source hashes.
**The nine query-group divergences below are resolved. Full-stage acceptance
remains open for the cumulative declaration/operator/dump source census.**
No executed probe in this group fails. Ralph state, goal and acceptance policy
were neither reset nor edited.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-query-audit-20261013/`.
Final Rust cppgm SHA-256:
`31a22019dc5241025ec182b0ff02de6f0da4f41859a5307446ba423da8aaba81`.
Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.
Commands/costs are in `checks.json`; raw source probes, initial mismatches,
final comparisons, quality results and measurements are retained under that
root. `handoff.json` records the reviewed source commit and still-open goal.

### Reviewed responsibilities and corrected behavior

Consulted frozen `analyzer_expressions.cpp::size_of`, `empty_functional_cast`,
`resolve_lone_function`, `typed_value` and `check_resolved`; `layout.cpp::type_size`;
`analyzer_builtins.cpp::constant_p_call`; `constant_expressions.cpp::discard` and
its comma/rvalue/glvalue paths; and `constant_statements.cpp`'s builtin-call
failure. Compared their Rust owners and current source-map responsibilities.

- `queries.rs::semantic_size` owns semantic sizeof analysis over the original AST.
  A function operand or unresolved overload is rejected at the query location
  with `sizeof of a function`, rather than an operand-target or generic incomplete
  type diagnostic. A lone ordinary function is selected through the existing
  designation API; deleted-function failures retain their order. Written type-id
  queries keep their distinct incomplete/function-type path. A parenthesized bare
  name that now denotes a type uses that binding without a reparse. An empty
  functional cast/type-name call directly under parenthesized sizeof follows the
  frozen function-type reading, while extra parentheses and braced value
  initialization remain expressions. This fixes the previously successful
  `sizeof(I())` alias case. Successful queries retain size_t `TypeId` and
  `query_value`, with no copied operand tree or printer-time type calculation.
- Reviewed scalar/enum/reference/pointer/member-pointer sizes, alignments, unknown
  bounds and incomplete types against `layout.cpp`. Checked array multiplication
  is deliberate target overflow checking, and element failures precede outer
  overflow. The large scalar/nested/incomplete-array probes match; no wrapping
  size or host-size rejection was substituted for the frozen rule. Unevaluated
  null indirection, increments and direct/indirect call operands remain typed
  without interpreting their values.
- Semantic comma evaluation in `constants.rs` now discards its left operand
  rather than reading it as an integer. Integral binary spines stop before a
  semantic comma and consume its recorded result normally; PA6's name/integral
  evaluator keeps its existing path. `constant_addresses.rs::discard_constant`
  walks original IDs iteratively: comma effects stay left-to-right, void casts
  discard their operand, void conditionals visit only the selected arm, calls
  retain their normal constant-evaluation boundary, and source glvalues designate
  a place without reading variable storage. Parentheses use existing flattened
  facts. Nested operations reuse `constant_discard_work` with an explicit saved
  base, and both success/failure restore it while retaining capacity.
- Synthesized abort entities retain a typed `BuiltinFunction::Abort` origin fact.
  Required constant evaluation emits the frozen `a builtin function that is not
  a constant expression` failure. It does not infer builtin origin from a source
  spelling or turn a missing constexpr body into a generic zero. Ordinary user
  functions still have their separate non-constexpr failure. The builtin remains
  recognized semantically without executing abort or requiring later lowering.

Exact initial failing IDs were `sizeof-function`, `sizeof-function-parens`,
`sizeof-overload`, `sizeof-overload-address`, `sizeof-empty-alias`,
`sizeof-function-reference`, `query-void-discard`, `constant-void-discard` and
`constant-abort-discard`. All are resolved. The expanded 86-case set includes
unread ordinary/volatile glvalues, nested commas under arithmetic, void casts and
selected/unselected void calls, mutable scalar temporary effects, query results,
function pointers/references/deleted functions, template-id address queries,
fundamental/reference/array sizes, alignment and overflow/failure ordering.
Initial failed receipts and output files are retained in `probes-initial.jsonl`,
`probe-initial.log` and `mismatches/`; stale build-attempt logs are not final proof.

The new ownership test checks that the source expression count stays unchanged,
unread storage remains nonconstant, query results are published correctly,
builtin origin travels with the selected entity, and repeated failing evaluation
restores the same discard/evaluation scratch state. Strict Clippy required the
cohesive recorded-constant fact path to be extracted from the dispatcher; no
length waiver, blanket allowance or mechanical file split was added. No new
module, dependency, unsafe code, FFI, text transport, environment read, source
recognition or C++ production fallback was introduced. Parser ownership, compact
IDs, explicit demand, complete query ownership and backend/optimizer/allocator
policies remain intact. Seven new regressions and their new oracle-derived
references are in the earliest executed PA7 lane; existing inputs and references
were not formatted or changed.

### Validation and preservation

| Command/check | Final result | Artifact relative to root |
| --- | --- | --- |
| `make build` | Pass, locked release workspace | `build-final.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | 198/198 fixed cases | `stage-required.log`, `fixed-manifest.json` |
| `make test-pa7` | Pass, same fixed coverage | `stage-make.log` |
| `make course-pa7` | 296/296, no failure marker | `course-final.log`, `course-result.json`, `course-counts.txt` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | All PA1–PA6 fixed/full-course lanes pass | `prior-final.log`, `prior-summary.json` |
| Protected preflight/preservation | Pass | `preflight.log`, `preserve.log`, `preservation.json` |
| `make lint` / protected quality checker | rustfmt, strict Clippy, 186 library tests in debug/release plus tools/docs, file audit pass | `lint-final.log` |
| `make audit-rust` | 135 files, zero findings | `audit-final.log` |
| `python3 .../pa7-query-audit-20261013/probe.py` | 86/86 | `probe-expanded.log`, `summary.json`, `probes.jsonl` |
| Re-executed preceding probes | 147/147, 62/62, 70/70, 78/78, 55/55 and 411/411 PA7-scope procedural probes | `addresses/`, `preceding/`, `templates/`, `initialization/`, `scalar-audit/`, `procedural/` |
| New fixture comparisons | 7/7 | `fixture-probes.jsonl`; final full-course trace |
| Resource/native probes | 12 exact scaling receipts, two protected native qualifications | `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl`, `measure.log` |

Independent preservation is 198 entry/current fixed passes with zero lost.
Full-course identity preservation is 289 entry cases retained, seven new cases
and all 296 exact receipts, recorded in `course-preservation.json`. The original
229-case lane remains included. PA1–PA6 fixed counts remain
58/26/20/106/196/121, full-course counts 58/28/20/112/296/694.
The preceding procedural probe's explicitly outside-PA7 floating-object mismatch
remains recorded separately; it is not an accepted negative or a gate exception.

### Resources and next boundary

All gate/quality/probe jobs were terminal before measurement. Same supported
inputs/output paths and 16 MiB stacks were used, warm A/A then ABBA/BAAB sampling
with four measured runs per tool and two protected receipts per input. Counts
mean discarded void-cast comma operands or sizeof queries. Source/binary/output
hashes and individual timing/RSS samples remain in `measurements.json`.

| Workload | Count | C++ median ms / peak KiB | Rust median ms / peak KiB |
| --- | ---: | ---: | ---: |
| Discarded comma | 128 | 7.34 / 6,492 | 20.99 / 7,998 |
| Sizeof queries | 128 | 8.15 / 5,820 | 21.80 / 8,744 |
| Discarded comma | 512 | 9.24 / 6,512 | 27.34 / 8,368 |
| Sizeof queries | 512 | 10.95 / 6,466 | 25.79 / 8,376 |
| Discarded comma | 2,048 | 25.04 / 7,154 | 64.00 / 8,374 |
| Sizeof queries | 2,048 | 20.87 / 7,200 | 52.99 / 8,366 |

These are process-level observations, separate from semantic-byte equality and
from any generated-code optimization objective. Scratch values are reclaimed at
query boundaries, capacities reused, and the discard traversal is iterative.
No PA35 target or PA34 object/bootstrap proof is inferred.

The inherited 1,000-deep pointer conditional and 1,500-parenthesis address probes
were executed from a 1 MiB stack. Both receipts report `match=true` and
`stack_qualification.passed=true`, with both tools succeeding identically at
16 MiB. Raw original exhaustion attempts remain retained. No cutoff or diagnostic
position was calibrated; the typed byte-budget guard remains in production.

No executed finding from this group remains open. The next dependency boundary
is still the cumulative ordinary declaration/redeclaration, scalar operator and
semantic dump census, reconciling these query/effect corrections with earlier
partial ownership rows. Full class/template/constexpr-frame responsibilities stay
with the documented later contracts; status 86 is never a successful rejection.
Full-stage acceptance and Ralph's existing goal remain open until that complete
source review, rather than fixture totals, justifies acceptance.

## Procedural acceptance-audit checkpoint — 2026-10-13

Reviewed clean entry and group base:
`a4f09d05042874cc9e4e1224f2a206df61de5dc7`.
Stage base remains `9a851dcedb1e56bbc6585c84c5990f6ef1a1c372`.
Reviewed resulting implementation commit:
`67419338047bc8419b75b35d334efecc2c6eec29`.
The final staged code diff (including the new module) has SHA-256
`5305feee23d422377303cdad90c53c51538d6c59bda402724e47d5dc77215546`;
`reviewed-source.json` retains each source hash.
This continued the existing full-stage audit, without creating/resetting its goal,
editing Ralph state, changing acceptance policy or advancing the stage.
**The procedural/control/default/scalar-effect findings below are fixed;
full-stage acceptance remains open for the remaining cumulative source census.**
There are no failing executed PA7-scope probe or fixture identities in this group.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-procedural-audit-20261013/`.
Final Rust cppgm SHA-256:
`6a9afadc8a420b86448786dc3d44b1f78c04e006df921eae404634d3d63ba846`.
Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.
`checks.json`, the source/diff snapshots and `handoff.json` identify the final
commands, reviewed implementation commit and explicit still-open goal status.

### Source responsibilities, findings and completeness reasoning

Read the current repository instructions, clone contract, specification, PA7
handout, plan, source map and ledger, then compared the frozen owning functions
with their Rust algorithms. `responsibility-review.json` records 55 functions
in nine C++ owners, source locations/hashes, Rust owners, reasoning and remaining
boundaries. It is a review of these groups, not an implementation-wide completion
census. The source map records each corrected responsibility and supersedes the
previous checkpoint's unit-level cast-temporary memo description explicitly.

- `analyzer_bodies.cpp`'s procedural scope, loop/switch, condition, case and
  return owners map to `statements.rs`/`procedural_statements.rs`. Checked guarded
  substatements and header/body lifetimes, nested switch isolation, automatic
  initialization counts, static/extern/thread-local exclusions, duplicate-label
  precedence and sorting only after the whole switch body. A case first resolves
  its operand and evaluates its constant, then records operand conversion and
  checks representability; a pointer-valued label previously produced the wrong
  conversion diagnostic. An unresolved overload in a void return or case is
  rejected before the result/constant diagnostic. Declared conditions now retain
  noninitializing read/promotion/boolean conversion facts against their original
  `DeclaratorId`; source object types remain distinct and scoped switch targets
  strip top-level cv. Dumping requires those recorded facts without retyping.
- `analyzer_initialization.cpp::list_argument` and the non-template default
  owners map to `default_arguments.rs`/`initialization.rs`. A braced reference
  default initializes a prvalue of the referent's object type, rather than a
  reference-typed list or direct binding to its source element. Scalar cv removal,
  array qualifiers, nonconst/volatile lvalue-reference rejection and narrowing
  follow the frozen owner. Original default/list/element IDs and declaration
  visibility are retained; printing does no default checking.
- `analyzer_calls.cpp`, `conversions.cpp` and their Rust call/conversion owners
  were compared for indexed lookup, allowed ADL edges, source/target identity,
  standard/reference ranks, arity/default filtering, expected candidate rejection,
  candidate-local selected function facts, winner-versus-all ambiguity checks and
  publication only after selection. Singleton ordinary function designations are
  valid ellipsis operands; true overload sets still require a target. Callee
  parentheses now share the original designation facts and suppress ADL. Missing
  qualified or parenthesized callees fail before argument typing; ordinary
  unqualified calls preserve the argument analysis ADL needs. A 169-case ordinary
  scalar/reference ranking matrix matched without new ranking policy.
- `analyzer_expressions.cpp::assignment` maps to `scalar_expressions.rs` and
  `scalar_operators.rs`. LHS overload resolution precedes ordinary and braced RHS
  typing. Scalar braced assignment uses the unqualified left object type and
  keeps its original list/element facts. Compound assignments retain operation
  and count `TypeId`s independently from the source/result lvalue type, so later
  consumers do not reclassify arithmetic.
- `constant_expressions.cpp`'s assignment, compound assignment, increment and
  glvalue/rvalue rules map to the cohesive `constant_mutations.rs` and existing
  value/place/arithmetic owners. Simple assignment evaluates/converts the RHS
  before locating the LHS; compound assignment designates/reads it once. Recorded
  operation types drive target wrapping/truncation, shifts and signed overflow.
  Pointer moves use the existing symbolic bounds algorithm. Undefined arithmetic,
  bad pointer moves and read failures precede writes at the C++ locations.
  Pointer, subscript, conditional, comma, reference and member places no longer
  reach the generic missing-assignment path. External variable, string-literal
  and extended-temporary writes retain their distinct normal failures.
- `constant_interpreter.cpp::root_storage/read/write` maps to private typed
  scalar temporary storage. Cast temporaries created inside an evaluation may
  change, including assignment, compound assignment and prefix/postfix results.
  `Evaluation` owns sparse original-`ExprId` integer/pointer values; nested
  operations share that owner. Both success and structured failure clear all
  values and return the empty scratch container for capacity reuse. The previous
  unit-owned expression-temporary memo was removed, so no mutable value, duplicate
  initial snapshot or negative result survives the query. Only declaration-owned
  immutable constants/references and their extended temporary storage remain in
  `Graph`. Full constexpr bodies/frames are still an explicit later owner and
  cannot enter this expression-lifetime store.
- A temporary bound before the evaluation is readable only through a const view.
  This check occurs before arithmetic or pointer stepping. Pointer reference
  casts now designate their resulting place/view and read through it, rather
  than bypassing the view as a pointer-value cast. That closes both an incorrect
  successful constexpr pointer read and an incorrect constant-query result.
  Genuine query-local mutable temporaries remain readable. Known scalar unary
  prvalues fail normal glvalue designation instead of reaching a missing stub.
- Integral-required address operands preserve address evaluation failures before
  the nonintegral-result diagnostic. `__builtin_constant_p` selects its integer
  or address evaluator once from recorded type facts; it does not execute a
  pointer expression twice. Missing later execution is still status 86 and never
  silently becomes query zero.

Initial concrete divergences included `switch-case-pointer-bad`,
`return-void-overload-bad`, `default-list-ref`, `case-overload-bad`,
`ellipsis-function`, `ellipsis-function-address`, the indirect/subscript/
conditional/comma/member assignment probes, `assignment-constexpr-bad`,
`assignment-pointer-constexpr-bad`, the four `mutable-cast-lvalue*` boundary
probes, `undeclared-qualified-argument-order`, `undeclared-parenthesized`, and
`extended-pointer-temporary-nonconst-read-bad`/`...-query`. They are resolved.
Raw failed attempts and mismatch outputs remain in the artifact root; the
xvalue-mutation boundary probes reject normally in both implementations and were
not substituted for the successful lvalue-temporary cases.

`case-float-id-bad` remains a separately classified later-boundary probe:
both tools reject it, with different diagnostics because complete floating
constant-object storage is not yet implemented. PA7's handout explicitly excludes
floating-point literals. Its raw mismatch remains recorded; it is not a matched
negative case, a gate exception or a reason to certify the remaining source
review. Full floating/class/array interpreter state remains due to later owners.

Three meaningful ownership tests prove that semantic analysis preserves the
original expression count, compound operation/count types stay separate from the
left object, declared conditions retain read/target facts, and repeated successful
and failing evaluations of the same original mutation expression start fresh and
release their active values. They supplement byte comparisons. Twenty-six new
regressions and new oracle-derived references are in PA7's executed general lane;
existing compiler inputs, references, protected manifests and checker code were
not modified.

No unsafe code, FFI, source/fixture recognition, C++ implementation fallback,
text transport/reparse, new dependency, global retry, mechanical file split or
lint allowance was added. The new module owns scalar effects, not a chunk of a
large dispatcher. Compact IDs, canonical types, parse-once syntax and private
fact/storage APIs remain the boundaries. Function-local control contexts and
resolution scratch die/reset at their owning body/query; backend lifetimes,
optimizer and allocator policies were not changed. Oracle tools were used only
by the protected comparator, reference generation and resource comparison.

### Exact validation and preservation

| Command / check | Final result | Artifact relative to root |
| --- | --- | --- |
| `make build` | Pass, locked release workspace | `build-final.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | Pass, 198/198 fixed cases | `stage-required.log`, `fixed-manifest.json` |
| `make test-pa7` | Pass, same fixed coverage | `stage-make.log` |
| `make course-pa7` | Pass, 289/289, no failure marker | `course-final.log`, `course-result.json`, `course-counts.txt` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | Pass, all PA1–PA6 fixed and full course lanes | `prior-final.log`, `prior-summary.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | Pass | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | Pass | `preserve.log`, `preservation.json` |
| `make lint` / protected `clone_quality.py` | Pass, rustfmt, strict Clippy, 185 library tests in debug/release plus tool/doc tests, file audit | `lint-final-accepted.log` |
| `make audit-rust` | Pass, 135 files, zero findings | `audit-final.log` |
| `python3 .../pa7-procedural-audit-20261013/probe.py` | 411/411 PA7-scope comparisons; one recorded out-of-scope floating mismatch | `probe-final.log`, `probes.jsonl`, `summary.json` |
| Preceding address/full-stage/template/initialization/scalar probes | 147/147, 62/62, 70/70, 78/78, 55/55 | `addresses/`, `preceding/`, `templates/`, `initialization/`, `scalar-audit/` |
| New fixture comparisons | All 26 match in final full-course trace | `fixture-final.jsonl`, `course-preservation.json`; initial failed attempts retained separately |
| `python3 .../pa7-procedural-audit-20261013/measure.py` | 18 exact workload receipts and two successful protected native qualifications | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

Fixed identity preservation is independently 198 entry/current passes, zero
lost and no fixed-manifest coverage change. Full-course preservation compares the
entry commit's 263 `.t` identities with the final trace: every entry ran and
matched, 26 new cases ran, and all 289 receipts match. The original 229-case
course lane and all preceding added cases are retained. Earlier fixed counts are
58/26/20/106/196/121; earlier full-course counts are 58/28/20/112/296/694.
Final commands were rerun after the code changes; intermediate gate receipts and
failed quality attempts are retained separately. No failed attempt is presented
as an accepted negative.

### Resource observations and protected stack qualification

Measured only after all gate/probe/quality jobs had finished. Each supported
input used the same 16 MiB stack and output path for both binaries, warm A/A runs
followed by ABBA/BAAB samples, with four measured runs per tool and two protected
adapter receipts. Source/binary hashes, individual samples and exact semantic
output hashes are retained. Counts mean switch labels, separately initialized
mutable scalar temporaries, or reference-default declarations plus calls.

| Workload | Count | C++ median ms / peak KiB | Rust median ms / peak KiB |
| --- | ---: | ---: | ---: |
| Switch labels | 128 | 6.91 / 5,766 | 19.79 / 8,732 |
| Mutable temporaries | 128 | 7.56 / 6,506 | 20.44 / 8,738 |
| Reference defaults | 128 | 7.08 / 6,478 | 19.50 / 8,742 |
| Switch labels | 512 | 7.62 / 6,526 | 23.63 / 8,374 |
| Mutable temporaries | 512 | 12.36 / 7,236 | 32.15 / 8,370 |
| Reference defaults | 512 | 12.77 / 7,204 | 32.08 / 7,990 |
| Switch labels | 2,048 | 10.18 / 6,520 | 29.21 / 8,746 |
| Mutable temporaries | 2,048 | 22.66 / 9,280 | 58.90 / 9,952 |
| Reference defaults | 2,048 | 45.89 / 10,684 | 105.64 / 9,954 |

These are complete-process compile observations, not generated-code optimization
or universal complexity claims. RSS noise at small counts is retained. Scalar
query storage reuses capacity without retaining values; switch duplicate checking
is one final sort and defaults reuse their original committed syntax/facts.
Semantic byte equality is proved separately by the protected receipts. No LowIR,
object/bootstrap or PA35 optimization acceptance is inferred from these timings.

`native-pointer-conditional.cpp` contains 1,000 nested pointer conditions;
`native-address-parentheses.cpp` contains 1,500 parentheses around an address
comparison. Both protected receipts report `match=true` and
`stack_qualification.passed=true`: from the original 1 MiB stack, both tools
succeed with identical outputs at the same 16 MiB stack. The original exhaustion
attempts are retained. No native cutoff or diagnostic location was calibrated.
The production guard remains a real typed byte-budget check. The direct-entry
ownership test establishes the same stack anchor as `Analyzer::analyze`; it does
not bypass or loosen that guard.

### Remaining acceptance work

This is a coherent audited increment, not full-stage certification. Finish the
cumulative ordinary declaration/redeclaration, scalar operator/type-query and
semantic rendering source census, reconciling these corrections with all earlier
ownership rows and reviewed checkpoints. The next concrete owners are frozen
`analyzer_declarations.cpp`, `analyzer_types.cpp`, `analyzer_expressions.cpp`,
`analyzer_operators.cpp` and `semantics_dump.cpp`, with their existing Rust
owners. Preserve the explicit PA11–PA23 class/template/full-interpreter boundaries
in the plan; do not treat an outside-scope successful C++ probe as a matched Rust
rejection. PA33's release/debug/complete-implementation census and PA34's full
object/tool fixed point remain later obligations. Ralph's existing phase goal
remains active until full-stage audit acceptance is actually justified.

## Symbolic address acceptance-audit checkpoint — 2026-10-13

The preceding turn made authoritative progress in `47bb96a`; this turn inspected
its clean worktree, current ledger, source map and retained receipts before
continuing the same full-stage audit. **The six preceding address findings are
resolved. Full-stage acceptance remains open pending the remaining complete
stage-owned source-responsibility review.** Passing this checkpoint's fixtures
and probes is not a substitute for that review or for later implementation-wide
PA33 ownership/release checks. Ralph's existing goal/state and acceptance policy
were not changed or replaced.

Reviewed entry/group base:
`47bb96aef40fbfb74c1c064747685eeb779f989d`.
Stage base remains `9a851dcedb1e56bbc6585c84c5990f6ef1a1c372`.
Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-address-audit-20261013/`.
`checks.json` records final command exits and costs; `handoff.json` records the
committed checkpoint and explicit still-open audit status. Final Rust cppgm
SHA-256: `2f1998fc86dac9aeb980a4612efad1ae446078bc2d20e537d544e4a67dee1d94`.
Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

### Reviewed C++ responsibilities and Rust ownership

Read the owning handout and current contracts/plan/ledger; consulted frozen
`sema/constant_values.h`, `constant_interpreter.cpp`'s literal identity,
read/storage, array bounds, advance, publication and reference-temporary rules,
`constant_expressions.cpp`'s rvalue/glvalue, entity/member/reference-cast places,
scalar conversions, pointer arithmetic/comparison and conditional control flow,
`constant_statements.cpp`'s indirect-call/evaluable paths,
`constant_evaluator.cpp`'s initializer/variable-value entries,
`analyzer_builtins.cpp::constant_p_call`, `entity_records.h::VariableEntity`,
`analyzer_expressions.cpp`'s singleton function and explicit/reference casts,
`analyzer_calls.cpp`'s sequences/ADL, and `semantics_dump.cpp`'s initializing
versus explicit conversion views. Source-map rows record partial ownership and
future interpreter/class/template boundaries explicitly.

- `constant_addresses.rs` owns symbolic null, variable, function, literal,
  extended scalar temporary and query-local scalar temporary roots. Values
  retain canonical designated `TypeId`, deliberate signed target byte offsets
  and one-past flags. Private typed stores distinguish evaluating, unavailable
  and successful values. Original committed `EntityId`/`ExprId` keys replace host
  pointers; generic syntax-ID hashing follows ordinal equality without payload
  trait bounds. Hash iteration never controls output order.
- Places and reads remain different operations. Taking an object's address or
  walking its array consumes its type/layout without reading variable storage.
  Scalar/reference reads preserve volatile, uninitialized, nonconstant,
  null-indirection and one-past diagnostics. Literal reads consume decoded bytes
  through checked host offsets. Array/subarray and class/anonymous-member bounds
  preserve source order, C++'s 256-level walk and first matching subobject rules.
  Pointer moves/differences use explicit target wrapping and scalar/object bounds,
  not host addresses or fabricated zero adjustments.
- `constant_pointer_operations.rs` consumes recorded expression facts for
  null/object/function/literal equality, order, difference, truth, short-circuit
  logic and pointer/integer casts. Literal identity uses type/kind and decoded
  code units, preserving the frozen literal-merging choice. Reference casts use
  their original selected conversions, including scalar temporary binding;
  unsupported inherited layouts, member-pointer adjustments and general object
  interpretation retain explicit dependencies.
- Pointer/null constants and references are checked at their declaration.
  Publishing an address requires static non-thread storage; ordinary evaluation
  can use local addresses to form an integer result. First-declaration typed
  `ThreadStorage` is retained beside automatic/extern facts. Direct scalar
  temporaries may persist with their reference; a glvalue reference cast's
  query-local temporary cannot be published by pretending it has static lifetime.
  Successful immutable cast-temporary values are memoized by their original
  bound expression identity. Frame-dependent/instantiated body evaluation cannot
  enter this memo; it remains an explicit missing owner.
- Integral reference initialization binds a place instead of prematurely reading
  its referent. Later reads consume the recorded place and view. Implicit
  pointer/function-to-bool constants use that same scalar truth operation.
  Integer type-name calls consume original argument ranges and retained cast
  facts, while alias reference forms follow the C++ glvalue rule. Static member
  designation evaluates its object expression, preserving the frozen distinction
  between a static member reached through null and a nonstatic null-member access.
- `__builtin_constant_p` returns one for supported integral values or symbolic
  null/literal roots and zero for genuine nonconstant results or other addresses.
  An unported execution path still returns 86; no generic zero hides missing
  semantics. Indirect function-reference/pointer constant calls consume the
  retained function identity and diagnose null/non-constexpr calls. Actual
  constexpr bodies/frames remain a later owning group.
- Singleton ordinary function operands retain their known identity for casts,
  standard conversion sequences and ADL of their signature types. Genuine
  overload sets and partial explicit template bindings still require target
  deduction. The selected function/conversion remains recorded on the original
  operand; losing candidates publish no selection. Explicit reference-cast
  conversions are noninitializing facts, preventing an extra fabricated
  temporary-conversion node in the semantic view.

No syntax/tree copy, text transport, reparse, source recognition, C++ production
fallback, new dependency, unsafe code, environment read, blanket lint allowance
or policy change was added. The two new files own distinct value/place and
arithmetic/bounds algorithms. When strict Clippy and the companion audit caught
oversized dispatchers, their cohesive reference-cast and function-selection
algorithms were extracted without a waiver or a mechanical part split. Backend,
optimizer, allocator and generated-code policies are untouched; PA35 targets and
PA34 object equality are not claimed by semantic-view comparisons.

The ownership test proves compact array root identity, target byte offset 12 and
one-past state, a reference's distinct pointer-object view, retained literal
syntax identity, extended scalar storage, and integer result publication. Integer
facts never stand in for an address. Twenty-three required new regressions and
new immutable-oracle-derived references live in PA7's executed general lane;
existing compiler inputs, references and protected manifests were not changed.

### Exact validation and preservation evidence

| Command / check | Final result | Artifact relative to root |
| --- | --- | --- |
| `make build` | Pass, locked release workspace | `build-final.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | Pass, 198/198 fixed identities | `stage-required.log`, `fixed-manifest.json` |
| `make test-pa7` | Pass, same fixed coverage | `stage-make.log` |
| `make course-pa7` | Pass, 263/263, no failure marker | `course-final.log`, `course-result.json`, `course-counts.txt` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | Pass, all PA1–PA6 fixed and full course lanes | `prior-final.log`, `prior-summary.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | Pass | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | Pass | `preserve.log`, `preservation.json` |
| `make lint` / protected `clone_quality.py` | Pass, rustfmt, strict Clippy, 182 library tests in debug/release plus tool/doc tests and file audit | `lint-final.log` |
| `make audit-rust` | Pass, 134 files, zero findings | `audit-final.log` |
| `python3 .../pa7-address-audit-20261013/probe.py` | 147/147 protected address/conversion/publication comparisons | `address-probes.jsonl`, `address-summary.json`, `address-initial.jsonl` |
| Copied preceding full-stage `inherited-probe.py` | 62/62; all six preceding failures resolved | `baseline-probe.log`, `probes.jsonl`, `probe-summary.json` |
| Copied signature/initialization/scalar audit probes | 70/70, 78/78, 55/55 | `templates/`, `initialization/`, `scalar-audit/` probe summaries/receipts |
| `python3 .../pa7-address-audit-20261013/measure.py` | Exact workload outputs, 12 scaling receipts and two protected native qualifications | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

Fixed preservation is independently 198 entry/current passes, zero lost and
unchanged manifest coverage. `stage-entry.log` reuses the verified entry commit's
preceding checkpoint receipt; final gates were actually executed on this binary.
Full-lane identity preservation compares the entry commit's 240 `.t` identities
with the final trace: every entry ran and matched, exactly 23 new cases ran, and
all 263 receipts match. PA1–PA6 retain 58/26/20/106/196/121 fixed passes and
58/28/20/112/296/694 full-course passes. Raw outputs/traces remain under the
protected clone case/course artifact roots as well as supplementary mismatch
receipts. The transient xvalue-reference regression was caught by both the
preceding scalar probe and full course lane, corrected and rerun;
`course-regression-trace.jsonl` and `course-regression-result.json` retain that
failed attempt rather than presenting it as an accepted negative case.

### Resource observations and protected native qualification

Accepted warm measurements ran alone after all gate and probe jobs finished,
using A/A calibration followed by ABBA/BAAB samples. All source/binary hashes,
wall/RSS readings and output digests are retained; every digest agrees and all
12 protected scaling receipts match. An earlier overlapping-probe measurement
is retained with `-contended` filenames and is excluded from this table. Counts
are pointer/boolean declaration pairs; distinct-root inputs additionally declare
one array per pair. Observations include startup and dump I/O and make no
hardware-counter, equal-runtime or generated-code improvement claim.

| Workload / pairs | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| shared-root / 128 | 7.91 / 6476 | 25.73 / 8752 |
| distinct-roots / 128 | 7.14 / 6494 | 20.39 / 7994 |
| shared-root / 512 | 15.28 / 7222 | 38.06 / 8374 |
| distinct-roots / 512 | 16.21 / 7854 | 35.48 / 8370 |
| shared-root / 2048 | 32.83 / 11434 | 71.98 / 14156 |
| distinct-roots / 2048 | 34.03 / 12092 | 88.96 / 16310 |

Native probes fix 1,000 nested pointer conditionals and 1,500 parentheses around
an address comparison, starting at 1 MiB. Both protected receipts report
`match: true` and `stack_qualification.passed: true`, with identical successful
streams/files at a shared 16 MiB stack. Raw attempts/retries are retained under
`stack-qualifications/`; original exhaustion positions/exits are not treated as
language defects or calibrated to C++ frame sizes. The genuine byte-budget guard
is unchanged. Shared storage remains about 28.59 GB used under the
107,374,182,400-byte ceiling.

### Remaining acceptance work and next boundary

There are **no failing fixed/course identities or executed supplementary probe
IDs in this checkpoint**. The six sources listed by the preceding audit now all
match, including nonconstant pointer/array queries, null casts, object-address
queries, pointer comparisons and constexpr null equality. This closes that
concrete finding group, not the entire acceptance audit.

Continue the complete PA7 source-responsibility census/review of declaration and
procedural control flow, default visibility, conversion/ranking and semantic
rendering beyond these address paths. Keep the named later-class/template/
constexpr-frame boundaries recorded below distinct from due procedural work.
Do not infer full-stage acceptance from the green counts, erase unfinished
production owners, advance Ralph's stage, or claim PA33/PA34 obligations are met.


## Full-stage acceptance audit checkpoint — 2026-10-13

**Full-stage acceptance is withheld.** The clean reviewed entry was
`fd213562ce8bebe7a4bb1ed0e3fba1944e1535f4`. Its fixed and course lanes passed,
but independent source review and protected probes found missing PA7 behavior.
This checkpoint fixes a coherent integral-conditional, unevaluated-bound and
pointer-completeness group. Six successful-oracle address/constant probes remain
mismatched, and the broader stage-owned responsibility review must continue.
Neither passing fixtures nor this commit completes Ralph's existing phase goal.
Ralph state, goals, acceptance policy, frozen sources and protected files are
unchanged. The previous goal turn was authoritative progress in `fd21356`;
this audit revalidated the actual clean entry and its passing fixed identities.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-full-stage-audit-20261013/`.
`audit-scope.json` records the reviewed head, requirement digests and the explicit
nonacceptance decision. `checks.json` retains final commands, exits and elapsed
costs; `handoff.json` records this checkpoint's final commit and binary hashes.
Final candidate cppgm SHA-256:
`c89003571c08e0e3c803132303f7971190d75694057d1733c4095fb00229652f`.
Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

### Responsibilities reviewed and corrected

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, `docs/rust-clone-plan.md`,
the full PA7 handout, current ledger and relevant source-map responsibilities.
Consulted frozen C++ owners directly under
`/home/vishvananda/work/references/v5rust-cpp/dev/src/sema/`:
`analyzer_types.cpp::array_bound`, `analyzer_expressions.cpp`'s scalar operators,
`complete_pointee` and `subscript`, `analyzer_bodies.cpp`'s conditions, cases,
returns and control contexts, `analyzer_calls.cpp`'s indirect/resolved calls,
`conversions.cpp`, `analyzer_builtins.cpp::constant_p_call`,
`constant_evaluator.cpp`'s evaluator/conditional/size entries,
`constant_expressions.cpp::{conditional_value,assignment,increment}`,
`constant_interpreter.cpp`'s variable storage/read failures, and
`constant_statements.cpp`'s call/evaluable control flow.
`source-map.tsv` records the corresponding partial Rust owners.

- `constants.rs::constant_conditional` evaluates only the selected integral arm
  after semantic typing has checked both. It consumes the original expression's
  common `TypeId` and explicit target conversion, then publishes its constant
  fact. An untaken ordinary call, assignment, increment, pointer conversion,
  indirection or volatile read imposes no evaluation obligation. Types mode
  retains its distinct integral evaluator and common-type behavior.
- Non-template semantic array bounds type the original expression before
  constant evaluation, matching the C++ entry. Ordinary/overloaded/indirect
  calls inside `sizeof`, prefix mutations and assignments therefore use the
  shared PA7 typed-expression path. Evaluation reads the recorded query value;
  it does not repeat overload selection or type an unevaluated operand again.
  Wrong arity and invalid operands still fail at the C++ owning boundary.
- Typed direct calls reject non-constexpr evaluation using the selected binding.
  Semantic variable reads preserve the leading article, volatile-read failure
  and prefix/postfix read location. Simple external scalar assignments check
  the right value before the left place/write; compound assignments read the
  left first. Parameters, ordinary variables and unsupported places remain
  distinct. General constexpr execution and pointer-place state remain explicit
  missing dependencies, rather than fabricated successful rejections.
- `scalar_operators.rs::complete_pointer` is now read-only. It checks void,
  unknown-bound-array and direct incomplete-class pointees without demanding
  size/layout. This preserves arithmetic on complete derived-class pointers
  even while class layout is a later owner. Incomplete-class arithmetic uses
  the exact arithmetic diagnostic. Subscripts reject void, but preserve the
  frozen C++ acceptance of incomplete-class/unknown-bound-array pointees.
  Pointer subtraction's separate existing control flow is unchanged.

Eleven new `.t` regressions and their immutable-oracle-generated new `.ref`
pairs live in PA7's executed `tests/general/` lane. They cover untaken effects,
`sizeof` call/effect bounds, invalid indirect arity, selected call/variable/
assignment/increment/volatile failures, incomplete-class subscripts/arithmetic,
and complete derived-pointer arithmetic without layout demand. Existing inputs
and reference files were not modified or formatted.

The ownership test traces a mixed-type conditional and a `sizeof` overload
through their original syntax IDs. The selected literal retains its source
category/type and conversion; the untaken call retains its selected binding
without a value fact. The query retains the selected defined overload, with no
trailing function demand. Derived-pointer arithmetic leaves both layout and
construction unstarted. The billion-element-tail invariant now verifies the
new facts on both written array bounds: nine bounded source/initializer records
instead of seven initializer-only records, and no zero-tail element work.

Strict Clippy initially caught a 249-line evaluator after the fixes; conditional
evaluation was moved into its cohesive owning algorithm, without a waiver or
mechanical split. The existing storage test initially caught the two additional
bound facts; its corrected assertion explicitly proves their original IDs and
constant facts. Final debug/release tests and the companion source audit pass.
No unsafe code, new dependency, blanket allowance, environment read, fixture
recognition, production C++ delegation, syntax clone or text transport was added.
The semantic tool still runs the shared Rust frontend directly; oracle adapter
invocations supply comparison evidence only. No lowering, optimizer, allocator,
backend lifetime, code-generation policy or PA35 target was changed.

### Validation and preservation

| Command / evidence | Final result | Artifact relative to root |
| --- | --- | --- |
| `make build` | Pass, locked release workspace | `build-final.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | Pass, 198/198 fixed identities | `stage-required.log`, `fixed-manifest.json` |
| `make test-pa7` | Pass, same 198/198 | `stage-make.log` |
| `make course-pa7` | Pass, 240/240; original 229 plus 11 new | `course-final.log`, `course-result.json`, `course-counts.txt` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | Pass, all PA1–PA6 fixed and original course lanes | `prior-final.log`, `prior-summary.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | Pass | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | Pass | `preserve.log` |
| `make lint` / protected `clone_quality.py` | Pass, rustfmt, strict all-feature Clippy, debug/release unit/doc tests and audit | `lint-final.log` |
| `make audit-rust` | Pass, 132 files, zero findings | `audit-final.log` |
| `python3 .../pa7-full-stage-audit-20261013/probe.py` | 56/62; six explicit acceptance findings remain | `entry-probes.jsonl`, `entry-probe-summary.json`, `probes.jsonl`, `probe-summary.json` |
| `python3 .../pa7-full-stage-audit-20261013/supplement.py` | 20/20 selected-failure/type/completeness comparisons | `entry-supplement.jsonl`, `supplement.jsonl`, `supplement-summary.json` |
| `python3 .../pa7-full-stage-audit-20261013/measure.py` | Exact paired workload results and two protected native qualifications | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

`preservation.json` independently compares all 198 entry passes by identity:
198 retained, zero lost, unchanged fixed coverage. `course-preservation.json`
compares the entry commit's source identities with the final full-lane trace:
all original 229 ran, all 240 receipts match, and exactly 11 new cases ran.
No failed count or course failure marker remains. PA1–PA6 fixed counts are
58/26/20/106/196/121; their full course counts are 58/28/20/112/296/694, all
passing. These proofs establish preservation and the corrected group, not full
stage completeness. PA33 release/census and PA34 object equality are not due
at PA7, and no such completion claim is made.

### Resource observations and stack qualification

Final-binary warm process measurements ran after the gate jobs ended, with A/A
calibration followed by paired ABBA/BAAB samples. Every source/binary digest,
wall/RSS sample and output digest is retained. All 12 protected scaling receipts
match, and all measured semantic outputs agree. These observations include
startup and dump I/O; they do not measure native generated-code improvements.

| Workload / declarations | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| selected-conditionals / 128 | 7.43 / 6516 | 22.53 / 8368 |
| unevaluated-bounds / 128 | 7.19 / 5780 | 19.99 / 8740 |
| selected-conditionals / 512 | 9.78 / 6498 | 25.25 / 8366 |
| unevaluated-bounds / 512 | 9.05 / 6452 | 23.75 / 8002 |
| selected-conditionals / 2048 | 19.20 / 8628 | 47.01 / 9296 |
| unevaluated-bounds / 2048 | 14.88 / 8584 | 41.06 / 9274 |

Native probes fix 1,000 nested ordinary calls inside an array-bound `sizeof`
and 1,000 nested conditional branches with an untaken call, starting at 1 MiB.
Both protected receipts report `match: true` and
`stack_qualification.passed: true`. Both tools succeed with identical bytes at
a shared 16 MiB stack; raw attempts/retries remain under `stack-qualifications/`.
Only the protected receipt's match establishes qualification. Original adapter
exits and physical exhaustion positions are retained, not calibrated. The
genuine byte-budget guard remains intact. Shared storage was about 28.55 GB
used, below the 107,374,182,400-byte budget.

### Exact open findings and next dependency boundary

The following source identities, relative to this artifact root, have successful
C++ results and remain mismatched after the fixes:

```text
builtin-pointer-nonconstant.cpp       Rust 86, C++ 0
builtin-array-object.cpp              Rust 86, C++ 0
builtin-null-cast.cpp                 Rust 86, C++ 0
builtin-address-object.cpp            Rust 86, C++ 0
builtin-pointer-compare.cpp           both 0, wrong folded semantic bytes
constexpr-null-compare.cpp            Rust 1, C++ 0
```

The first four expose the broad `builtin_calls.rs` missing-address boundary;
the last two expose integral evaluation of pointer/null comparisons. They are
not accepted negative cases, resource qualifications or oracle issues. Sources,
exact streams and mismatch directories remain in the protected probe receipts.
The next coherent group is typed symbolic null/object/function/literal address
facts and the owning constant place/scalar conversion/equality operations:
`constant_interpreter.cpp`, `constant_expressions.cpp`, `constant_values.h`,
`analyzer_builtins.cpp`, `constants.rs` and `builtin_calls.rs`. Preserve compact
entity identity and target offsets; do not return zero generically for an
unsupported constant query. Then resume the complete stage-owned C++ census,
control-flow, conversion/ranking and dump review beyond the current probes.
Existing later template-body/class-construction boundaries remain separately
recorded below; passing their limited PA7 fixtures does not erase them.

## Ordinary function-template signature checkpoint — 2026-10-13

Completed a coherent signature-specialization group for the `pa7 full-stage`
implementation phase. The preceding turn was progress: `77d7de1` completed
ordinary nonvirtual inheritance and reduced fixed failures from three to one.
This entry inspected the clean checkout, reread the contract, specification,
plan, owning handout and current ledger, consulted frozen C++ owners directly,
and reran the entire entry manifest. Stage base and last-audited commit above
remain unchanged; implementation does not advance Ralph's stage or audit pointer.

The unchanged fixed manifest improves **197/198 → 198/198**, **1 → 0 failures**,
with identical identities and all 197 entry passes preserved. The full original
course lane improves **228/229 → 229/229**; its additional 31 cases execute and
pass. Exact fixed-stage failure reduction comes from the existing identity
`tests/general/300-static-cast-overloaded-function-template-argument.t`, rather
than added tests. No fixed or course failures remain in this run. Passing these
lanes establishes this checkpoint's validation, not implementation-wide PA7
completion or advancement; the remaining production boundaries below are explicit.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-function-templates-20261013/`.
`baseline.log` and `baseline-manifest.json` retain the independent turn-start
run. `protected-stage-result.json`, `course-result.json`, `preservation.json`,
`checks.json` and `handoff.json` retain final identity coverage, exit statuses,
evidence and binary hashes. Protected per-case diagnostics and raw outputs remain
under `../clone/cases/pa7/<case-hash>/`; supplementary sources, mismatches and
attempts remain in this checkpoint artifact root. Artifacts are not committed.

### Frozen owners and typed data flow

Consulted `/home/vishvananda/work/references/v5rust-cpp/dev/src/sema/` directly:
`function_templates.cpp::{explicit_specializations,SoleTemplateHead,specialize,
complete_deduction,deduced_specialization,substitute,substitute_function,without_cv,
deduce_type,deduce_argument,deduce_overloaded,deduce_for_call,deduce_for_target,
function_type_at_least_as_specialized,reference_ordering}`;
`template_specializations.cpp::declaration_pattern`;
`analyzer_expressions.cpp::{select_overloaded,resolve_overloaded,typed}`;
`analyzer_calls.cpp` candidate collection/conversions;
`analyzer_declarations.cpp` retained template parameter clauses;
`semantics_dump.cpp::{schedule,trailing,specialization_declaration}`;
`function_records.h` and `template_records.h`. `source-map.tsv` records partial
owners and boundaries. Syntax display reuses the existing `ast_render.cpp`
translation, rather than creating or reparsing name text.

- `function_templates.rs` owns signature formation, type-only argument overlays,
  substitution, structural deduction and partial ordering. Unit-owned signature
  keys contain canonical primary `EntityId` and argument `TypeId`s. Explicit
  `InProgress` and `Complete(BindingId)` states prevent duplicate formation;
  failed substitutions remove their entry rather than caching a context-dependent
  absence. A query's immutable argument overlay is borrowed by all substitution
  frames, with a query-owned type memo. Successful type-default memos use the
  canonical head, parameter index and preceding arguments; defaults cannot legally
  replace an already used default, and adding later defaults cannot invalidate
  these successful entries. Non-type parameters, packs and dependent class/alias
  signatures remain explicit dependencies.
- `graph.rs` privately publishes hidden function bindings with compact IDs,
  canonical entity origin, declaration home and source access. Hidden partial
  explicit-argument and specialized bindings do not enter ordinary name indexes,
  overload indexes or printed scope lists. Using declarations retain the original
  entity's namespace. Source parameter clauses remain borrowed AST identities,
  following the frozen redeclaration naming rules. Canonical template parameter
  entities retain positional facts so deduction never keys on printed types/names.
- `lookup.rs`, `function_resolution.rs` and `procedural_calls.rs` distinguish
  explicit id/address specialization from call deduction. Calls first perform
  indexed ordinary/ADL lookup, then retain explicit arguments; invalid call
  substitution therefore produces the frozen call diagnostic instead of an
  id-expression specialization diagnostic. Arity filtering precedes deduction.
  Explicit arguments permit ordinary conversions; inferred arguments remain
  consistent across parameters. Structural deduction supports pointer/reference,
  fixed-array, function and member-pointer type edges, reference collapsing,
  argument decay and cv removal, forwarding references, and target-type forwarding
  adjustment. Non-template preference and structural/reference partial ordering
  run only after conversion ordering ties.
- Completed selection and conversion facts are retained on the original expression
  IDs. Only the selected function contributes demand; losing signature candidates
  instantiate no body/default and produce no trailing declaration. Primary-keyed
  demand records make later template-definition arrival an explicit dependency
  event. Both already-defined and later-defined demanded template bodies remain
  status 86, as do selected omitted template default arguments; these are not
  fabricated language rejections. Deleted singleton specializations diagnose at
  the original designation before any body demand.
- `semantics_dump.rs` consumes selected bindings, retained parameter clauses and
  function-owned demand facts. It prints demanded specialization declarations,
  canonical parameter types and original parameter names in first-use order.
  `syntax::write_expression_name` preserves original template-id spellings,
  including partially explicit and empty argument lists. The printer performs
  no lookup, deduction, substitution, body analysis or semantic demand.
- The ownership regression in `sema/tests.rs` checks repeated specialization
  identity, unchanged ordinary overload membership, compact selected origins,
  and exclusion of losing specializations from the trailing demand queue.

### Validation and exact evidence

Candidate cppgm SHA-256:
`011c76002ddeba49a8f1b8ed87fc1d838a93de3aa2cf10c5da17a410df07e183`.
Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | Pass | `build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | Pass, 198/198 fixed identities | `stage-required.log`, `protected-stage-result.json` |
| `make test-pa7` | Pass, same 198/198 | `stage.log` |
| `make course-pa7` | Pass, 229/229, no failure marker | `course.log`, `course-result.json`, `course-counts.txt` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | Pass, PA1–PA6 fixed and course lanes | `prior.log`, `prior-summary.json` |
| `make lint` | Pass, rustfmt, strict Clippy, unit/doc tests and protected quality | `lint.log` |
| `make audit-rust` | Pass, zero findings | `audit.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | Pass | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | Pass, all 197 previous identities | `preserve.log`, `preservation.json` |
| `python3 .../pa7-function-templates-20261013/probe.py` | 70/70 exact protected comparisons | `probe-summary.json`, `probes.jsonl` |
| `python3 .../pa7-function-templates-20261013/measure.py` | Exact supported scaling and native qualification | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

The 70 exact supplementary comparisons cover full/partial/empty explicit argument
lists, type defaults, call and target deduction, returning-type deduction,
forwarding/cv/reference/array/function/member-pointer signatures, namespaces,
using declarations, ADL, repeated/different specialization identities,
non-template preference, structural/reference ordering, inconsistent deduction,
invalid pointer/reference/return substitutions, deleted functions, arity filtering,
variadic calls, renamed template redeclarations, unused definitions and unevaluated
body use. They supplement the unchanged fixed/course lanes; they do not count as
fixed-manifest progress. `boundary.jsonl` retains successful-oracle reproducers
for eight remaining boundaries. Every receipt judges `match`, including resource
qualification, rather than comparing native exhaustion locations.

The earlier Ralph progress receipt treated the fixed lane's 198 results as a
229-case run, yielding 31 unrun cases. `harness-counts.json` retains the unchanged
external receipt and both lane counts. This run executes all 198 fixed entries
and separately all 229 course entries, both passing. Checker policy, manifests,
fixtures, references, frozen sources and Ralph state are untouched. Ralph owns
interpretation, checkpoint/final audits and advancement.

### Resource measurements and protected stack qualification

Measurements run after the gate jobs finish, with A/A calibration followed by
paired ABBA/BAAB blocks. Every input/binary hash, sample, wall/RSS reading, exit,
output digest and protected receipt is retained. All workload output digests
agree and all 18 protected workload receipts match. These warm process readings
include startup, parsing, semantic analysis and dump I/O; they are not a cold-cache
or hardware-counter gate. PA35 generated-code improvement targets do not apply.

| Workload / count | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| distinct-signatures / 128 | 6.83 / 6486 | 20.01 / 8730 |
| shared-signature / 128 | 7.08 / 6506 | 21.07 / 8746 |
| distinct-signatures / 512 | 12.13 / 8604 | 31.82 / 8724 |
| shared-signature / 512 | 10.27 / 7182 | 31.01 / 7996 |
| distinct-signatures / 2048 | 40.07 / 15608 | 87.60 / 17712 |
| shared-signature / 2048 | 19.10 / 10048 | 51.68 / 9248 |
| overload-arity / 16 | 5.44 / 5838 | 17.82 / 8736 |
| overload-arity / 64 | 7.47 / 6502 | 20.29 / 8370 |
| overload-arity / 256 | 29.56 / 11444 | 67.56 / 8670 |

Distinct signatures scale one template over independent class identities; shared
signatures scale repeated uses of one specialization. Overload arity scales
source declarations of one name with progressively longer parameter lists and
one one-argument call. The last workload intrinsically grows total source/parameter
volume quadratically in candidate count; its measurement does not establish a
linear bound in candidate count alone. Queries visit only indexed candidates;
substitution memos and the dependency walk avoid revisiting a shared type DAG.
Partial-order unique placeholder identities are bounded by the largest queried
head and never complete or instantiate source classes.

Native probes fix 1,000 nested function-template calls and 1,500 parentheses in a
template call argument, starting at a 1 MiB stack. Both protected receipts have
`match: true` and `stack_qualification.passed: true`, with identical successful
results at a shared 16 MiB retry. Raw originals and retries remain under
`stack-qualifications/<receipt-id>/`. The genuine byte-budget guard is retained;
no native cutoff, frame size or exhaustion diagnostic position was calibrated.
Shared storage remains below the 107,374,182,400-byte ceiling.

### Next dependency boundary and architecture review

No fixed PA7 failures remain. The next template group is demand-owned body/default
instantiation and dependent signature substitution: `function_templates.cpp`,
`class_templates.cpp`, `template_names.cpp`, `template_specializations.cpp`, and
retained fact-bearing body instantiation. It must bind immutable parent-linked
environments, preserve definition-time name lookup and access, allocate instance
facts once, and queue definition demand by specialization identity. Non-type and
pack argument formation, class/alias instantiation, dependent names, noexcept,
member-template contexts and expanded ordering remain separate related owners.
The inherited broader class-call/user-conversion/construction groups also remain
incomplete; these passing lanes are not evidence of their completion.

`boundary-body-demand.cpp`, `boundary-late-body-demand.cpp`,
`boundary-default-demand.cpp`, `boundary-explicit-default-demand.cpp`,
`boundary-pack.cpp`, `boundary-non-type.cpp` and `boundary-class-signature.cpp`
all retain candidate status 86 against successful C++ output. The earlier phase
`boundary-data-member-address.cpp` reaches the existing data-member designation
rejection (candidate 1, oracle 0); pointer-to-data-member address formation belongs
to `analyzer_member_pointers.cpp`/`analyzer_expressions.cpp`, before template
signature deduction. None of these probes is treated as a matched negative case.

Review a template-id from its single parsed name through indexed lookup, hidden
explicit bindings, canonical signature memoization, recorded call/operand facts
and the first-use trailing declaration. Then review two candidate specializations
where only the selected function demands output, and a later template definition
that triggers the recorded primary's dependency boundary. Single-AST ownership,
compact IDs, immutable type identity and per-query overlays are preserved. No
unsafe code, FFI, compiler fallback, text reparsing, fixture recognition, new
dependency, lint suppression, global retry or speculative class/body work is added.
The optimizer, code generator, register allocator and backend lifetimes are
unchanged; this group makes no PA34 object-parity claim.

## Ordinary nonvirtual inheritance checkpoint — 2026-10-13

Coherent partial handoff for the `pa7 full-stage` implementation phase; PA7
remains incomplete. The preceding turn was progress: `ba31318` completed
checked scalar-class prvalue copy elision and reduced fixed failures from four
to three. This entry inspected the clean checkout, reread the contract, spec,
plan, handout and ledger, consulted frozen C++ owners, and reran all 198 entry
cases. Stage base and last-audited commit above remain unchanged; implementation
does not advance Ralph's stage or audit pointer.

The fixed manifest improves **195/198 → 197/198**, **3 → 1 failures**,
with identical case identities and all 195 previous passes preserved. The
original course lane improves **226/229 → 228/229**; its additional 31 cases
execute and pass. Both lanes still fail on the function-template case below.
Fixtures, references, frozen C++ sources, protected checks, Ralph state and
acceptance policy are unchanged. Optimizer, register allocator, lower/native
code and backend lifetimes are unchanged; no PA34 object-parity claim is made.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-inheritance-20261013/`.
`baseline.log` and `baseline-manifest.json` retain entry coverage. Final
`protected-stage-result.json`, `course-result.json`, `preservation.json`,
`checks.json` and `handoff.json` retain exits, identities and binary/evidence
hashes. Per-case protected diagnostics and raw outputs remain under
`../clone/cases/pa7/<case-hash>/`; supplementary attempts are in `probes.jsonl`,
`boundary.jsonl`, `mismatches/` and the generated source files. Artifacts and
scratch inputs are not committed.

### Frozen owners and typed responsibilities

Consulted `/home/vishvananda/work/references/v5rust-cpp/dev/src/sema/` directly:
`analyzer_classes.cpp::{class_body,base_classes,distinct_direct_bases}`,
`lookup.cpp::{class_members,merge_base,derives_from}`,
`conversions.cpp::{is_base_of,reference_related,standard,reference,compare}`,
`analyzer_access.cpp::{check_base_access,base_accessible,check_unqualified_access}`,
`analyzer_member_pointers.cpp::{member_subobject,check_conversion_access,
upcast_base,downcast_base}`, `analyzer_expressions.cpp::{member_access,
member_qualifier,member_of_object,explicit_conversion}`,
`analyzer_calls.cpp::associated`, `analyzer_initialization.cpp::implicit_constructor`,
`analyzer_construction.cpp::{implicit_default_facts,constructor_plan,
default_constructor,declare_destructor}` and `semantics_dump.cpp::conversion_node`.
`source-map.tsv` records the partial translations and dependent owners.

- `inheritance.rs` owns source-ordered `BaseClass` edges, enum-valued derivation
  permission and a private unit path store. Formation resolves elaborated or
  decltype base names in the class's parent scope before members, strips type
  cv by canonical class identity, then checks completeness, final/union/pack
  rules, duplicate direct identities and access. Dependent and virtual bases
  remain explicit status-86 boundaries. No member body or layout is demanded.
- `lookup.rs` searches indexed own names before source-ordered base edges.
  Explicit frames and query-local scope memos avoid native recursion and
  repeated shared-graph traversal. A memo retains a representative declaring
  scope or absence, collecting its overload vector only once at the result.
  Qualified class-function addresses retain the frozen ambiguous-member diagnostic.
  Own declarations hide branches; equal entities/designated types converge;
  differing declarations from nonvirtual branches remain ambiguous. This is
  separate from namespace overload merging. Memos live for one query, so no
  declaration event can make a cached absence stale.
- `inheritance.rs` counts subobjects with a graph walk, saturating at two;
  repeated nonvirtual paths never expand exponentially. Successful unique
  paths are memoized by canonical `(derived EntityId, base EntityId)` and
  retained through compact `BasePathId` facts. The base graph is immutable
  after formation. Failed counts and context-dependent access are query-local.
  Member access retains original designated `BindingId`, naming class, cv and
  value category. Object-to-naming and naming-to-declaring ambiguities are
  checked separately. Static members, enumerators and types use accessible
  edge walks independently of instance multiplicity. Protected object checks
  consume lexical, declaring and object class identities.
- `scalar_conversions.rs` records typed `DerivedToBase` pointer and direct
  reference sequences, with exact cv/qualification flags and conversion rank.
  `function_resolution.rs` preserves subsequence ordering, less-qualified and
  nearer-base preferences, and base-versus-void ranking. Ranking performs no
  access check; the selected call or initializing conversion checks ambiguity
  and access, then retains its path. Original argument locations are preserved.
  Dense original-AST expression scopes provide the conversion's access point.
- `scalar_casts.rs` owns separate reference hierarchy and inverse pointer
  conversion helpers. Static hierarchy casts require an unambiguous accessible
  base; C-style casts retain their access/cv freedom while still rejecting
  ambiguous bases. Original operand types/categories remain beneath carriers.
  `semantics_dump.rs::conversion_node` consumes recorded initializing
  conversions to print exact pointer/reference cast wrappers; it performs no
  lookup, ranking, path query or demand.
- `construction.rs` completes implicit trivial nonvirtual base plans bottom-up
  on demand and retains source-ordered child `ConstructorId`s. Already completed
  plans are reused. User-provided declarations do not demand their bases'
  default construction. Trivial base plans create no extra definition roots.
  Deleted implicit base construction retains the frozen location-zero planning
  diagnostic. Nontrivial/declared/defaulted/written or extended base action
  plans and demanded declared base destruction remain explicit incomplete
  dependencies; layout and copy/move consumers also guard their missing base
  plans rather than silently ignoring bases.
- `call_lookup.rs` adds complete direct/indirect base types to ADL without
  counting inheritance depth against the original nested-type budget. No
  unrelated namespace or declaration is scanned.

Architecture review: one parsed AST, canonical compact identities, dense facts,
private path-store APIs, explicit successful construction states and separate
emission demand. The two new ownership regressions prove shared path identity,
original operand typing, retained child plans, no speculative layout or extra
base emission roots, and saturated counts on a 64-level shared diamond graph.
Query storage is reclaimed with its query. No syntax synthesis, reparsing,
whole-program retry, compiler subprocess, environment read, unsafe code,
new dependency or lint suppression was introduced.

### Validation and evidence

| Command | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | 197/198, one failure, exit 1 | `stage-required.log`, `protected-stage-result.json` |
| `make test-pa7` | Same fixed coverage/results, make exit 2 | `stage.log` |
| `make course-pa7` | 228/229, same failure, make exit 2 | `course.log`, `course-result.json`, `../clone/course/pa7/test/trace.jsonl` |
| `python3 .../clone_check.py prior pa7` | PASS, PA1–PA6 fixed and course lanes | `prior.log`, `prior-summary.json` |
| `python3 .../clone_check.py preflight pa7` | PASS | `preflight.log` |
| `python3 .../clone_check.py preserve pa7` | PASS; independent 195-identity preservation also passes | `preserve.log`, `preservation.json` |
| `make lint` / protected `clone_quality.py` | PASS, rustfmt, strict all-feature Clippy, 179 debug/release library tests each | `lint.log` |
| `make audit-rust` | PASS, 131 files, zero findings | `audit.log` |
| `python3 .../pa7-inheritance-20261013/probe.py` | 87/87 exact protected matches | `probe.log`, `probe-summary.json`, `probes.jsonl` |
| `python3 .../pa7-inheritance-20261013/boundary.py` | Nine status-86 boundaries, oracle success | `boundary.log`, `boundary.jsonl` |
| `python3 .../pa7-inheritance-20261013/measure.py` | Exact supported scaling and protected native qualification | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

The 87 supplementary probes cover formation and alias/decltype lookup,
qualification and hiding, repeated subobjects versus static/type names,
private/protected access and object restrictions, ADL, implicit trivial base
construction and nondemanded declared constructors, cv-qualified pointer and
reference initialization, hierarchy casts, qualification/base/void overload
ranking, selected-call access and ambiguity diagnostics. The nine successful-
oracle boundary reproducers cover virtual formation, base layout/copy,
written/declared/defaulted/nontrivial/polymorphic base construction and base
destruction. Their status 86 is an implementation failure, never a matching
C++ rejection. Stage/course exits above remain failing checks.

The unchanged Ralph progress receipt treats 198 fixed results as a 229-case
run and reports 31 unrun cases. `harness-counts.json` retains the external
failure log, manifest cardinality and the complete fixed/course receipts as a
reproducer. The commands actually execute 198 fixed and 229 course entries;
`preservation.json` proves exact fixed failure reduction from three to one.
No checker policy or Ralph state was edited; advancement is Ralph's decision.

### Resource measurements and protected stack qualification

Measurements ran after gate jobs completed, with A/A calibration followed by
paired ABBA/BAAB blocks. Inputs and both binaries are hashed; each sample
retains wall time, peak RSS, exit, output size and digest. All successful raw
output digests agree and all 18 protected workload receipts match. The three
workloads scale distinct base/member queries, repeated conversions sharing a
path, and one hierarchy's depth plus an implicit trivial construction demand.
Hot-path runs use a fixed shared 16 MiB stack. These are warm process
observations including startup, parsing, semantics and dump I/O, not a cold
cache or hardware-counter gate; PA35 improvement targets do not apply.

| Workload / count | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| distinct-bases / 128 | 8.16 / 7180 | 23.15 / 7986 |
| shared-path / 128 | 7.32 / 6504 | 19.92 / 8746 |
| base-depth / 128 | 6.62 / 6512 | 18.32 / 8744 |
| distinct-bases / 512 | 15.78 / 9304 | 41.98 / 8368 |
| shared-path / 512 | 10.17 / 7182 | 28.49 / 8008 |
| base-depth / 512 | 9.33 / 7224 | 22.82 / 8000 |
| distinct-bases / 2048 | 45.81 / 19902 | 102.33 / 21942 |
| shared-path / 2048 | 24.31 / 10726 | 53.96 / 10788 |
| base-depth / 2048 | 16.69 / 12108 | 36.51 / 8740 |

Native probes fix 1,000 nested ordinary pointer-returning calls with
derived-to-base arguments and 1,500 parentheses in a pointer upcast, starting
at a 1 MiB stack. Both protected receipts have `match: true` and
`stack_qualification.passed: true`, with both tools succeeding identically at
a shared 16 MiB retry. Raw original attempts and retries remain under
`stack-qualifications/<receipt-id>/`. Only these protected receipts establish
qualification; the adapter's original exit/streams are not successful results.
The genuine byte-budget guard is unchanged. No native cutoff, frame size or
exhaustion diagnostic position was calibrated. Shared storage measured
28.53 GB used under the 107.37 GB ceiling.

Candidate cppgm SHA-256:
`f8990852e389951d8f95bd0556d06b0ad00f860b1b270e480115d4519cf59632`.
Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.


### Newly passing identities and next dependency boundary

The exact newly passing fixed identities, relative to `pa7/`, are:

```text
tests/general/100-qualified-base-data-member-hides-function.t
tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t
```

The exact remaining fixed/course identity is:

```text
tests/general/300-static-cast-overloaded-function-template-argument.t
```

It stops at `lookup.rs::lookup_result` with explicit status 86 for template
specialization lookup. Its next coherent group is retained-pattern function
specialization with explicit arguments, target-directed deduction and call
deduction, from the frozen template and overload owners. Template bodies must
retain their parsed representation and separate declaration/body/emission demand.

Related inheritance dependencies remain explicit: virtual/dependent base
formation and lookup; nonvirtual subobject layout including empty-base placement;
written/defaulted/nontrivial/polymorphic base construction and declared base
destruction; class subobject copying/moving/assignment; inherited member-pointer
adjustments, bound member calls, friendship and full deferred access. The new
base identities and path facts are ready for those owners; they must not infer
plans from dump text or introduce speculative class/body work. This completed
ordinary nonvirtual lookup/conversion group is a coherent partial handoff,
not PA7 or inheritance completion.

## Checked scalar-class prvalue copy elision checkpoint — 2026-10-13

Coherent partial handoff for the `pa7 full-stage` implementation phase; PA7
remains incomplete. The previous goal turn was progress: `2aed55f` translated
scalar special-member construction/lifetime facts and reduced fixed failures
from six to four. This turn revalidated the clean checkout and complete entry
manifest. Stage base and last-audited commit above are unchanged; implementation
does not advance the stage or its independent audit pointer.

The fixed manifest improves **194/198 → 195/198**, **4 → 3 failures**.
Coverage has identical case identities and retains all 194 prior passes.
The course lane improves **225/229 → 226/229**, with the same three failures.
The additional 31 course cases execute. This is a completed checked-prvalue-
copy-elision group, not completion of all class copying or PA7.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-copy-elision-20261013/`.
`baseline.log` and `baseline-manifest.json` retain the independent turn-start
run; `protected-stage-result.json`, `preservation.json` and `checks.json` retain
final coverage, identity preservation, exits and hashes. Raw protected case
receipts, diagnostics and output files remain in
`../clone/cases/pa7/<case-hash>/`. Development attempts are retained under
`development-validation/` and `mismatches/`; only the final receipts below
support the handoff. Sources, dumps, measurements and logs are uncommitted
artifacts. No protected fixtures, references, oracle sources, checks, policy
or Ralph state changed.

### Frozen owners and typed data flow

Read the frozen `/home/vishvananda/work/references/v5rust-cpp/dev/src/sema/`
owners directly: `analyzer_copy.cpp::{copy_kind,classify_copy_members,
implicit_copy_members,declare_implicit_copy_members,implicit_copy_member,
const_copy_source,defaulted_copy_facts,constructor_candidates}`;
`analyzer_construction.cpp::{initialize_class,direct_initialization,
same_class_source,copy_initialization,record_copy,select_constructor,
check_destruction}`; `analyzer_classes.cpp` completion;
`analyzer_initialization.cpp::{initialize_variable,type_default_arguments}`;
`analyzer_calls.cpp::convert_arguments`, `analyzer_bodies.cpp` return typing,
`analyzer_declarations.cpp` special/defaulted definitions,
`conversions.cpp` reference binding and `semantics_dump.cpp` initialization
views. The source map records the translated responsibilities and boundaries.

- `copy_initialization.rs` classifies original non-template constructor
  identities at class completion, using canonical parameter types and recorded
  default arguments. A later addition of a default does not retroactively
  reclassify the declaration. Constructor candidate viability still observes
  the defaults visible at the use, matching the distinct C++ responsibilities.
- Implicit copy/move candidates are demanded once per complete scalar-subobject
  class. They retain canonical const-lvalue-reference/rvalue-reference parameter
  IDs and memoized scalar deletion facts. Own constructor, assignment and
  destructor indexes determine suppression/deletion. Reference members,
  rvalue-reference copy deletion, scalar arrays, const members and union storage
  follow the frozen owner. No class-subobject plan is fabricated.
- Query scratch retains capacity and clears contents on success or failure.
  Candidates are filtered by explicit policy, arity/defaults and standard
  parameter binding. Deleted defaulted moves are ignored; deleted declared
  moves still participate. Best-against-all ranking distinguishes ambiguity,
  no constructor set and no viable candidate. Access/deletion checks precede
  successful elision. Class completion and selection inspect only the owning
  class's indexes, not unrelated declarations.
- Original `ExprId` facts retain `CopySelection`: a declared origin `BindingId`
  or implicit `(EntityId, RefKind)` identity and the full selected parameter
  `Conversion`. The source type/category and any existing operand conversion
  remain intact. Parentheses share the original fact. A class prvalue binds a
  reference directly rather than through a scalar conversion-created temporary.
- Variable assign/direct initialization, by-value defaults, selected direct and
  indirect call arguments, returns, and functional/static/C-style casts consume
  those facts. Elision creates no constructor/body emission demand. Written
  bodies still receive normal declaration-time analysis and retain dependencies
  on their own function. Returns use the original return location; initializer,
  cast and argument diagnostics use their owning source locations.
- `construction.rs` checks late-defaulted scalar copy deletion and by-value
  parameter destructor access/deletion in caller context. The semantic view
  keeps the original initializer expression with its original cv-qualified
  declared object type and emission order. The printer performs no new lookup,
  typing, selection or textual transport.

The ownership regression proves declared/implicit origin reuse, original
prvalue facts and parameter bindings, shared parenthesized facts, cv-qualified
object identity, absent copy actions/default-constructor demand, and that an
undemanded copy body's nested constructor dependency does not become a root.
Architecture review: one parsed AST, canonical compact identities, dense
facts, explicit class-completion classification, unit-owned immutable copy
memos and reusable query scratch. Scalar-member support/deletion is checked
once per demanded class rather than once per use. No syntax synthesis,
reparsing, whole-program retry, speculative body demand, environment access,
compiler subprocess, unsafe code, new dependency or lint suppression is added.
Optimizer, register allocator, lower/native code and backend lifetimes are
unchanged; this group makes no PA34 object-parity claim.

### Validation and preserved evidence

Candidate cppgm SHA-256:
`ce10daf8eeeb544ca8dbccc2077107a7151da8ac463ae3a757e065239d885953`.
Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | 195/198; three failures, exit 1 | `stage-required.log`, `protected-stage-result.json` |
| `make test-pa7` | Same fixed coverage/outcomes; make exit 2 | `stage.log` |
| `make course-pa7` | 226/229; same three failures, marker retained; make exit 2 | `course.log`, `course-result.json`, `../clone/course/pa7/test/trace.jsonl` |
| `python3 .../clone_check.py prior pa7` | PASS, PA1–PA6 fixed and course lanes | `prior.log`, `prior-summary.json` |
| `python3 .../clone_check.py preflight pa7` | PASS | `preflight.log` |
| `python3 .../clone_check.py preserve pa7` | PASS; independent 194-identity preservation passes | `preserve.log`, `preservation.json` |
| `make lint` / protected `clone_quality.py` | PASS, rustfmt, strict all-feature Clippy, 177 debug/release library tests each | `lint.log`, `lint-final.log`, `ownership-final.log` |
| `make audit-rust` | PASS, 130 files, zero findings | `audit.log` |
| `python3 .../pa7-copy-elision-20261013/probe.py` | 119/119 exact protected matches | `probe.log`, `probe-summary.json`, `probes.jsonl`, sources |
| `python3 .../pa7-copy-elision-20261013/boundary-probe.py` | Four explicit status-86 boundaries, oracle success | `boundary.log`, `boundary-probes.jsonl` |
| `python3 .../pa7-copy-elision-20261013/measure.py` | Exact supported scaling and native qualification | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

The stage/course failures are retained evidence of unfinished dependencies;
their commands are not reported as passing checks. `handoff.json` records
final commit and evidence/binary hashes. Rustfmt, strict Clippy and the
companion file audit pass without new lint expectations or suppression.


The 119 exact protected probes cover copy/direct initialization, const/volatile
results, parameter qualification ranking, private/deleted/explicit constructors,
first/later default definitions, declared copy/move competition, own assignment
and destructor suppression, reference/rvalue-reference/array/union members,
extra defaults, late-added defaults, repeated selection, namespace/local classes,
by-value direct/indirect calls, defaults, returns, casts and undemanded body
closure. They supplement the unchanged fixed and course manifests.
`boundary-probe.py` and `boundary-probes.jsonl` retain successful-oracle
reproducers for lvalue copies, class-subobject copies, conditional object copies
and user-conversion competition; each candidate path remains explicit status 86
and a mismatch, never a fabricated successful rejection.

The existing Ralph progress receipt counts the fixed lane's 198 results as a
229-case run and reports 31 unrun cases. `harness-counts.json` retains the
reproducer commands, unchanged external failure log and both complete receipts.
The fixed command executes all 198 entries; the course command executes all 229.
The independent identity receipt proves exact fixed failure reduction. Checker
policy and Ralph state are untouched; advancement remains Ralph's decision.

### Resource measurements and stack qualification

Measurements run after the gate jobs finish, with A/A calibration followed by
paired ABBA/BAAB blocks. Inputs/binaries are hashed; every sample includes
wall time, RSS, exit, output size and digest. All output digests agree and all
18 protected workload receipts match. Workloads scale distinct scalar-member
classes, repeated copy/move selection from one class, and scalar member counts
at 128 copies. They include warm process startup, parsing, semantics and dump
I/O, not a cold-cache or hardware-counter gate. PA35 generated-code improvement
targets do not apply here.

| Workload / count | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| distinct-copies / 128 | 6.89 / 7204 | 21.20 / 8362 |
| shared-selection / 128 | 4.66 / 5794 | 15.68 / 8008 |
| distinct-copies / 512 | 12.68 / 10030 | 29.93 / 8006 |
| shared-selection / 512 | 6.80 / 6492 | 23.51 / 7986 |
| distinct-copies / 2048 | 43.76 / 21974 | 83.67 / 17070 |
| shared-selection / 2048 | 14.85 / 8554 | 37.52 / 8000 |
| scalar-members / 16 | 5.60 / 5774 | 20.14 / 8374 |
| scalar-members / 64 | 4.98 / 5798 | 16.40 / 7988 |
| scalar-members / 256 | 5.94 / 6444 | 18.59 / 8006 |

Native probes fix 1,000 nested ordinary class-valued/by-value calls and 1,500
parentheses in a same-class prvalue initializer, starting at a 1 MiB stack.
Both protected receipts have `match: true` and `stack_qualification.passed:
true`, with both tools succeeding identically at a shared 16 MiB retry. Raw
original attempts and retries remain under `stack-qualifications/<receipt-id>/`.
Only the protected receipts establish qualification. The genuine byte-budget
guard is unchanged; no exhaustion cutoff, frame size or diagnostic position
was calibrated. Shared storage measured 28.51 GB used under the 107.37 GB ceiling.


### Remaining identities and next dependency boundary

The newly passing fixed identity is
`tests/general/300-elaborated-local-struct-copy-init.t`. The exact remaining
fixed identities are:

```text
tests/general/100-qualified-base-data-member-hides-function.t
tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t
tests/general/300-static-cast-overloaded-function-template-argument.t
```

The two inheritance failures first diverge at class-base formation
(`declarations.rs::class_type`, status 86). Their next coherent group needs typed
base edges/paths, base member lookup, implicit base construction, reference and
pointer derived-to-base sequences/ranking, access facts and static downcasts
from the corresponding frozen class/lookup/conversion owners. The template
failure needs retained-pattern function specialization and target/call deduction.

Within construction, nonelided lvalue/xvalue/conditional object copying,
copy/move subobject and assignment plans, general/list construction,
user-conversion competition, derived sources, implicit return moves and full
temporary/abstract/virtual lifetime semantics remain separate explicit
boundaries. The new selection facts are ready for those consumers; they must
not substitute scalar zero/default plans or infer semantics from dump text.
This coherent checked-elision partial handoff does not claim stage completion.

## Scalar special-member definitions and construction checkpoint — 2026-10-13

Coherent partial handoff for the `pa7 full-stage` implementation phase; PA7
remains incomplete. The preceding goal turn is classified as progress:
`71998f2` translated class-function identity/address/demand and reduced fixed
failures from 12 to 6. This entry revalidated the clean checkout, read the
owning guides, inspected frozen C++ owners and reran the complete turn-start
manifest. Stage base and last-audited commit remain unchanged; implementation
does not advance the stage or the independent audit pointer.

The fixed manifest improves **192/198 → 194/198**, **6 → 4 failures**,
with identical coverage and all 192 previously passing identities retained.
The original course lane improves **223/229 → 225/229**, with those same four
failures. The additional 31 course regressions all execute and pass. No
fixtures, references, C++ sources, protected checks, Ralph state or acceptance
policy changed. Optimizer, code generator, register allocator and backend
lifetimes are unchanged. This frontend group makes no PA34 object-parity claim.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-special-members-20261013/`.
All generated sources, dumps, diagnostic attempts, receipts, logs and timings
remain outside the committed checkout. `baseline.log` and
`baseline-manifest.json` retain the exact entry coverage; `preservation.json`
retains the complete identity comparison. Raw per-case protected traces and
mismatch files remain under `../clone/cases/pa7/<case-hash>/`.

### Frozen owners and typed responsibilities

Consulted `/home/vishvananda/work/references/v5rust-cpp/dev/src/sema/` directly:
`analyzer_declarations.cpp::{special_member,declare_function,redeclare_function,
default_definition,declare_variable}`, `analyzer_initialization.cpp::{
implicit_constructor,default_initialize,check_const_default}`,
`analyzer_construction.cpp::{implicit_default_facts,constructor_plan,
member_plans,member_plan,mem_initializer_target,declare_destructor,
check_destruction,check_subobject_destruction}`, `analyzer_bodies.cpp::
function_body` and `semantics_dump.cpp::{declaration,constructor_action,
function_definition,schedule,trailing}`. `source-map.tsv` records these
partial translations and their remaining dependent owners.

- `special_members.rs` uses the existing canonical `MemberKey`/function
  identities in semantic mode. The `SpecialDefinition` enum retains written
  `BodyId`, deletion, and first-versus-later defaulted state. Special-member
  defaults use the existing typed expression/visibility and class-completion
  queue. Default-argument diagnostics include defaults retained from an earlier
  declaration. First-defaulted constructors defer scalar planning until use;
  later defaulted definitions plan immediately and reject deletion there.
  The frozen implementation permits repeated late default definitions; that
  behavior, original body retention and first-defaulted state are preserved.
- `graph.rs` indexes constructors by canonical signature and retains only
  their own class's source-ordered bindings for zero-argument selection. A
  compatible special redeclaration inherits the original binding's access and
  visibility ordinal. Constructor labels never enter ordinary name lookup.
  Default selection inspects that named set, including recorded parameter
  defaults, prefers callable non-templates, diagnoses ambiguity/absence and
  memoizes the selected compact `ConstructorId`. Template deduction remains
  an explicit incomplete dependency when no ordinary constructor qualifies.
- `construction.rs` owns private unit constructor records and original
  `(member EntityId, Initializer)` plans. Each written body plans scalar
  members in declaration order; a mem-initializer takes precedence over the
  member default. Name lookup follows class then function context. Duplicate,
  invalid target, reference/const omission, narrowing, conversion and union
  designation checks use the frozen ordering and diagnostics. A temporary
  hash map indexes the written designations during this body only. The plan
  retains conversions on original `ExprId`s without synthesized syntax or
  reparsing. An earlier selected declaration and its later definition share
  the same constructor record ID; a later plan updates that record once.
- `analyzer.rs` uses the existing deferred-body queue, function scope and
  ephemeral `BodyContext`. Constructor plans run before the original compound
  statement with parameters visible; inline-body dependencies stay with that
  function, while ordinary definitions publish root demand. Scalar destructor
  bodies/defaulted definitions check their supported subobjects. At class
  completion, the typed `DestructorSpecification` enum selects the omitted
  scalar destructor exception fact; explicit specifications retain their
  existing owner. Variable lifetime checks diagnose deleted/inaccessible
  destruction and count nontrivial automatic initialization exactly once.
  The frozen `check_destruction` does **not** demand a destructor definition
  in this semantic view; that distinction is preserved.
- `semantics_dump.rs` reads original special-member declarations and recorded
  body facts. Defaulted definitions have an empty compound view; written
  constructor/destructor bodies retain the additional enclosing compound.
  Explicit constructor actions omit the synthesized implicit-constructor call.
  Demanded written bodies versus bodyless defaulted declarations use the
  existing member queue and identity deduplication. Standalone local class
  definitions omit the frozen printer's absent empty simple-declaration line.
  The printer performs no name lookup, typing or semantic planning.

Architecture review: one parsed AST, immutable canonical types and dense
expression facts; typed class-local callable indexes and constructor records;
completion events rather than whole-unit retries. Member planning is linear
in that class's members/designations, with average constant-time typed lookup.
Repeated zero-argument selection uses the cached identity. Undemanded first-
defaulted constructor plans stay absent. Scope scratch dies with each body;
original initializers retain only IDs/ranges into the unit-owned syntax. The
new ownership regression proves selected origin stability across a later
out-of-class definition, repeated object reuse, member-order plans over written
reverse order, original member-default expressions and retained scalar operand
conversions, with an unrelated deleted-on-use constructor left undemanded.
No dependency, unsafe code, FFI, environment read, subprocess fallback, blanket
lint suppression or new lint expectation was added. Two stale syntax-location
expectations were removed because semantic planning now consumes those fields.

### Validation evidence

`validate.py` and `checks.json` retain terminal exits, commands, binary/log hashes
and elapsed wall time. Candidate cppgm SHA-256:
`092791d4ee674f01050613bb38b6ad79a7b9d885b3242f56e10e67221033e3fb`. Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.
The independent identity comparison proves both full fixed coverage and zero lost passes. Course failure markers are retained and are not
reported as successful validation.

| Command | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | 194/198, four failures; exit 1 | `stage-required.log`, `protected-stage-result.json`, `fixed-manifest.json` |
| `make test-pa7` | Same fixed identities/counts; make exit 2 | `stage.log` |
| `make course-pa7` | 225/229, same four failures; marker retained; make exit 2 | `course.log`, `course-result.json`; `../clone/course/pa7/test/trace.jsonl` |
| `python3 .../clone_check.py prior pa7` | PASS, every PA1–PA6 fixed and course lane | `prior.log`, `prior-summary.json` |
| `python3 .../clone_check.py preflight pa7` | PASS | `preflight.log` |
| `python3 .../clone_check.py preserve pa7` | PASS; independent 192-identity preservation also passes | `preserve.log`, `preservation.json` |
| `make lint` / protected `clone_quality.py` | PASS, rustfmt, strict all-feature Clippy, 176 debug/release library tests each and file audit | `lint.log` |
| `make audit-rust` | PASS, 129 files, zero findings | `audit.log` |
| `python3 .../pa7-special-members-20261013/probe.py` | 90/90 exact protected matches | `probe.log`, `probe-summary.json`, `probes.jsonl`, sources |
| `python3 .../pa7-special-members-20261013/measure.py` | Exact supported scaling and qualified native receipts | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

The probes cover first/later default definitions, declarations alone,
zero-argument defaults/ambiguity, private access including later definitions,
deleted functions, const/reference default deletion, explicit/member-default
initialization, scalar conversion/narrowing and reference binding, missing/
static/function/type targets, duplicate designations, unions, local classes,
parameter shadowing, function-try blocks, nested demand and repeat selection,
scalar destruction/access/deletion/exception compatibility, and switch bypass
counts. They supplement the unchanged fixed and course suites. Class-subobject,
anonymous-inner, delegation, copy/move and template construction remain
explicit status-86 dependencies, rather than fabricated successful rejections.

The existing Ralph stage-progress receipt incorrectly treats the fixed lane's
198 results as a 229-case run and reports 31 unrun cases. The required fixed
command executes all 198 manifest entries; `make course-pa7` executes all 229
course identities. `harness-counts.json` retains the reproducer commands and
both complete receipts alongside the unchanged external failure log. No
checker policy or Ralph state was edited. Exact fixed reduction and all course
outcomes are supplied for checkpoint review; external advancement remains
Ralph's decision.

### Resource measurements and protected stack qualification

Warm observations run after the test jobs finish, with A/A calibration then
paired ABBA/BAAB blocks. `measurements.json` retains source/binary hashes,
output digests, every sample and peak RSS. All output hashes agree and all 18
protected workload receipts match. Workloads scale distinct first-defaulted
classes with initialized scalar members, repeated zero-argument selection from
one class, and reverse-written scalar mem-initializers in one constructor.
Measurements include process startup, parsing, analysis and semantic output;
no cold-cache or hardware-counter claim is made. PA35 generated-code
improvement targets do not apply to this exact frontend translation.

| Workload / count | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| defaulted-classes / 128 | 6.05 / 6490 | 20.40 / 8362 |
| shared-selection / 128 | 5.16 / 5806 | 16.50 / 8368 |
| defaulted-classes / 512 | 10.48 / 8622 | 27.81 / 8368 |
| shared-selection / 512 | 5.84 / 6498 | 20.08 / 8378 |
| defaulted-classes / 2048 | 29.37 / 15650 | 64.56 / 14930 |
| shared-selection / 2048 | 9.89 / 7508 | 28.39 / 8364 |
| scalar-members / 16 | 4.48 / 5818 | 15.52 / 8354 |
| scalar-members / 64 | 4.72 / 5778 | 17.00 / 8368 |
| scalar-members / 256 | 5.25 / 6450 | 18.13 / 8376 |

Native probes fix 1,000 nested ordinary calls, 1,500 unary operators and 1,500
parentheses inside a scalar mem-initializer, starting at a 1 MiB stack. All
three protected receipts have `match: true` and `stack_qualification.passed:
true`; both tools succeed identically at a shared 16 MiB retry. Raw initial
attempts/retries remain in `stack-qualifications/<receipt-id>/`. Only the
protected receipt establishes qualification; the genuine byte-budget guard
is unchanged. No native exhaustion depth, frame size or diagnostic position
was calibrated. Shared storage measured 28.52 GB used under the 107.37 GB
capacity ceiling.

### Exact newly passing fixed identities

```text
tests/general/100-private-nested-out-of-class-constructor-definition.t
tests/general/300-most-vexing-local-function-member-call-bad.t
```

### Remaining identities and next dependency boundary

The coherent group completes scalar special-member definitions, default
constructor selection/plans, destruction checks and their semantic views.
The next construction boundary is typed class-subobject/anonymous-inner plans,
copy/move selection and prvalue copy elision, delegation and implicit-object
expressions. These need the frozen class-construction/conversion owners;
scalar default plans cannot substitute for them. Inheritance must retain typed
base paths and conversion/access facts. Template target deduction remains a
separate owner, consuming retained parsed patterns rather than replaying tokens.
Constexpr object interpretation and virtual destruction also remain dependent
work. A partial handoff is justified by the completed production group and
exact preserved reduction; the stage has not passed or advanced.

```text
tests/general/100-qualified-base-data-member-hides-function.t
tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t
tests/general/300-elaborated-local-struct-copy-init.t
tests/general/300-static-cast-overloaded-function-template-argument.t
```

## Class-function identity, addresses and demand checkpoint — 2026-10-13

Coherent partial handoff for the `pa7 full-stage` implementation phase; PA7
remains incomplete. The preceding goal turn is classified as progress:
`4663046` translated explicit data members and reduced fixed failures from
14 to 12. This entry revalidated the clean checkout, read all owning guides
and consulted the frozen C++ owners before rerunning the full turn-start
manifest. Stage base and last-audited commit above are unchanged; implementation
does not advance the stage or certify the remaining groups.

The fixed manifest improves **186/198 → 192/198**, **12 → 6 failures**,
with identical coverage and all 186 prior passing identities preserved.
The original course lane improves **217/229 → 223/229**, with the same six
failures. All 31 additional existing course regressions execute and pass.
No fixture, reference, oracle, protected checker, Ralph state or acceptance
policy changed. Optimizer, register allocator, lowering, emission and backend
lifetimes are preserved; this frontend group does not claim PA34 object parity.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-member-functions-20261013/`.
Generated inputs, outputs, raw attempts, receipts and measurements remain
outside the committed checkout. The fixed turn-start log and identity snapshot
are `baseline.log` and `baseline-manifest.json`; the preceding final course
receipt is retained as `baseline-course-result.json`.

### Frozen owners and typed responsibilities

Consulted `/home/vishvananda/work/references/v5rust-cpp/dev/src/sema/` directly:
`analyzer_declarations.cpp::{declare_function,redeclare_function,
check_member_overload,signature_of,define_function,require_definition_types}`,
`analyzer_expressions.cpp::{binding_expression,address_of,select_overloaded,
resolve_overloaded,odr_use}`, `analyzer_calls.cpp::resolved_call`,
`analyzer_initialization.cpp::{record_default_arguments,type_default_arguments}`,
`analyzer_classes.cpp::finish_class_nesting`, `analyzer_bodies.cpp`'s body/demand
context, and `semantics_dump.cpp::{function_type,function_definition,id_node,
schedule,trailing}`. `source-map.tsv` records the partial translation and
remaining owners; no C++ implementation is invoked by Rust compiler code.

- `function_names.rs` and `declarations.rs` publish class functions through
  the existing typed callable factory, retaining canonical parameter/cv/ref
  signatures, first entity identity, static properties and source bindings.
  A new member overload checks static/nonstatic and mixed ref-qualification
  conflicts only within its named set. Existing signatures follow redeclaration
  rules, retaining effective static properties for out-of-class definitions
  and allocation functions. All matching named overloads participate in
  in-class redeclaration checks. By-value definition parameters/results must
  be complete at body analysis, after class completion.
- `function_resolution.rs` reads original qualified address operands once.
  Parenthesized designation remains distinct from `&C::f`. A single ordinary
  function resolves immediately; genuine overloads retain compact `OverloadId`
  slices for target-directed selection. Member/static kind, canonical function
  type and owner identity filter candidates; a matching non-template wins
  before template deduction. Inheritance paths and template specialization
  deduction remain explicit dependencies. Selected access/deleted diagnostics
  preserve the frozen name versus address location. The original operand keeps
  its selected function `TypeId`; the unary address keeps its distinct immutable
  member-pointer `TypeId`. Argument ranking and return/assignment/cast contexts
  consume those facts without another lookup.
- `expression_facts.rs` retains the address context `ScopeId` and selected
  canonical function origin on the original `ExprId`. The dump reads original
  name spelling, including aliases, and computes its function presentation
  from recorded ownership/cv facts. Member-pointer function types keep their
  cv/ref suffix; function designations/definitions show the implicit object
  pointer and `parameter this`. No source spelling is a semantic identity.
- `default_arguments.rs` records default expression IDs and introduction
  visibility once, then checks member clauses through the source-ordered
  `DeferredMember` class-completion queue alongside bodies. Complete-class
  lookup may find later members; namespace lookup stops at the class-completion
  event. Defaults have no enclosing function's implicit object. Nonstatic data
  members and calls reject exactly, while unevaluated members, enum/static
  defaults, lists and ordinary conversions retain their shared owners.
- `function_demands.rs` owns private typed demand lists keyed by function
  `EntityId` or `ConstructorId`. Repeated uses deduplicate while retaining first
  order and merging whether the function is called. `DefinitionEmission`
  separates ordinary root definitions from inline/constexpr demand. An
  ephemeral `BodyContext` carries the current function entity; inline/constexpr
  bodies retain their own dependencies, while ordinary out-of-class definitions
  publish root uses. Scalar default construction in member bodies records the
  same typed plans as ordinary declarations. No unrelated body is instantiated.
- The explicit semantic view closes demand with separate implicit/member queues
  and identity deduplication in C++ order. Cycles terminate; address-only uses
  of undefined members do not create declarations, while calls may. New demand
  encountered while printing a member joins the appropriate next queue round.
  Printing performs no analysis, name lookup or reparsing. `sizeof`/`alignof`/
  `decltype` selection stays under the unevaluated boundary, preventing output
  demand from query-only addresses.

Architecture review: one parsed AST, canonical immutable types, original dense
expression facts and per-entity source/demand records. Lookup visits the named
class/overload set and required lexical prefixes, never unrelated declarations.
Deferred defaults/bodies run on outermost class completion, not unit polling.
Demand collection is keyed and deduplicated; demand closure is linear in the
reachable edge set. Body context and overload scratch retain their existing
lifetimes. The ownership regression proves const overload selection, shared
origin on repeated addresses, original operand/address type separation,
query-only exclusion, constructor closure from only the selected member, and
undemanded owner layout/default construction. Rustfmt, strict Clippy and the
companion file audit pass with no new lint expectations, blanket suppression,
dependency, unsafe code, FFI, environment read or subprocess compiler fallback.

### Validation evidence

Candidate cppgm SHA-256:
`7454c3a5f934d8386bf6cd745aa79957dd4db1a1897a3363efd0f1d61d962198`.
Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

`checks.json` retains terminal command exits and binary hashes.
`preservation.json` proves the exact fixed coverage, failure reduction and
zero lost passing identities. `protected-stage-result.json` retains every
case receipt; raw diagnostics/outputs remain under
`../clone/cases/pa7/<case-hash>/trace.jsonl` and its mismatch directory.
The course receipt retains its failure marker; process exit alone is not
reported as successful course validation.

| Command | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | 192/198, six failures; exit 1 | `stage-required.log`, `protected-stage-result.json`, `fixed-manifest.json` |
| `make test-pa7` | Same fixed identities and counts; make exit 2 | `stage.log` |
| `make course-pa7` | 223/229, same six failures; marker retained; make exit 2 | `course.log`, `course-result.json`; `../clone/course/pa7/test/trace.jsonl` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | PASS, PA1–PA6 fixed and course lanes | `prior.log`, `prior-summary.json`, `prior-course-results.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | PASS | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | PASS; independent preservation of all 186 turn-start passes | `preserve.log`, `preservation.json` |
| `make lint` / protected `clone_quality.py` | PASS, rustfmt, strict all-feature Clippy, debug/release workspace tests (175 library tests each), file audit | `lint.log` |
| `make audit-rust` | PASS, 129 implementation files, zero findings | `audit.log` |
| `python3 .../pa7-member-functions-20261013/probe.py` | 100/100 exact protected matches | `probe.log`, `probe-summary.json`, `probes.jsonl`, sources |
| `python3 .../pa7-member-functions-20261013/measure.py` | Supported scaling and protected native receipts | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl`, qualification attempts |

The probes cover cv/ref overloads, pointer-to-cv return types, single versus
true overloaded addresses, static/nonstatic kind, owner/type mismatch,
reference/assignment/return/argument/cast use, aliases and original name
spelling, deleted/private selection, duplicate and return-only declarations,
static/ref conflicts, canonical parameter normalization, out-of-class and
private nested definitions, complete-class defaults, incomplete definition
types, constexpr emission, root versus function-owned demand, deduplication,
cycles, queue rounds and unevaluated addresses. These supplement the unchanged
fixed and course suites; they do not replace either lane.

The existing `last-stageProgress.log` counts the fixed gate against 229 course
cases and reports 31 cases unrun. The protected manifest has 198 entries;
`stage pa7` executes all 198, while `course pa7` executes all 229. The mismatch
in that progress report is retained in `harness-counts.json`, reproducible with
the required stage command and `make course-pa7`. The protected policy and
Ralph state remain unchanged. Both complete lane counts and exact fixed
identity preservation are supplied for Ralph's checkpoint review; no external
stage-advancement claim is made.

### Measured costs and stack qualification

Warm paired measurements run after test jobs finish, with A/A calibration,
ABBA/BAAB blocks, every timing/RSS sample, source/binary hash and output digest
retained in `measurements.json`. All paired output hashes agree and all 18
protected workload receipts match. Workloads independently scale distinct
classes with two cv overloads, repeated selection from one shared class, and
a reachable chain of inline member-address demands. The shared class emits
one selected definition despite repeated uses; the demand chain follows only
its reachable edges. Observations include process startup and dump I/O, not
cold-cache or hardware-counter claims. PA35 generated-code improvements are
outside this group's requirements.

| Workload / count | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| classes / 128 | 7.10 / 6466 | 21.87 / 8368 |
| shared-overloads / 128 | 5.01 / 6514 | 17.00 / 8378 |
| demand-chain / 128 | 6.34 / 6490 | 19.28 / 7988 |
| classes / 512 | 15.75 / 9294 | 39.61 / 8734 |
| shared-overloads / 512 | 8.72 / 6522 | 22.94 / 8000 |
| demand-chain / 512 | 10.39 / 7900 | 27.95 / 7996 |
| classes / 2048 | 46.57 / 17734 | 94.34 / 17736 |
| shared-overloads / 2048 | 15.28 / 8598 | 37.78 / 7996 |
| demand-chain / 2048 | 26.25 / 13554 | 64.01 / 12120 |

Native probes fix 1,000 nested ordinary calls around a member-function address,
1,500 unary operators over that address, and 1,500 enclosing parentheses,
starting at a 1 MiB stack. All three protected receipts have `match: true`
and `stack_qualification.passed: true`: both tools succeed with identical
streams, deterministic counters and generated files at a shared 16 MiB
retry. Raw original attempts and retries remain under `stack-qualifications/`.
Judge the protected receipt, not the adapter's retained original exit or
exhaustion position. The genuine byte-budget guard is unchanged; no native
cutoff, physical frame size or diagnostic position was calibrated. Shared
storage was about 28.50 GB used under the 107.37 GB capacity ceiling.

### Exact newly passing fixed identities

```text
tests/general/100-private-nested-out-of-class-member-definition.t
tests/general/300-default-argument-nonstatic-member-bad.t
tests/general/300-deferred-demand-closure.t
tests/general/300-member-cv-overload-identity.t
tests/general/300-member-function-pointer-return-pointer-const.t
tests/general/300-static-cast-member-overload-prefers-nontemplate.t
```

### Remaining identities and next dependency boundary

The coherent group completes ordinary class-function identity, qualified
function-address selection and demand closure for scalar default-construction
plans. Bound member designation/calls and implicit object expressions remain
explicit status-86 dependencies. The next fixed groups are special-member
semantic actions (two), inheritance formation/conversions (two), class copying
(one), and template target deduction (one), as recorded in
`failure-clusters.json`. Special-member declaration identity already exists
from PA6; semantic constructor actions need typed body/construction plans and
the frozen special-member presentation. Class subobjects, bit fields, friend/
inheritance access, operator calls, template specialization and constexpr
interpreter execution remain separate owners. A status-86 path is still a
failed case, even for the oracle's rejecting most-vexing declaration.

```text
tests/general/100-private-nested-out-of-class-constructor-definition.t
tests/general/100-qualified-base-data-member-hides-function.t
tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t
tests/general/300-elaborated-local-struct-copy-init.t
tests/general/300-most-vexing-local-function-member-call-bad.t
tests/general/300-static-cast-overloaded-function-template-argument.t
```

## Explicit data-member designation checkpoint — 2026-10-13

Coherent partial handoff for the `pa7 full-stage` implementation phase; PA7
remains incomplete. The previous goal turn is classified as progress:
`0808a91` translated anonymous storage and scalar default construction,
reducing fixed failures from 17 to 14. This entry revalidated the clean
checkout and reran every fixed case to establish the turn-start baseline.
Stage base and last-audited commit above are unchanged. Implementation work
does not certify or advance the stage.

This group reduces fixed failures **14 → 12**, **184/198 → 186/198**,
with identical coverage and all 184 previously passing identities preserved.
The original course lane improves **215/229 → 217/229**, with those same 12
failures. The 31 additional existing course regressions all execute and pass.
Both lanes completed; no unrun cases are being counted as passing. The two
newly passing fixed identities are:

```text
tests/general/100-distinct-anonymous-type-identities.t
tests/general/200-local-anonymous-union-variable.t
```

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-members-20261013/`.
All generated sources, outputs, traces, measurements and scripts remain
outside the commit. No oracle, fixture, reference, protected checker, Ralph
state or acceptance policy changed.

### Frozen owners and completed behavior

The full baseline's 14 failures cluster by earliest divergence into class
function/member declaration and demand (8), inheritance formation (2),
template specialization lookup (1), copy initialization (1), and explicit
member expression analysis (2). The last two now pass declarations and default
construction, then stop at the missing expression path. The protected traces
and raw output establish that boundary; the final 12 traces and diagnostics
are summarized in `failure-clusters.json`.

Consulted the frozen owners directly under
`/home/vishvananda/work/references/v5rust-cpp/dev/src/sema/`:
`analyzer_expressions.cpp::{member_access,member_qualifier,member_of_object,
binding_expression,explicit_conversion,static_constant}`,
`analyzer_access.cpp::{declare_in,member_access,redeclared_access,check_access}`,
`analyzer_classes.cpp::anonymous_union`,
`analyzer_declarations.cpp::{simple_declaration,check_variable_specifiers}`,
`analyzer_types.cpp::specifiers`, `analyzer_conversions.cpp::conditional`,
`semantic_graph.h::Binding`, `expression_facts.h`,
`constant_evaluator.cpp::{eval,DesignatesWithoutEffects}`,
`constant_expressions.cpp::{rvalue,member_object,glvalue}`,
`constant_statements.cpp::evaluable`, and
`semantics_dump.cpp::expression`. `source-map.tsv` records each partial
responsibility and its remaining boundary.

- `member_expressions.rs` analyzes the original base expression, preserves
  arrow's pointer conversion, reads an already-complete class scope and
  resolves the original member name through indexed qualified lookup.
  Dot on an lvalue gives an lvalue; dot on a prvalue/xvalue gives an xvalue;
  arrow gives an lvalue. Instance member cv joins object cv, with mutable
  removing only the object's const bit. Arrays qualify their elements;
  reference members retain their referent type and lvalue category. Static
  members retain their own type and lvalue category. Enumerators retain their
  typed folded value and literal presentation. Every explicit designation
  publishes a compact selected `BindingId` on its original `ExprId`.
- Own-class qualified and aliased names follow the frozen class-first,
  then contextual prefix lookup. Namespace prefixes remain typed `ScopeId`
  edges. The path checks incomplete/nonclass objects, invalid arrow operands,
  missing names, wrong qualifiers and non-value members at their original
  source locations. Binding access is checked before ordinary member typing.
- `graph.rs` owns a private dense access store indexed by `BindingId`.
  Access belongs to a declaration: anonymous injection preserves its shared
  `EntityId` but receives the enclosing class's declaration access; ordinary
  compatible redeclarations retain prior access. Each class scope retains
  typed formation access, initialized from its defining class-key and updated
  by access declarations. Direct public/private/protected checks use lexical
  class enclosure; inheritance and friend access remain separate dependencies.
- `specifiers.rs` and `data_members.rs` retain typed `Mutability`, validate
  mutable declarations against the frozen typedef/function/static/const/
  reference/array rules, and record member properties and source order.
  Diagnostic locations use the original root declarator for mutable checks.
  The owning data-member publication helper preserves existing initializer,
  layout-attribute and instance-storage behavior. No lint suppression or
  mechanical split was added.
- Named variable designation checks direct member access, diagnoses evaluated
  nonstatic members without an object, and retains the unevaluated form.
  Namespace/block anonymous storage retains its existing explicit edge.
  Implicit object designation inside a class remains a status-86 boundary.
- Same-class reference casts use the existing reference conversion rules
  before class-by-value construction demand. The cast's target and category
  remain typed presentation facts. The conditional owner now tests compatible
  same-class glvalues before class copying/conversion, in the frozen order.
- `constants.rs` reads flattened original-expression aliases before folded
  enumerator evaluation and checks supported object designations and their
  effects. Skipped conditional/logical operands do not evaluate their object.
  Named/member/comma/conditional/reference-cast/array forms use recorded facts;
  selected non-constexpr calls diagnose at their original call location.
  Semantic constexpr integral initialization requests required evaluation.
  Pointer places, constexpr function execution and general object state still
  report their explicit implementation boundaries.
- `semantics_dump.rs` renders explicit `member-expression` from its selected
  binding and original base child, preserving operator spelling and child
  order. Folded enumerators remain literals. Dump work is iterative and
  performs no lookup, expression analysis or text reparsing.

Ownership/architecture review: one parsed syntax tree and unit-owned graph;
private compact binding/access stores; immutable interned type identities;
dense original-AST semantic facts and separately retained operand conversions.
A member query visits only the named class and required qualifier/lexical
edges. No whole-program scan or unrelated class/member demand was introduced.
Member access does not demand layout, constructors or a function body.
Representation storage is flat and released with its translation unit;
query scratch dies with the query. The new ownership regression verifies
shared injected entity identity, distinct binding access, original immutable
member type versus use-site const type, retained designation and undemanded
layout/construction. Existing optimizer, register allocator, code generator
and per-function backend lifetimes are preserved. PA34 object/tool equality
remains a later proof.

### Remaining identities and next dependency boundary

Explicit ordinary data-member designation is complete for the supported
non-inherited, non-bit-field objects in this group, including related cv,
reference, static, qualification and value-category behavior. The next fixed
failure group needs class function declaration identity, implicit object
parameters and bound member designation/calls, plus function-owned demand
lists. Private nested qualified definitions and default arguments depend on
those contexts. Class copy/subobject construction, inheritance/access paths,
member-pointer designation, special member/arrow operators, bit fields and
template target deduction remain separate owning groups. Full constant object
places and constexpr call/frame execution are also pending; scalar folded
member reads do not stand in for that interpreter. Missing implementation
status 86 remains a failed case, including the two oracle rejections.

```text
tests/general/100-private-nested-out-of-class-constructor-definition.t
tests/general/100-private-nested-out-of-class-member-definition.t
tests/general/100-qualified-base-data-member-hides-function.t
tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t
tests/general/300-default-argument-nonstatic-member-bad.t
tests/general/300-deferred-demand-closure.t
tests/general/300-elaborated-local-struct-copy-init.t
tests/general/300-member-cv-overload-identity.t
tests/general/300-member-function-pointer-return-pointer-const.t
tests/general/300-most-vexing-local-function-member-call-bad.t
tests/general/300-static-cast-member-overload-prefers-nontemplate.t
tests/general/300-static-cast-overloaded-function-template-argument.t
```

### Validation and measured costs

Candidate cppgm SHA-256:
`7684577287db71831c50791f94c1c79cf7014ec927ed3cac468e8e0146031ce8`.
Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

`checks.json` retains the commands, terminal exits, binary hashes and artifact
paths. `baseline.log`/`baseline-manifest.json` and `stage-required.log`/
`fixed-manifest.json` retain exact identity sets; `preservation.json` proves
unchanged 198-case coverage, failure reduction and zero lost passing identities.
The required protected stage command and make wrapper both run the full fixed
manifest. The course result retains its failure marker rather than treating
its process exit alone as success.

| Command | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log`, `checks.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | 186/198, exactly 12 failures; exit 1 | `stage-required.log`, `protected-stage-result.json`, `fixed-manifest.json` |
| `make test-pa7` | Same 186/198; make exit 2 | `stage.log` |
| `make course-pa7` | 217/229, same 12 failures; marker retained; make exit 2 | `course.log`, `course-result.json`; `../clone/course/pa7/test/trace.jsonl` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | PASS, all PA1–PA6 fixed and course lanes | `prior.log`, `prior-summary.json`, `prior-course-results.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | PASS | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | PASS, 184/184 identities; independently verified against the full baseline | `preserve.log`, `preservation.json` |
| `make lint` | PASS, rustfmt, strict all-feature Clippy, debug/release workspace tests including 174 library tests in each profile, file audit | `lint.log` |
| `make audit-rust` | PASS, 128 implementation files, zero findings | `audit.log` |
| `python3 .../pa7-members-20261013/probe.py` | 74/74 protected matches | `probe.log`, `probe-summary.json`, `probes.jsonl`, sources |
| `python3 .../pa7-members-20261013/measure.py` | All paired output hashes and 18 protected workload receipts match; 3/3 native matches, 2 protected qualifications | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl`, `stack-qualifications/` |

Earlier fixed counts are PA1 58/58, PA2 26/26, PA3 20/20, PA4 106/106,
PA5 196/196 and PA6 121/121. Earlier course counts are 58/58, 28/28, 20/20,
112/112, 296/296 and 694/694, all with no failure marker.

Measurements run after test jobs finish, using A/A calibration and warm paired
ABBA/BAAB samples. Source/binary hashes, output hashes/lengths and every time/
RSS sample are retained. Observations include process startup and dump I/O.
Workloads independently scale distinct classes with arrow access, functions
sharing one class with const dot access, and direct members of one wide class.
The wide-class expression is an ordinary left binary spine; indexed member
lookup never scans all its other fields. No generated-code improvement or
C++/Rust runtime equality requirement is imposed.

| Workload / count | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| classes / 128 | 5.97 / 6468 | 19.23 / 8000 |
| shared-class / 128 | 5.10 / 6498 | 17.44 / 8752 |
| wide-class / 128 | 4.61 / 6492 | 16.55 / 7994 |
| classes / 512 | 12.23 / 8564 | 30.41 / 8794 |
| shared-class / 512 | 7.95 / 7222 | 24.62 / 8730 |
| wide-class / 512 | 6.67 / 6438 | 23.62 / 7990 |
| classes / 2048 | 34.81 / 16354 | 82.41 / 15636 |
| shared-class / 2048 | 19.82 / 10706 | 50.34 / 10014 |
| wide-class / 2048 | 28.87 / 8632 | 71.53 / 7996 |

Native probes start with a 1 MiB stack: 1,000 nested calls around dot access,
1,500 unary operators over arrow access, and a 600-link chain of pointer-valued
data members ending in a scalar member. All three protected receipts have `match: true`.
Calls and unary probes qualify with both tools succeeding with identical
streams, deterministic counters and files at the shared 16 MiB stack; the
member chain succeeds on the initial stack. Raw attempts and qualification
reruns are retained. The genuine byte-budget guard is unchanged; native
exhaustion depths and diagnostic positions were not calibrated. Shared
storage was 28.43 GB used within the 107.37 GB ceiling.

## Anonymous storage and scalar default-construction checkpoint — 2026-10-13

Coherent partial handoff for the `pa7 full-stage` implementation phase; PA7
remains incomplete. The preceding goal turn is classified as progress: commit
`8e3665e` translated member-pointer types/scalar semantics and reduced fixed
failures to 17. This entry revalidated the clean checkout and collected a full
turn-start baseline instead of using the interrupted 229-case report. The
protected fixed manifest contains 198 cases; the original course lane contains
229, including the 31 existing audit regressions. The fixed baseline was
rerun here; the clean-entry course result is retained in the preceding
checkpoint's `../pa7-member-pointer-20261013/course-result.json`. Both final
lanes completed here. The last-audited commit is unchanged; implementation
work does not advance the stage or certify the remaining class/template groups.

This group reduces fixed failures **17 → 14**, **181/198 → 184/198**,
with identical coverage and all 181 prior passing identities preserved. The
course lane improves **212/229 → 215/229**, with the same 14 remaining cases.
No fixture, reference, oracle, protected checker, Ralph state or acceptance
policy changed. The three newly passing fixed identities are:

```text
tests/general/100-namespace-static-anonymous-union-injected-members.t
tests/general/300-block-anonymous-union-injected-members.t
tests/general/300-reference-binding-pointee-const-pointer.t
```

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-anonymous-storage-20261013/`.

### Owners, earliest divergences and completed behavior

The turn-start traces split the 17 failures into inheritance formation (2),
class function/member declaration and demand (8), template specialization
lookup (1), class copy initialization (1), explicit member expressions (2),
and successful-output differences in anonymous storage (2) and ordinary class
default construction (1). The last three share the default-initialization
owners, rather than requiring member-call overload resolution. Raw diagnostics
and output paths are retained in the protected case traces; the final remaining
case receipts and diagnostic clustering are in `failure-clusters.json`.

Read the frozen owners directly under
`/home/vishvananda/work/references/v5rust-cpp/dev/src/sema/`:
`analyzer_classes.cpp::{anonymous_union,name_unnamed_type,class_body}`,
`analyzer_declarations.cpp::{simple_declaration,declare_variable,
earlier_variable,new_variable}`, `aggregates.h::MemberInitialized`,
`analyzer_initialization.cpp::{implicit_constructor,default_initialize,
check_const_default,name_local_type}`, `analyzer_construction.cpp::{
implicit_default_facts,member_plan}`, `analyzer_expressions.cpp::{
binding_expression,odr_use,check_not_deleted}`, `analyzer_bodies.cpp` demand
contexts, and `semantics_dump.cpp::{simple,variable,storage,id_node,
constructor_action,function_name,function_type,trailing}`.
`docs/port/source-map.tsv` records their Rust responsibilities and boundaries.

- `anonymous_types.rs` retains one typed hidden storage binding per anonymous
  union, outside ordinary lookup and print bindings. Dense original `DeclId`
  facts put its declaration at the source declaration point; injected members
  still share their original `EntityId`. Class-held hidden storage retains
  its separate owner and layout role. Namespace/block storage selects default
  construction only in semantic mode outside template patterns.
- `construction.rs` owns a private unit-local `ConstructorId` store and an
  ordered demand queue. Each requested scalar-subobject class moves through
  `NotStarted`, `InProgress`, `Ready(id)` and memoizes deletion, const-default
  eligibility, canonical object pointer, presentation signature and member
  initializer count. Const/reference fields without initializers delete a
  nonunion implicit constructor; union variants are not default-initialized.
  Const class/union objects use the frozen default-initialization diagnostics.
  Scalar const objects retain the first declaration's extern rule. No class
  layout or unrelated member body is demanded.
- Definition declarations record the selected constructor on the variable.
  Constructor demands are deduplicated in O(1), in first-use order. An array
  shows one element action and never demands the constructor definition or
  walks its bound. Ordinary automatic construction with member initializers
  updates the switch initialization count; anonymous hidden storage preserves
  the separate C++ path that does not update that count. Static/thread objects
  do not count. Existing initialized static data members retain their value.
- `data_members.rs` checks ordinary block-variable redeclarations: two extern
  declarations may share an entity; other repeated declarations reject before
  initialization. At class completion, it memoizes direct initialized-member
  counts, including initialized anonymous member objects, and rejects several
  default initializers in a union at the original class-key location.
  `ast.rs` removes the now-unfulfilled dead-code expectation on that location.
- Semantic anonymous classes from simple declarators receive checked stable
  `__local_type<N>` presentation ordinals. A typedef still supplies the
  constructor's own name; aliases retain their original token label. These
  labels never enter lookup or structural type identity. PA6 naming is intact.
- Dense original `ExprId` facts record an anonymous member's selected storage
  binding. The iterative dump renders member access over that storage using
  the existing conversion facts, with no second expression analysis or name
  lookup. Typed variable/constructor facts drive constructor actions, array
  actions and trailing scalar-only implicit definitions. Constructor calls
  omit unnamed-namespace qualifiers, while definitions/types retain them.

Ownership/architecture review: a single parsed tree and unit-owned graph;
compact typed storage edges and dense declaration/expression facts; canonical
identity stays O(1). Successful constructor facts are computed once per class,
class completion counts only that class's ordered members, and repeated
objects perform constant-time constructor/demand queries plus lexical-scope
checks. The demand queue visits only requested definitions. Storage and scratch
are flat and released with the unit; dump work is iterative. No semantic tree,
text transport, reparsing, whole-program scan, unsafe code, new dependency or
lint suppression was introduced. All new logic stays with its owning phase.
Optimizer, code generator, register allocator and backend lifetimes are intact;
this checkpoint makes no PA34 object/tool-equality claim.

The ownership regression checks hidden storage outside ordinary bindings,
shared constructor identity, array versus object demand, an unused class's
unstarted constructor state, and undemanded layout. Protected probes exercise
namespace/block/static storage, shadowing, qualification, arrays, pointer and
reference members, conversions/queries, empty unions, naming/typedef identity,
extern redeclarations, const/deleted constructors, scalar member initializers,
union completion and switch diagnostics. Their sources and raw receipts stay
outside the checkout.

### Exact remaining identities and next dependency boundary

The next coherent group needs explicit class data-member designation and
object/member facts: both unnamed-object tests now pass declaration/default
construction and stop at the missing member-expression phase. Member functions
need implicit object parameters, access, overload identity and function-owned
demand lists. Construction within member bodies returns status 86 at that
explicit boundary instead of incorrectly publishing the body's demands as unit
roots. Named/anonymous class subobject initialization, user-declared default
selection, explicit/copy initialization and temporary lifetimes remain pending.
Inheritance/access paths and template specialization/target deduction are
separate frozen owners. Missing implementation remains a failing case even
when the oracle rejects normally.

```text
tests/general/100-distinct-anonymous-type-identities.t
tests/general/100-private-nested-out-of-class-constructor-definition.t
tests/general/100-private-nested-out-of-class-member-definition.t
tests/general/100-qualified-base-data-member-hides-function.t
tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t
tests/general/200-local-anonymous-union-variable.t
tests/general/300-default-argument-nonstatic-member-bad.t
tests/general/300-deferred-demand-closure.t
tests/general/300-elaborated-local-struct-copy-init.t
tests/general/300-member-cv-overload-identity.t
tests/general/300-member-function-pointer-return-pointer-const.t
tests/general/300-most-vexing-local-function-member-call-bad.t
tests/general/300-static-cast-member-overload-prefers-nontemplate.t
tests/general/300-static-cast-overloaded-function-template-argument.t
```

### Validation and measured costs

Candidate cppgm SHA-256: `a8cd1536eb587d5ecada0e4063018c0503ba16c057c8b73a042abf8be62f9d9c`.
Frozen cppgm++ SHA-256: `9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

Final command results, binary hashes, manifest identity sets, course results,
measurements and protected stack receipts are retained under the artifact root.
The fixed-manifest identity comparison is `preservation.json`; raw before/after
logs are `baseline.log`, `stage-required.log` and `stage.log`. Commands/results:

| Command | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log`, `checks.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | 184/198, exactly 14 failures; exit 1 | `stage-required.log`, `fixed-manifest.json` |
| `make test-pa7` | Same 184/198; make exit 2 | `stage.log` |
| `make course-pa7` | 215/229, same 14 failures; course failure marker retained; make exit 2 | `course.log`, `course-result.json`; `../clone/course/pa7/test/trace.jsonl` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | PASS, all PA1–PA6 fixed and course lanes | `prior.log`, `prior-summary.json`, `prior-course-results.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | PASS | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | PASS, 181/181 prior identities, no coverage change | `preserve.log`, `preservation.json` |
| `make lint` | PASS, rustfmt, strict all-feature Clippy, debug/release tests and file audit | `lint.log` |
| `make audit-rust` | PASS, 127 implementation files, zero findings | `audit.log` |
| `python3 .../pa7-anonymous-storage-20261013/probe.py` | 67/67 protected matches | `probe.log`, `probe-summary.json`, `probes.jsonl`, sources |
| `python3 .../pa7-anonymous-storage-20261013/measure.py` | Matching paired outputs and all 18 protected workload receipts; both native probes qualify | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl`, `stack-qualifications/` |

Earlier fixed counts are PA1 58/58, PA2 26/26, PA3 20/20, PA4 106/106,
PA5 196/196, PA6 121/121. Earlier course counts are 58/58, 28/28, 20/20,
112/112, 296/296 and 694/694, all exit 0 with no comparison-failure markers.
Progress is the exact fixed failure reduction 17 → 14 and preservation of
181 passing identities, independent of the extra 31 existing course inputs.

Warm measurements run after all test jobs finish: A/A calibration, followed by
paired ABBA/BAAB observations. They include process startup and semantic-dump
I/O. Source/binary hashes, output hashes/lengths and every RSS/time observation
are recorded in `measurements.json`. Workloads scale independently declared
anonymous storage/constructors and repeated objects sharing one class's
constructor. Huge array bounds confirm that default construction performs no
per-element semantic work. No PA35 generated-code target or equality of runtime
costs is imposed.

| Workload / count | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| unions / 128 | 7.09 / 7172 | 20.67 / 8376 |
| shared-class / 128 | 6.02 / 6468 | 19.48 / 8728 |
| unions / 512 | 15.11 / 9328 | 39.50 / 8374 |
| shared-class / 512 | 7.76 / 7246 | 25.48 / 8000 |
| unions / 2048 | 46.41 / 17788 | 116.76 / 19102 |
| shared-class / 2048 | 22.88 / 10710 | 52.13 / 9238 |
| array-bound / 128 | 5.03 / 5800 | 18.84 / 8368 |
| array-bound / 4096 | 4.89 / 5774 | 16.88 / 8002 |
| array-bound / 100000000 | 4.81 / 5810 | 17.31 / 8372 |

Native probes start with 1 MiB stacks and use 1,000 nested ordinary calls or
1,500 unary operators over an injected anonymous member. Both protected
receipts have `match: true` and `stack_qualification.passed: true`; both tools
succeed with identical output at the shared 16 MiB stack. Raw attempts and
reruns are retained. The genuine byte-budget guard is unchanged; native
exhaustion depths and diagnostic positions were not calibrated. Shared storage
was 28.39 GB used within the 107.37 GB ceiling; all generated evidence is
outside the commit.

## Member-pointer scalar implementation checkpoint — 2026-10-13

Coherent partial handoff for the `pa7 full-stage` implementation phase. PA7
remains incomplete. This group reduces fixed-manifest failures from 19 to 17:
**179/198 → 181/198**, preserving every prior passing identity and all 198
covered identities. The preceding completed goal turn is classified as
progress: its independent audit repaired the implementation and left the
reviewed `a05b466` implementation recorded by clean documentation commit
`9d32c2a`. This turn re-read the actual checkout, fixed manifest, protected
traces and frozen source owners. The last-audited commit above is unchanged;
this implementation checkpoint does not advance the assignment or certify
class/member/template call semantics.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-member-pointer-20261013/`.
The two newly passing fixed identities are:

```text
tests/general/300-member-function-pointer-type-alias-and-function.t
tests/general/300-member-pointer-type-alias-and-function.t
```

### Failure clustering and frozen owners

The turn-start protected traces divide the 19 failures by earliest divergence:
member-pointer type formation (2); class inheritance formation (2); class
function/member declaration and demand (8); template specialization lookup (1);
class construction/copy initialization (1); unsupported extended/member
expressions (2); and successful-output differences in anonymous-union storage
(2) and class-construction presentation in a reference-binding case (1).
The exact trace diagnostics, rather than the
fixture names, determined the group. The two member-pointer aliases fail at
`analyzer_types.cpp::ptr_operator`, before any class body or member selection
is needed. Their ordinary function parameters and returns then exercise the
shared canonical type and scalar conversion owners.

Consulted frozen files directly under
`/home/vishvananda/work/references/v5rust-cpp/dev/src/sema/`:
`analyzer_types.cpp::{ptr_operator,declarator}`,
`type_table.cpp::{member_pointer,canonical_of}`, `semantic_types.h`,
`conversions.cpp::{SimilarLevels,reference,value_conversion,const_castable,
casts_away_constness}`, `analyzer_member_pointers.cpp::{member_pointer_cast,
static_member_pointer_cast,composite_pointer,cv_combined}`,
`analyzer_expressions.cpp::{explicit_conversion,typed,size_of}`,
`analyzer_calls.cpp::associated`, `layout.cpp::type_size`,
`semantics_dump.cpp::{AppendType,expression}` and
`analyzer_declarations.cpp::{check_parameter_redeclaration,declare_variable}`.
`docs/port/source-map.tsv` records each translated responsibility and the
remaining owner boundaries.

### Completed behavior and typed flow

- `declarators.rs` consumes the original ptr-operator and Prefix lookup.
  It strips owner cv, checks class/dependent-owner identity, rejects void or
  reference members at the root declarator location, and publishes a qualified
  `MemberPointer { owner: TypeId, element: TypeId }`. An incomplete owner is
  sufficient; no class body, layout or constructor is demanded.
- `types.rs` interns both edges as immutable structural identity and walks
  both during iterative canonical DAG completion. Canonical member functions
  adjust by-value parameters and retain their independent function cv/ref
  qualifiers. Aliased owners share identity; different owners, function cv
  and function ref qualifiers remain distinct. `pointer_level` is a borrowed
  typed view of an ordinary/member pointer shell, not a rendered type key.
- `scalar_conversions.rs` uses the shared shell view for mixed-level
  qualification, canonical owner matching, intermediate const checks,
  array/function member shape, null/zero and bool conversions, and CWG 2352
  reference qualification. Existing candidate ranking consumes those same
  conversions; unsuccessful candidates still return compact optional facts.
- `scalar_operators.rs` accepts member-pointer equality and null composites,
  rejects relational/arithmetic/dereference/inc/dec operations, and combines
  comparison cv levels with const above differing levels. Conditional member
  pointers require conversion to one operand's qualified type, as the frozen
  owner does. The iterative cv-combination scratch is O(pointer depth), dies
  with the query, and preserves ordinary-pointer behavior.
- `scalar_casts.rs` retains typed owner/member shape for const casts and data
  versus function category for reinterpret casts. Static member-pointer casts
  record the selected conversion and the frozen value-retyping presentation.
  Array-member const casts use the same array-shape check. Member designation,
  bound member calls and inheritance adjustments remain separate dependencies.
- `queries.rs` implements the Itanium 8/8 data-member and 16/8 function-member
  size/alignment rules without completing the owner. Semantic `alignof` checks
  its typed unevaluated operand, strips reference/array shells, and renders as
  `sizeof-expression`. Dense original-ExprId `query_value` facts preserve size
  and alignment for constant reads without another operand analysis or
  changing source presentation. `call_lookup.rs` follows both member-pointer
  type edges and associates an incomplete class through its recorded home.
- A related parenthesized declaration probe exposed a diagnostic-location
  gap: `M(p);` is a declaration, and its forbidden parameter redeclaration
  diagnoses at the original root `(`, rather than the nested identifier.
  `ValueScope` carries that source location separately; lookup, declaration
  identity and other name diagnostics continue to use their own locations.

Ownership review: one parsed syntax tree and unit-owned immutable type store;
original AST IDs carry dense semantic facts. Canonical identity comparisons
remain constant-time after memoized completion. All changed algorithms stay
in their existing semantic owners, with no mechanical module split, text
transport, environment read, external implementation, unsafe code, new
optimizer transformation or whole-program scan. Two unit regressions verify
owner/function identity and query facts while the owner stays incomplete.
The 62 protected success/rejection probes cover the production invariants,
including aliases, redeclarations, nulls, casts, mixed levels, array members,
ADL and queries; they retain raw receipts and reproducible sources outside the
checkout. Frozen fixtures, references, checker policy and Ralph state remain
unchanged.

### Next dependency boundary and exact remaining identities

Next work needs class member/function designation, implicit object parameters,
construction/storage facts and demand before member addresses or calls can
be resolved. Inheritance formation/access paths, template specialization and
member-target deduction, deferred demand closure and interpretation remain
separate C++ owners. This checkpoint makes no PA34 object/tool-equality claim;
code generation, optimization and register allocation are unchanged.

```text
tests/general/100-distinct-anonymous-type-identities.t
tests/general/100-namespace-static-anonymous-union-injected-members.t
tests/general/100-private-nested-out-of-class-constructor-definition.t
tests/general/100-private-nested-out-of-class-member-definition.t
tests/general/100-qualified-base-data-member-hides-function.t
tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t
tests/general/200-local-anonymous-union-variable.t
tests/general/300-block-anonymous-union-injected-members.t
tests/general/300-default-argument-nonstatic-member-bad.t
tests/general/300-deferred-demand-closure.t
tests/general/300-elaborated-local-struct-copy-init.t
tests/general/300-member-cv-overload-identity.t
tests/general/300-member-function-pointer-return-pointer-const.t
tests/general/300-most-vexing-local-function-member-call-bad.t
tests/general/300-reference-binding-pointee-const-pointer.t
tests/general/300-static-cast-member-overload-prefers-nontemplate.t
tests/general/300-static-cast-overloaded-function-template-argument.t
```

### Final validation evidence

Candidate cppgm SHA-256:
`3204e6b79b74513910a464a6c20ef612e3f110df73523fd2e85d980013cd5a16`.
Frozen cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.
All following paths are relative to this checkpoint's artifact root.

| Command / check | Result | Evidence |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | 181/198; exit 1, exactly 17 unfinished cases | `stage-required.log`, `fixed-manifest.json`, `checks.json` |
| `make test-pa7` | Same 181/198; make exit 2 | `stage.log`, `checks.json` |
| `make course-pa7` | 212/229, same 17 unfinished cases; course failure marker retained, make exit 2 | `course.log`, `course-result.json`; `../clone/course/pa7/test/trace.jsonl` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | PASS, all PA1–PA6 fixed differential and original course lanes | `prior.log`, `prior-summary.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | PASS | `preflight.log`, `checks.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | PASS, 179/179 historical identities; independent final-manifest comparison has zero lost passes and identical coverage | `preserve.log`, `turn-start-manifest.json`, `turn-start.log`, `preservation.json` |
| `make lint` (`python3 /home/vishvananda/work/v5rust-control/clone_quality.py`) | PASS, rustfmt, strict all-feature Clippy, debug/release tests and file audit | `lint.log` |
| `make audit-rust` | PASS, 126 files, zero findings | `audit.log`, `checks.json` |
| `python3 .../pa7-member-pointer-20261013/probe.py` | 62/62 protected byte/diagnostic matches | `probe-summary.json`, `probes.jsonl`, probe sources |
| `python3 .../pa7-member-pointer-20261013/measure.py` | All paired outputs and 12 protected workload receipts match; two native stack receipts qualify | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl`, `stack-qualifications/` |

The 527 earlier fixed cases pass (PA1 58/58, PA2 26/26, PA3 20/20,
PA4 106/106, PA5 196/196, PA6 121/121). Original earlier course counts are
58/58, 28/28, 20/20, 112/112, 296/296 and 694/694; all have exit 0 with
no failed-comparison markers. Stage progress is demonstrated by exact identity
sets: 198 inputs before/after, 179 prior passes retained, two new fixed passes,
and failures reduced 19 → 17. No added passing fixture substituted for a
fixed-manifest failure reduction. Full PA7 gates correctly remain failing.

### Measured resource costs and stack qualification

Warm measurements used final binaries after test jobs ended: A/A calibration,
then ABBA/BAAB paired blocks. Raw timings/RSS, source/binary SHA-256 and output
hashes/lengths are retained. Inputs scale independent ordinary functions
making calls over overloads with distinct member-pointer owner identities.
Numbers include process startup and semantic-dump I/O; no PA35 generated-code
improvement or runtime equality requirement is imposed.

| Functions / overloads | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| 128 / 4 | 5.84 / 6496 | 19.40 / 8776 |
| 512 / 4 | 10.39 / 7178 | 30.52 / 8356 |
| 2048 / 4 | 24.60 / 10696 | 65.53 / 10656 |
| 128 / 16 | 6.90 / 6480 | 20.39 / 8002 |
| 128 / 64 | 7.61 / 6446 | 24.08 / 8008 |
| 128 / 256 | 13.31 / 7200 | 30.13 / 8376 |

Native probes retain 1,000 nested ordinary calls carrying a member pointer
and 1,500 unary operators starting with a member-pointer operand, both starting
at 1 MiB. Both protected receipts have `match: true` and
`stack_qualification.passed: true`; both tools succeed with identical output
at a shared 16 MiB stack. Judge these receipts rather than the adapter's
retained original exit, exhaustion depth or diagnostic position. Raw attempts
and reruns are retained. The genuine byte-budget guard is unchanged; native
cutoffs and frame sizes were not calibrated. Shared storage was 28.38 GB used
within the 107.37 GB ceiling. Generated outputs and evidence remain outside
the commit.

## Independent checkpoint audit — 2026-10-13

Certified scope: the completed scalar/procedural, ordinary-call and nonclass
list/array initialization groups in all three accepted checkpoints
`e72f1d8`, `37c4358` and `f50f537`, reviewed from stage base `9a851dc`.
The previous goal work is classified as progress: these commits changed the
implementation and reduced the fixed-manifest failures. This audit reviewed
and repaired their combined implementation; it does not certify future
class/member/template/interpreter behavior or advance PA7.

Reviewed implementation commit: `a05b466c47eac23ca20be5d23d5bed00183e72b8`. The following documentation
commit records the audit of that implementation. The fixed manifest remains
**179/198**, with exactly the same **19** failing identities listed under
“Exact remaining fixed identities” below. All **179** accepted passing
identities and all **198** covered identities are preserved. Thirty-one required
regressions were added under `pa7/tests/general/890-audit-*.t`; each passes
both the protected exact comparator and its original course comparison.
The course lane now reports **210/229**; its overall failure marker still
represents the same unfinished 19 cases. Added passes did not compensate for
any additional fixed-manifest failure.

### Findings and repairs

| Finding in completed scope | Repair and frozen owner |
| --- | --- |
| Unscoped-enum operands were confused with arithmetic destination types, permitting integer/distinct-enum initialization and enum mutation | `scalar_conversions.rs` distinguishes destination arithmetic from operand arithmetic; `scalar_operators.rs` rejects enum increment/compound assignment. `conversions.cpp::value_conversion`, `analyzer_expressions.cpp::increment/assignment` |
| Fixed-underlying-enum promotion omitted the underlying type as an additional promotion target | Retain both underlying-type and ordinary promoted-type ranks, including the oracle's ambiguous-overload result. `conversions.cpp::IsIntegralPromotion` |
| Nonconst/volatile lvalue references accepted xvalues | Direct xvalue binding now requires an rvalue reference or exactly const lvalue reference; failed candidates remain compact rejections. `conversions.cpp::reference` |
| Pointer-to-void sequences lost their qualification adjustment, including array element cv | Record the adjustment for overload ordering and use effective array cv in implicit conversions/cast constness. `conversions.cpp::value_conversion/CvOf/casts_away_constness` |
| Conditional glvalues and pointer composites stripped array element cv too deeply | Compare shallow array identity before qualified pointer alternatives; retain the frozen array/void conditional rejection. `analyzer_conversions.cpp::conditional`, `analyzer_member_pointers.cpp::composite_pointer` |
| `bool += object_pointer` was omitted | Translate the owner's compound-assignment branch and complete-pointee check. `analyzer_expressions.cpp::assignment` |
| Target-directed overload casts had incorrect diagnostic bytes/location and missed reference-to-pointer targets | Strip reference then pointer; diagnose at the cast with the frozen spelling. `analyzer_expressions.cpp::explicit_conversion/select_overloaded` |
| Function designation facts were recursively overwritten through nested unary nodes | Use the lookup representative initially; publish the selected designation and immediate unary operand only, retaining deeper independently typed facts. `analyzer_expressions.cpp::resolve_overloaded` |
| Retyped reference-cast presentation incorrectly changed call category and selected function-id type | The view reads the call's category/declared result and selected function's canonical type as the frozen printer does. `semantics_dump.cpp::call/id_node` |
| A repeated default label after a local initializer reported the bypass error before the duplicate error | Check duplicate-default state first, retaining exact frozen diagnostic precedence. `analyzer_bodies.cpp::statement`, `procedural_statements.rs::semantic_jump` |
| Winning overloaded arguments were selected again; semantic constants rederived published name/type facts; scratch retained completed-query contents | Carry the selected `BindingId` with each candidate conversion; publish without a second selection. Reuse original-AST binding/type facts in semantic constant reads/casts. Clear candidate/sequence contents on success and failure, retaining capacity. `procedural_calls.rs`, `constants.rs`, `declarators.rs` |

The independent pre-repair probes reproduce 14 mismatches across the first
39 cases; supplementary array probes retain three further mismatches. A final failure-path
probe reproduced duplicate-default diagnostic precedence before its repair
(`before-default-probes.jsonl`).
`before-probes.jsonl`, `before-probe-summary.json` and
`before-array-probes.jsonl` retain their exact diagnostics and output bytes.
After repair, **55/55** expanded audit probes and **78/78** initialization
probes match exactly. Course regressions also cover repaired diagnostics,
reference and pointer rankings, enum mutation/promotions, array qualifier
retention, bool compound assignment and nested function designation output.
The storage-invariant test additionally checks qualification flags/ranking
without overwriting source facts and rejects a nonconst reference-to-xvalue
sequence. No oracle or protected reference was changed; the newly introduced
reference fixtures were produced by the frozen C++ owner in the owning lane.

### Source responsibility and architecture review

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, the handout and clone plan,
and reviewed the diff since INITIAL/stage base. Consulted frozen
`analyzer_expressions.cpp`, `analyzer_conversions.cpp`, `conversions.cpp`,
`analyzer_calls.cpp`, `analyzer_member_pointers.cpp`,
`analyzer_initialization.cpp`, `analyzer_construction.cpp`,
`analyzer_bodies.cpp`, `analyzer_declarations.cpp`, `analyzer_types.cpp`,
`analyzer_literals.cpp`, `constant_evaluator.cpp`, `constant_expressions.cpp`
and `semantics_dump.cpp`; their owning Rust modules and repaired boundaries
are recorded in `docs/port/source-map.tsv`.

Traced original syntax `ExprId`/`StmtId`/declarator IDs through canonical
`TypeId`, scoped/indexed `BindingId`/`EntityId`, source visibility, candidate
conversion facts, selected direct/indirect calls, initialization completion
and the iterative dump. Parentheses share the original facts. There is one
syntax store and no second semantic syntax tree, synthetic element AST,
textual transport, duplicate parser or rendered semantic identity key.
Function indexes visit only the queried name/associated namespaces. No
unrelated declaration scan, global cache invalidation, fixture recognition,
C++ implementation delegation, unsafe path, new dependency, lint waiver,
mechanical split or production tool-name change was found or introduced.
Rust naming and source responsibilities remain compliant.

Candidate rejection is optional typed data, not diagnostic rendering.
Losing sequences truncate; selected argument bindings travel with their
conversion; query scratch releases contents on every return. Nested calls
complete before borrowing reusable scratch. Function-local body contexts
restore loop/switch state on failure and scope exit. Declaration failures
are terminal for the unit, whose owner is discarded; they do not retry
against partially published state. Namespace origins/default introduction
ordinals preserve source visibility. No speculative class/member/template
demand was added. Arrays process explicit elements only, including huge
zero-filled scalar tails. Binary left spines and semantic output are
iterative; remaining recursive walks retain the genuine byte-budget guard.

Numeric review covered target integer wrap/sign extension, enum promotions,
representability and floating narrowing overflow midpoints against decoded
literal bits. Diagnostic bytes and source/operand/dump order were compared
through the protected adapter. Nonliteral floating interpretation, braced
call-argument ranking and class construction/demand remain explicit future
boundaries, as recorded before this audit. No source/object generation was
changed: backend, optimizer and allocator lifetimes/order are preserved.
This checkpoint makes no PA34 provenance, path-identity or object equality
claim. The next dependency boundary remains class construction/member facts
and anonymous-union storage, followed by the separate base/member-pointer,
template, deferred-demand and interpreter owners listed below.

### Validation evidence

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-checkpoint-audit-20261013/`.
Final candidate cppgm SHA-256:
`9fb82d645dfba02d3bfb9192d6a1bf83818ab1905a54b64ac72b340f082428ec`.
Frozen C++ cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command / check | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | 179/198, same 19 unfinished failures; exit 1 | `stage-required.log`, `fixed-manifest.json` |
| `make test-pa7` | Same 179/198; exit 2 from make | `stage.log` |
| `make course-pa7` | 210/229; all 31 new regressions pass, same unfinished 19; overall failure retained | `course.log`, `regression-receipts.json`; `../clone/course/pa7/test/course-result.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | PASS, PA1–PA6 fixed differential and original course lanes | `prior.log`, `prior-summary.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | PASS | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | PASS, 179/179; independently same 198 identities, zero new failures | `preserve.log`, `turn-start-manifest.json`, `preservation.json` |
| `make lint` (protected `clone_quality.py`) | PASS, rustfmt, strict all-feature Clippy, debug/release tests and file audit | `lint.log`, `quality-final.log` |
| `make audit-rust` | PASS, 126 files, zero findings | `audit.log` |
| `python3 .../pa7-checkpoint-audit-20261013/probe.py` | 55/55 exact matches | `probe-summary.json`, `probes.jsonl` |
| `python3 .../initialization/probe.py` | 78/78 exact matches | `initialization/probe-summary.json`, `initialization/probes.jsonl` |
| `python3 .../pa7-checkpoint-audit-20261013/measure.py` | Paired output equality and two native qualification receipts pass | `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

Stage progress preservation is proven by identity comparison: accepted passes
179, final passes 179, lost identities zero, coverage 198 both times and final
failures 19, equal to the audit-start limit. This is a partial checkpoint
certification even though the full-stage/course commands correctly fail.

### Resource costs and protected stack qualification

Final-binary warm measurements ran after the test jobs ended: A/A calibration,
then paired ABBA/BAAB blocks, retained raw timings/RSS, source/binary hashes
and output digests, with two protected receipts per workload. Measurements
include process startup and dump I/O, and make no native frame cutoff or
PA35 generated-code improvement claim.

| Arrays / explicit elements | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| 128 / 4 | 6.59 / 6524 | 21.88 / 8746 |
| 512 / 4 | 9.12 / 7220 | 25.88 / 7988 |
| 2048 / 4 | 19.56 / 9288 | 48.77 / 8596 |
| 128 / 16 | 7.61 / 6478 | 22.40 / 8382 |
| 128 / 64 | 11.80 / 7176 | 31.89 / 8372 |
| 128 / 256 | 30.79 / 10688 | 72.02 / 12872 |

Final native probes retain 1,500 unary operators and 1,000 ordinary nested
calls in array initializers, starting at 1 MiB. Both adapter receipts have
`match: true` and `stack_qualification.passed: true`; both tools succeed with
identical output at a shared 16 MiB stack. Judge those protected receipts,
not the adapter's retained original exits or exhaustion positions. Raw
attempts/retries remain under `stack-qualifications/`. No stack cutoff,
frame size or diagnostic position was calibrated. Shared filesystem storage was about
28.3 GB used under the 107.4 GB capacity ceiling; artifacts remain outside
the committed checkout.

## Accepted implementation before this audit

Coherent partial PA7 scalar/list and array initialization handoff. The fixed
198-case manifest improves from 174 passes / 24 failures to 179 passes / 19
failures. All 174 turn-start passing identities remain; the five new passing
identities are recorded below. Coverage is unchanged. PA7 remains incomplete.
The preceding goal turn made authoritative progress in clean commit `37c4358`,
porting ordinary function calls and overload resolution. This turn inspected its
actual ledger, fixed receipts and C++ owners before extending the original-AST
initialization boundary. A commit or one passing fixture was not the stopping
boundary: initialization invariants, related casts/defaults/returns, source
responsibilities, supported probes and scaling were completed together.

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, `pa7/README.md`,
`docs/rust-clone-plan.md` and the prior stage ledger. Consulted frozen C++ owners
at `/home/vishvananda/work/references/v5rust-cpp/dev/src/sema/` directly:
`analyzer_initialization.cpp`, `analyzer_construction.cpp::initialize_object`,
`analyzer_expressions.cpp::{braced_list,functional_cast,subscript,typed}`,
`analyzer_declarations.cpp::{complete_array_from_initializer,declare_variable}`,
`analyzer_bodies.cpp`'s return list path, `constant_evaluator.cpp`,
`constant_expressions.cpp`, `constant_construction.cpp` and `semantics_dump.cpp`.
`docs/port/source-map.tsv` records their Rust responsibilities and explicit
remaining boundaries.

The prior completed scalar/procedural and ordinary-call groups remain intact:
original expression facts, shared parentheses, indexed lookup/ADL, canonical
function identities, selected conversions, ordinary overload ranking, source
visibility, default-argument IDs, builtins, procedural scopes/control flow and
the iterative semantic dump. Full prior ledgers are retained at `e72f1d8` and
`37c4358`. Their evidence remains in `artifacts/pa7-scalar-core-20261013/` and
`artifacts/pa7-calls-20261013/` under the shared artifact parent.

## Completed group and typed ownership

- `initialization.rs` owns nonclass object/list initialization. It consumes
  the original `ExprId`/initializer list ranges, canonical `TypeId` and lexical
  `ScopeId`; it does not reparse, clone syntax, render transport text or create
  synthetic element nodes. Scalar lists retain lvalue list facts, at most one
  element, empty value initialization and exact nested/multiple-list diagnostics.
  Elements keep original types/categories plus their selected conversion.
- Arrays consume explicit elements in source order, recursively respecting
  braced subarrays and brace elision. An omitted outer bound counts initialized
  objects rather than raw syntax entries. Zero-filled scalar tails need no
  per-element semantic allocation or work; semantic string output necessarily
  prints each requested array element. Class-element zero/default construction
  is an explicit status-86 dependency even with no written elements.
- `declarations.rs` keeps types-mode raw bound formation and semantics-mode
  typed completion distinct, checks element completeness before deducing a list
  bound and rejects a remaining incomplete object after initialization. Earlier
  array declarations supply their canonical bound. `Graph`'s private variable
  publication/completion methods update binding/entity type identity while the
  variable remains visible to its own initializer; no global declaration scan
  or cache invalidation is introduced. Declared references keep reference type
  identity even when their list denotes a temporary object.
- Narrow/wide strings consume retained decoded literal bytes. Character
  compatibility, signed/unsigned narrow characters, list/direct diagnostics,
  bound limits and parenthesized string rules follow the C++ owner. The semantic
  view expands a variable's string initializer into a typed braced list of code
  units and zero padding, preserving element cv and target signedness.
- Narrowing consumes original source facts and the selected conversion. Integral
  widths/signedness and representability use the existing explicit target
  arithmetic owner. Integer-to-floating constants check significant bits exactly;
  floating-to-integral always narrows. Narrower floating literals use their
  decoded binary64/x87 significands/exponents and the target overflow midpoint,
  including sign and zero handling; no host floating approximation is used.
  Nonliteral floating interpretation remains a named status-86 dependency.
- `scalar_casts.rs`, `default_arguments.rs` and `procedural_statements.rs`
  consume the same initialization algorithm for scalar/array functional lists,
  braced defaults and braced returns. Array functional lists use aggregate
  initialization directly, preserving the oracle's distinct character-string rule. Defaults retain introduction visibility
  and declaration-time checking. Calls consume the recorded default IDs.
  Bare lists in expression-only contexts diagnose exactly; overload ranking
  of explicit braced call arguments remains a separate dependency.
- `constants.rs` records the canonical zero of const integral empty-list objects
  and keeps the exact unavailable-reference diagnostic. Named constant reads
  have one owning helper, preserving unevaluated/short-circuit behavior.
  Full symbolic referents, class/array constant storage and interpreter demand
  remain pending.

One ownership test checks brace-elided bound completion, shared variable/binding
identity, unchanged source element category/type, selected element conversions,
and bounded facts for a billion-element scalar array with one initializer.
Rustfmt, strict Clippy and the companion file audit pass without new lint
expectations or suppression. The audit remains authoritative over Clippy.

No dependency, unsafe code, FFI, environment read under `dev/src/`, compiler
subprocess, fixture recognition, reference answer or protected input/policy
change was introduced. Optimizer, register allocator, lowering, emission order
and backend ownership are unchanged. This semantic group makes no PA34 or
whole-program object-byte completion claim.

## Validation and retained evidence

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-initialization-20261013/`.
`handoff.json` records the final commit, binary hashes and command exits.
Candidate cppgm SHA-256: `565bc14360dac94ea0b1544d656b93a8fb4640c63e3a0d776a911c3d8659599c`.
Frozen C++ cppgm++ SHA-256: `9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.
`fixed-manifest.json` retains all current identities/receipts; the independent
`turn-start-manifest.json` comparison requires all 174 prior passing identities
and all 198 covered identities. Raw protected traces, diagnostics and output
files remain at `artifacts/clone/cases/pa7/<case-hash>/`.

| Command | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log` |
| `make test-pa7` | FAIL overall; 179/198, 19 remaining failures | `stage.log`, `fixed-manifest.json` |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | FAIL overall; same 179/198 identities | `stage-required.log` |
| `make course-pa7` | FAIL overall; 179/198, remaining failure marker retained | `course.log`; full `artifacts/clone/course/pa7/test/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | PASS, PA1–PA6 differential and original course lanes | `prior.log`; `artifacts/clone/course/paN/test/` for N=1..6 |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | PASS; independent 174-identity comparison also passes | `preserve.log`, `preservation.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | PASS | `preflight.log` |
| `make lint` / protected `clone_quality.py` | PASS, rustfmt, all-feature strict Clippy, debug/release workspace tests and audit | `quality.log` |
| `make audit-rust` | PASS, 126 files, zero findings | `audit.log` |
| `python3 .../pa7-initialization-20261013/probe.py` | 78/78 protected exact supported probes | `probe.py`, `probe.log`, `probe-summary.json`, `probes.jsonl` |
| `python3 .../pa7-initialization-20261013/measure.py` | Exact supported scaling and native resource receipts | `measure.py`, `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

The probes cover copy/direct lists, zero initialization, integer/bool/float
narrowing boundaries, enum/pointer lists, reference-related versus temporary
lists, functional scalar/array lists, defaults/returns, array redeclarations,
multidimensional brace elision, too many/nested initializers, character kind,
decoded signed bytes, padding, and const propagation. They supplement the fixed
course gate. `boundary-probe.py` and `boundary-probes.jsonl` retain a class-array empty-list reproducer
that correctly remains an incomplete Rust path (86), not a successful rejection.
Four initial hexadecimal-floating probes exposed the existing strict C++11
lexer boundary (`lex/number.rs`); raw mismatch evidence remains under this root.
Decimal probes exercise the same initialization thresholds without changing
that earlier-phase grammar dependency. No oracle/harness defect was encountered.

## Measured costs and protected stack qualification

Measurements use 128/512/2,048 independent arrays with four brace-elided elements,
then 128 arrays with 16/64/256 elements. Every input/binary hash, output digest,
wall/RSS reading and A/A calibration is retained in `measurements.json`.
Paired ABBA and BAAB runs follow calibration; every workload has two protected
adapter receipts. These warm process observations include startup, parsing,
semantics and dump I/O, not a cold-cache or hardware-counter gate.
PA35's generated-code improvement targets are outside this group's requirements.

| Arrays / elements | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| 128 / 4 | 5.31 / 6532 | 18.18 / 8374 |
| 512 / 4 | 7.69 / 7226 | 23.75 / 8380 |
| 2048 / 4 | 21.14 / 9236 | 46.98 / 8574 |
| 128 / 16 | 6.59 / 6504 | 21.65 / 8006 |
| 128 / 64 | 13.03 / 7154 | 33.14 / 8730 |
| 128 / 256 | 30.91 / 10754 | 76.17 / 12808 |

Native probes fix 1,500 unary operators and 1,000 ordinary nested calls inside
an array initializer, starting at a 1 MiB stack. Both protected receipts have
`match: true` and `stack_qualification.passed: true`, with identical successful
output at a shared 16 MiB retry. Raw original attempts and retries are retained
under `stack-qualifications/<receipt-id>/`; only the protected receipt establishes
qualification. The genuine byte-budget guard is retained. No native cutoff,
frame size or exhaustion diagnostic position was calibrated.

## Next dependency boundary and architecture review

The next group is class object construction/member facts and anonymous-union
storage projection: `analyzer_initialization.cpp`, `analyzer_construction.cpp`,
class/member/function demand and `semantics_dump.cpp`. It needs typed
construction plans, explicit class completion, source-ordered members and
constructor/default-body demand. Arrays of class objects must consume those
plans, including empty tails, rather than imitate scalar zero initialization.
Base conversions/member pointers, template specialization and deferred closure
remain distinct dependent owners. Braced call-argument sequence ranking and
nonliteral floating constants are additional explicit production boundaries.

Review an array declaration from prior canonical identity through list
consumption and brace-elided bound completion to retained element conversions;
then review a reference list's declared type versus its temporary list facts.
The single AST, borrowed list ranges, compact IDs and dense facts are preserved.
No per-zero-tail element allocation, unrelated declaration scan, speculative
class completion, syntax synthesis or textual transport is added. Backend
lifetimes and optimizer ordering stay outside this frontend-only change.

The 19 fixed failures remain class/member/template dependencies (16 explicit
incomplete paths and three mismatching class/anonymous-union semantic views).
They are not accepted negative cases, and the stage has not advanced.

## Exact newly passing fixed identities

Relative to `pa7/`:

```text
tests/general/200-braced-narrowing-integer-constant-bad.t
tests/general/200-function-pointer-array-deduced-bound.t
tests/general/200-signed-char-array-string-initializer.t
tests/general/300-subscript-commuted-expression.t
tests/general/300-subscript-unscoped-enum-index.t
```

## Exact remaining fixed identities at the preceding independent audit

```text
tests/general/100-distinct-anonymous-type-identities.t
tests/general/100-namespace-static-anonymous-union-injected-members.t
tests/general/100-private-nested-out-of-class-constructor-definition.t
tests/general/100-private-nested-out-of-class-member-definition.t
tests/general/100-qualified-base-data-member-hides-function.t
tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t
tests/general/200-local-anonymous-union-variable.t
tests/general/300-block-anonymous-union-injected-members.t
tests/general/300-default-argument-nonstatic-member-bad.t
tests/general/300-deferred-demand-closure.t
tests/general/300-elaborated-local-struct-copy-init.t
tests/general/300-member-cv-overload-identity.t
tests/general/300-member-function-pointer-return-pointer-const.t
tests/general/300-member-function-pointer-type-alias-and-function.t
tests/general/300-member-pointer-type-alias-and-function.t
tests/general/300-most-vexing-local-function-member-call-bad.t
tests/general/300-reference-binding-pointee-const-pointer.t
tests/general/300-static-cast-member-overload-prefers-nontemplate.t
tests/general/300-static-cast-overloaded-function-template-argument.t
```
