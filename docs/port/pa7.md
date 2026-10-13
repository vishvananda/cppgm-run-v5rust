# PA7 port ledger

Stage base commit: `9a851dcedb1e56bbc6585c84c5990f6ef1a1c372`.
Current implementation group base: `8e3665e7b101ad33e94150a67a949e662ec064e2` (clean implementation entry).
Last audited commit: `a05b466c47eac23ca20be5d23d5bed00183e72b8` (independent partial checkpoint audit).

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
