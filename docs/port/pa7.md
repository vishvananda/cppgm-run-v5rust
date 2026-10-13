# PA7 port ledger

Stage base commit: `9a851dcedb1e56bbc6585c84c5990f6ef1a1c372`.
Current implementation group base: `e72f1d8d28ce56e34e16660fd124c402a2265103`.
Last audited commit: INITIAL (unchanged; implementation does not advance Ralph's audit).

## Current checkpoint

This is a coherent partial PA7 ordinary-function call and overload implementation
handoff. Full PA7 remains incomplete. The fixed 198-case manifest improves from
83 passes / 115 failures to 174 passes / 24 failures: 91 formerly failing
identities now match, all 83 turn-start passing identities remain, and coverage
is unchanged. The preceding goal turn made authoritative progress: its clean
commit `e72f1d8` implemented the scalar/procedural group and established the
83-case baseline. This turn inspected that commit, its ledger and actual raw
failure receipts before extending it.

The previous scalar checkpoint's source review, 57 probes, original scaling
inputs and receipts remain at
`/home/vishvananda/work/private/v5rust/artifacts/pa7-scalar-core-20261013/`.
The current group builds on those original-AST scalar facts and conversions.
The retained completed groups cover scalar literals/ids and shared parentheses,
parameter/reference/enumerator facts, scalar copy/direct conversions and casts,
built-in operators/subscripts, procedural scopes/returns/conditions/loops/switch
rules, and the iterative semantic view. Their owning modules and C++ mappings
remain in the source map; the full prior ledger is also retained at commit
`e72f1d8`.
Neither checkpoint establishes complete assignment coverage or advancement.
Ralph owns independent checkpoint/final audit and stage advancement.

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, `pa7/README.md`,
`docs/rust-clone-plan.md` and this stage ledger. Consulted the frozen implementation
at `/home/vishvananda/work/references/v5rust-cpp` directly, particularly
`dev/src/sema/analyzer_calls.cpp`, `analyzer_expressions.cpp`, `conversions.cpp`,
`analyzer_declarations.cpp`, `analyzer_initialization.cpp`, `analyzer_builtins.cpp`,
`analyzer.cpp`, `analyzer_lambdas.cpp`, `semantics_dump.cpp` and
`dev/src/lex/gnu_builtins.h`. Existing PA6/PA7 Rust owners were inspected alongside
them. `source-map.tsv` records translated responsibilities and the remaining
class/template/initialization/interpreter dependencies.

## Completed group and typed data flow

- `graph.rs`, `lookup.rs`, `declarations.rs` and `function_names.rs` index
  ordinary overloads by `(ScopeId, ValueKey)`. Canonical parameter signatures
  identify redeclarations; return-type conflicts, qualified mismatches,
  repeated definitions and deleted use preserve C++ diagnostics. The first
  entity origin and visibility survive interleaved declarations and using
  aliases. Using declarations introduce every required overload and diagnose
  signature conflicts. Lexical and namespace-edge lookup merge function lists
  without scanning unrelated declarations. Single-function query results stay
  inline; only larger sets allocate a temporary candidate vector.
- `function_resolution.rs` owns checked `OverloadId` slices in unit storage,
  function designations, target-directed ordinary function-pointer/reference
  selection and sequence ordering. Original `ExprId` facts retain a designation
  until its use selects a function. Parentheses share facts. One `&` or `*`
  designator can resolve by a call; deeper unresolved designations require a
  target, as the C++ owner specifies. Unsupported bare sets reject, and selected
  deleted functions reject. Explicit scalar casts consume the same selections.
- `procedural_calls.rs` types arguments before borrowing reusable resolution
  scratch. It filters arity/defaults, represents expected candidate rejection
  as `Option<Conversion>`, retains all viable scalar/reference/ellipsis sequences,
  selects a provisional best, then proves it better than every other candidate.
  Ambiguity/no-viable diagnostics are rendered only after resolution. Rank,
  subsequence, qualification, reference cv/category and pointer-bool ordering
  follow `ConversionRules::compare`; no new optimization or candidate pruning
  policy is introduced. The selected binding and operand sequences stay on the
  original graph; applying a selected sequence does not classify it again.
- Direct calls retain `CallTarget::Direct(BindingId)`; indirect function,
  function-reference and pointer calls retain `CallTarget::Indirect(TypeId)`.
  Indirect arity and argument conversions diagnose at the C++ positions. Call
  facts carry the semantic result category/type; the dump uses the declared
  return spelling, including references. Scalar result cv stripping and an
  rvalue reference to a function yielding an lvalue follow `result_category`.
- `call_lookup.rs` associates only argument types, enclosing namespaces and
  indexed inline namespace edges. It ignores using directives during ADL,
  preserves qualified-call/block-declaration suppression, and permits block
  using declarations whose entity originates in a namespace. Fundamental-only
  arguments need no association walk. Class bases, hidden friends, template
  associated entities and user-defined calls remain dependencies.
- `default_arguments.rs` checks canonical parameter defaults at declaration
  context, retains original expression IDs and introduction ordinals, rejects
  repeated/missing trailing defaults, and protects evaluated local-variable
  use. Parameter names are not invented in the default's lookup context;
  scalar declarations follow the observed C++ owner. Unevaluated sizeof and
  decltype/query contexts retain their existing distinction. Calls and the dump
  consume the retained defaults without parsing or typing them again. Member
  and template default demand remains pending.
- `builtin_calls.rs` recognizes interned reserved identifiers, lazily declares
  `__builtin_abort`, and lets normal resolution check its arity. The one-operand
  unevaluated `__builtin_constant_p` query folds propagated integral constants
  and floating/null/string literals. Nonliteral floating/address interpretation
  remains an explicit status-86 dependency; it is not silently approximated as
  nonconstant. Remaining hosted builtin forms are unported.
- `scalar_conversions.rs` canonicalizes target types and handles array-element
  cv/reference compatibility and pointer-to-array shape checks in its owning
  algorithm. Operators check unresolved function designations before treating
  them as scalar values. Semantic decltype consumes the call's existing facts.
- `semantics_dump.rs` prints selected direct callees, indirect operand trees,
  defaults and source operand categories in C++ order. `dump.rs` shares the
  iterative type renderer with a separate qualified-class semantic view,
  preserving PA6's source-facing type view. Namespace/anonymous namespace and
  class ownership is retained as typed scope facts, never inferred from text.

Two Rust ownership tests validate retained selected sequences on original AST
nodes and source visibility across interleaved overload redeclarations.
New lint expectations were not added; obsolete rank/qualification and parameter
clause-location expectations were removed as these facts acquired consumers.
Rustfmt, strict Clippy and the companion source audit remain authoritative.
No dependency, unsafe code, FFI, environment read in `dev/src/`, compiler
subprocess, fixture recognition, reference answer or protected policy change
was introduced. Optimizer, register allocator, lowering and object generation
are unchanged. This semantic-view group makes no new object-byte or PA34
completion claim.

## Validation and retained evidence

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-calls-20261013/`.
`handoff.json` records the implementation commit, exact binary hashes, command
results and stage identities after committing this ledger. Candidate cppgm
SHA-256: `1aea4927ed89436a0936b36e7375b819bc5d11396005919ba4f0447f39595630`.
Frozen C++ cppgm++ SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log` |
| `make test-pa7` and required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | FAIL overall; 174/198 exact passing identities, 24 remaining failures | `stage.log`, `fixed-manifest.json`, `failing-case-ids.txt` |
| `make course-pa7` | FAIL overall; 174/198, remaining failure marker retained | `course.log`; full `artifacts/clone/course/pa7/test/{course.log,course-result.json,counts.txt,trace.jsonl}` under the shared artifact parent |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | PASS, all PA1–PA6 differential and course lanes | `prior.log`; `artifacts/clone/course/paN/test/` for N=1..6 |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | PASS, 83/83 prior passing identities | `preserve.log`; `baseline-manifest.json`, `fixed-manifest.json` independently record zero regressions and 91 new identities |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | PASS | `preflight.log` |
| `make lint` / protected `clone_quality.py` | PASS, rustfmt, strict all-feature Clippy, debug/release workspace tests and audit | `quality.log` |
| `make audit-rust` | PASS, 125 files, zero findings | `audit.log` |
| `python3 .../pa7-calls-20261013/probe.py` | 71/71 protected exact matches | `probe.py`, `probe.log`, `probe-summary.json`, `probes.jsonl` |
| `python3 .../pa7-calls-20261013/measure.py` | Exact scaling receipts and protected native qualification | `measure.py`, `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

Each failing fixed case's raw exit/diagnostics/output hashes and mismatch paths
are in `fixed-manifest.json`. Original protected case traces remain at
`/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa7/<case-hash>/`.
The manifest retains all 198 fixed identities; counts alone are not the
preservation evidence. No oracle/harness defect was encountered.

The focused probes cover multiple using-imported overloads and conflicts,
nested calls, target-directed addresses/references/arguments/casts, scalar
promotions, pointer/array/reference ranking, indirect and reference-return
calls, canonical parameter adjustment, defaults and diagnostics, deleted use,
qualified definitions, declaration visibility, absence of a global best,
ellipsis, enum/inline/class-pointer ADL, suppressed ADL, unnamed namespaces,
reserved builtins, folded queries, decltype and conditional function callees.
They supplement the fixed/course gate and do not redefine full-stage coverage.

## Measured costs and native resource qualification

Measurements use frozen supported call inputs with 128, 512 or 2,048 independent
functions and a two-member overload set; a second axis uses 128 functions and
4, 16 or 64 overloads. All source/binary hashes, output digests, individual
wall/RSS observations and A/A calibration are retained in `measurements.json`.
Each input is checked through the protected adapter. Separate same-binary A/A
runs precede paired ABBA and BAAB blocks. These are warm process-level readings
including startup, frontend work and output I/O; no cold-cache or Linux hardware
counter gate is claimed. PA35's generated-code improvement targets are unused.

| Functions / overloads | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| 128 / 2 | 5.77 / 6472 | 18.84 / 8378 |
| 512 / 2 | 9.21 / 7252 | 25.50 / 7996 |
| 2048 / 2 | 22.56 / 10672 | 52.49 / 9950 |
| 128 / 4 | 5.98 / 6484 | 19.50 / 8378 |
| 128 / 16 | 6.48 / 6516 | 19.42 / 8358 |
| 128 / 64 | 6.84 / 6520 | 21.89 / 8000 |

The observations bound these supported paths, not all semantic workloads.
The genuine native byte-budget guard is retained. Fixed unary and nested-call
probes start at a 1 MiB stack, and only protected receipts establish equality
through a shared larger stack. Raw exhaustion diagnostics/positions are kept
in receipts; no cutoff, frame-size or diagnostic-position calibration is used.
`native-probes.jsonl` records `match` and `stack_qualification.passed`, with
raw attempts and retries under `stack-qualifications/<receipt-id>/`.

The unary input fixes 1,500 operators; the call input fixes 1,000 nested
ordinary calls. Both receipts report `match: true` and
`stack_qualification.passed: true`, with one shared 16 MiB successful retry.
Successful output SHA-256 values are respectively
`c816cbed6d5c3f042cdcd873c923cb5376a2fc9f4f3e327a39b7cf0929fc5021` and
`d47e1dcdf4efe5ca267533e88c3a97b318e368ef405e453523efee63c7147660`.

## Next dependency boundary and architecture review

The next coherent group is class objects and their initialization/member facts:
`analyzer_initialization.cpp`, class/member/constructor owners, array and list
initialization, anonymous-union storage/member projection and semantic printing.
It must preserve explicit class-completion/constructor demand, source order,
object identities and retained conversion/construction plans. Class bases and
member pointers have their own typed boundaries; template deduction and deferred
member/default bodies must not be pulled into scalar calls speculatively.

The 24 fixed failures cluster at the earliest current boundary: six class-function
stubs, five class/array initialization stubs, two class special-member stubs,
two inheritance stubs, two member-pointer stubs, two extended-member expression
stubs, one scalar-list stub and one template-specialization lookup stub. Three
non-stub mismatches remain: two anonymous-union storage/member views and one
class-object constructor-action view. The latter is the class object in
`300-reference-binding-pointee-const-pointer`, whose call/reference selection
itself is now supported. Nonliteral floating/address queries, full class/base
conversion ranking, template/hidden-friend ADL and full constexpr demand also
remain production dependencies even where this manifest does not isolate them.

Architecture review should trace an overload-imported call from indexed lookup
through canonical entity/signature, scratch candidate rejection and best-against-all
comparison to original operand sequences and dump; then trace a default argument
from its declaration context to one selected call. The ownership tests and source
review confirm no syntax/token clone, rendered-text transport, unrelated entity
scan or reclassification during ordinary selected-sequence application. Resolver
scratch clears contents between queries and preserves capacity; unit-owned facts
retain only compact IDs/conversions/default expressions. Class/template demand,
construction plans, later emission and per-function backend ownership remain
explicit later consumers, not claims established by 174 fixture matches.

## Exact remaining fixed-manifest case IDs

Relative to `pa7/`:

```text
tests/general/100-distinct-anonymous-type-identities.t
tests/general/100-namespace-static-anonymous-union-injected-members.t
tests/general/100-private-nested-out-of-class-constructor-definition.t
tests/general/100-private-nested-out-of-class-member-definition.t
tests/general/100-qualified-base-data-member-hides-function.t
tests/general/200-braced-narrowing-integer-constant-bad.t
tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t
tests/general/200-function-pointer-array-deduced-bound.t
tests/general/200-local-anonymous-union-variable.t
tests/general/200-signed-char-array-string-initializer.t
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
tests/general/300-subscript-commuted-expression.t
tests/general/300-subscript-unscoped-enum-index.t
```
