# PA7 port ledger

Stage base commit: `9a851dcedb1e56bbc6585c84c5990f6ef1a1c372`
Last audited commit: INITIAL (unchanged; PA7 has not had Ralph's independent audit).

## Contract and checkpoint

This is a coherent partial PA7 scalar-expression and procedural-statement
implementation group, not full-stage acceptance or advancement. The clean
incoming checkout returned status 86 at the `--emit-semantics` driver boundary
for all 198 fixed cases. No PA7 implementation from an interrupted turn was
retained there; this turn made implementation progress instead of restating
that boundary. The fixed manifest now passes 83/198: failures decreased from
198 to 115. Every case passing in the initial 61-case implementation run and
the subsequent 83-case run remains passing. The turn-start passing set was
empty; the protected preservation check reports 0/0.

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, `pa7/README.md`,
`docs/rust-clone-plan.md`, this ledger and the existing PA6 owners. Consulted
C++ directly in `/home/vishvananda/work/references/v5rust-cpp`:
`dev/cppgm++.cpp`, `sema/analyzer.cpp`, `analyzer_bodies.cpp`,
`analyzer_expressions.cpp`, `analyzer_literals.cpp`, `analyzer_conversions.cpp`,
`analyzer_initialization.cpp`, `analyzer_declarations.cpp`,
`analyzer_member_pointers.cpp`, `conversions.cpp`, `conversions.h`,
`expression_facts.h`, `semantics_dump.cpp` and existing type/lookup owners.
The source map records each partial responsibility and its unported remainder.

## Completed behavior group and production invariants

- The driver selects `--emit-semantics` and rejects simultaneous emit modes.
  Source wrappers, per-file ownership and located diagnostics share the
  existing PA5/PA6 pipeline. No tokens or printed representations are reparsed.
- `scalar_expressions.rs` types supported literal/id/parenthesis expressions
  against canonical `TypeId`, including parameter-object adjustment, reference
  removal, enumerator folding and the zero-integer-literal null flag.
  `expression_facts.rs` stores dense facts against original `ExprId` values.
  Parentheses share a flattened fact handle, including conversions.
- `scalar_conversions.rs` owns scalar copy/direct conversions and basic
  references: arithmetic promotions/conversions, pointer/null/bool/void
  conversions and recursive pointer qualifications. Deep cv additions require
  intermediate const. CWG 2352 pointer-reference compatibility is retained.
  Operands retain their source type/category; typed conversions retain target,
  result, lvalue transform, value conversion, rank, qualification, initialization
  and temporary state. Two narrow, test-aware dead-code expectations retain
  rank/qualification until the next overload consumer; they waive no audit cap.
- `scalar_operators.rs` owns built-in unary, prefix/postfix increment, binary,
  comparison, conditional, assignment/compound assignment, pointer arithmetic
  and subscript rules. The original binary left spine uses reusable explicit
  scratch. Pointer cv composition follows `analyzer_member_pointers.cpp`,
  including CWG 1512. No overload candidate scan is introduced on scalar paths.
- `scalar_casts.rs` owns supported scalar, enum, pointer and reference explicit
  casts, including scalar functional notation and named type-alias casts.
  Cast constness and reference value categories follow the C++ owner; carrier
  conversions do not overwrite the source operand. Empty scalar/void casts
  retain their folded course spelling. Class and list casts remain dependencies.
- `procedural_statements.rs` owns one function's return type, loop count and
  switch stack. Returns and conditions consume typed conversions. Cases are
  typed, evaluated and checked for narrowing; duplicate case/default labels and
  invalid break/continue positions are rejected in C++ order. Initialized local
  counts restore at block and selection/iteration lifetimes; static, extern and
  thread-local declarations do not count. Nested function entry moves/restores
  context instead of copying it.
- `semantics_dump.rs` is an iterative view of the same AST and graph. Source
  declaration order, canonical function parameters, namespace qualifiers,
  original operands, folded enumerators, null initialization retyping, temporary
  reference conversions and omission of sizeof operands follow the C++ printer.
  It creates no owning semantic syntax tree or phase transport.

Checked store IDs and target integer arithmetic reuse the existing owners.
Expression fact storage is registered as a model with the 1,800-code-line cap.
No new dependency, unsafe code, environment read in `dev/src/`, compiler
subprocess, fixture recognition or oracle-answer path was added. The optimizer,
register allocator, lowering and object generation were not changed; this group
emits only the semantic view and makes no new object-byte completion claim.

## Validation evidence

Retained artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa7-scalar-core-20261013/`.
`handoff.json` records the exact implementation commit, binary hashes and gate
results after committing this ledger. Candidate `cppgm` SHA-256:
`aa94a39f859915602b17b783821ae27c030901935aa5dc09f05c1fec127b495c`.
Frozen C++ `cppgm++` SHA-256:
`9cb6584b85c16aca6af92d7efc564c29996b8e31b6cc7bad1d19b0fdc9433d5e`.

| Command | Result | Evidence relative to artifact root |
| --- | --- | --- |
| `make build` | PASS, locked release workspace | `build.log` |
| `make test-pa7` / `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa7` | FAIL overall, 83/198 exact matches, 115 remaining failures | `stage.log`, `fixed-manifest.json`, `failing-case-ids.txt` |
| `make course-pa7` | FAIL overall, 83/198, failure marker retained | `course.log`; full `/home/vishvananda/work/private/v5rust/artifacts/clone/course/pa7/test/{course.log,course-result.json,counts.txt,trace.jsonl}` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa7` | PASS, all PA1–PA6 differential and course lanes after final compiler change | `prior.log`; `/home/vishvananda/work/private/v5rust/artifacts/clone/course/paN/test/` for N=1..6 |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa7` | PASS | `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa7` | PASS, 0/0 baseline passing identities | `preserve.log` |
| `make lint` / protected `clone_quality.py` | PASS: rustfmt, strict Clippy, supported feature builds, debug/release tests and audits | `quality.log` |
| `make audit-rust` | PASS, 120 files, zero findings | `audit.log` |
| `python3 .../pa7-scalar-core-20261013/probe.py` | 57/57 exact protected matches | `probe.py`, `probe.log`, `probe-summary.json`, `probes.jsonl` |
| `python3 .../pa7-scalar-core-20261013/measure.py` | Six exact scaling receipts; protected native qualification passes | `measure.py`, `measure.log`, `measurements.json`, `measurement-probes.jsonl`, `native-probes.jsonl` |

Each failing fixed case's protected receipt and mismatch directory is retained
in `fixed-manifest.json`. The original case trace/course log remains at
`/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa7/<case-hash>/`.
The preservation claim is by identities, not just fixture counts. No manifest,
fixture, checker, reference source, oracle binary or Ralph state was changed.
The local module-role metadata only registers the new model under the existing
stricter cap; protected gate policy remains unchanged.

The focused probes cover promotions, arithmetic, logical/comma categories,
address/increment failures, direct and temporary references, pointer-reference
qualification, deep const rejection, void pointers, pointer/integer comparisons,
commuted subscripts, declared conditions and loop scopes, switch duplicate and
initialization rules, return failures, scoped enums, scalar/reference casts,
named type-alias casts, sizeof, mixed-bool conditionals and constant cases.
Unit tests separately prove flattened fact aliases retain one conversion record
without changing source types/categories and retain promotion/qualification rank
facts for later overload resolution. No oracle defect was encountered.

## Measured costs and protected stack qualification

The fixed scaling inputs contain 128, 512 or 2,048 independent procedural
functions, each with a local arithmetic initializer, while loop and return.
Source and binary hashes, every wall/RSS sample and output digest are in
`measurements.json`. The final run followed the completed assignment gates;
A/A adapter checks precede paired ABBA and BAAB blocks. Measurements include
process startup, all front-end work and dump I/O. Warm observations only; no
Linux hardware-counter or cold-cache performance gate is claimed.

| Functions | C++ median ms / RSS KiB | Rust median ms / RSS KiB |
| --- | --- | --- |
| 128 | 6.19 / 6488 | 19.51 / 8002 |
| 512 | 11.84 / 7884 | 32.57 / 8378 |
| 2,048 | 36.98 / 13506 | 80.99 / 14948 |

These observations show bounded growth on this supported path, not a general
complexity or equal-throughput proof. PA35 generated-code targets are not used.
The native probe fixes 1,500 unary operators at a 1 MiB initial stack. Its
protected receipt reports `match: true`, `stack_qualification.passed: true`,
with one shared 16 MiB retry: both tools succeed with identical streams and
semantic output SHA-256
`c816cbed6d5c3f042cdcd873c923cb5376a2fc9f4f3e327a39b7cf0929fc5021`.
Raw initial attempts and retry artifacts are retained under the receipt's
`stack-qualifications/<id>/` path. The genuine byte-budget guard is retained;
no native exhaustion depth, frame-size imitation or diagnostic-position
calibration was attempted. Judge the protected receipt, not the raw initial
adapter exit.

## Remaining groups and next dependency boundary

The next coherent group is ordinary function overload sets and call resolution:
`analyzer_calls.cpp`, declaration/function records, indexed lookup function
lists, standard-conversion ordering and target-directed function addresses.
It must build on the recorded scalar conversions, distinguish expected candidate
rejection from missing implementation, preserve every required candidate and
retain the selected function plus operand sequences. Monomorphic/indirect calls,
arity, namespace/using lookup, cv-normalized redeclarations, default arguments,
variadic ranking and builtin constant/abort calls belong at that boundary.

The 115 failures cluster by earliest current owner: 75 call-resolution stubs,
15 overload/redeclaration stubs, two function-expression stubs, two extended
expression stubs, one list-initialization stub, five class/array-initialization
stubs, eight class-function/special-member stubs, two inheritance stubs and two
member-pointer stubs. Two anonymous-union storage/member dump divergences and
one decltype-of-call divergence remain non-stub failures. The fixed case receipts
identify all of them exactly. Class/member/constructor demand, anonymous storage,
array/list/default initialization, full array-reference/cv rules, constexpr
function demand, builtin propagation and extended expressions remain explicit
production dependencies even where this manifest does not isolate them.

Independent review should trace the original AST through an initialized scalar
local, a temporary reference and a nested switch, then review the next group's
complete overload lookup keys, source visibility and demand ownership. Neither
57 focused matches nor 83 fixed passing cases establishes whole PA7 coverage.
Ralph owns checkpoint/final audits and advancement; the last-audited field is
not advanced by implementation.

## Exact remaining fixed-manifest case IDs

All IDs below are relative to `pa7/`; coverage remains the same 198-case manifest.

```text
tests/general/100-bad-reference-binding-top-level-const-source.t
tests/general/100-bad-reference-binding.t
tests/general/100-bad-rvalue-reference-binding-top-level-const-source.t
tests/general/100-builtin-prefix-user-function-call.t
tests/general/100-conditional-const-enum-preserves-type.t
tests/general/100-distinct-anonymous-type-identities.t
tests/general/100-integer-zero-to-pointer-call.t
tests/general/100-namespace-alias-call.t
tests/general/100-namespace-static-anonymous-union-injected-members.t
tests/general/100-nested-namespace-call.t
tests/general/100-pointer-to-const-void-call.t
tests/general/100-private-nested-out-of-class-constructor-definition.t
tests/general/100-private-nested-out-of-class-member-definition.t
tests/general/100-qualified-base-data-member-hides-function.t
tests/general/100-reference-binding-top-level-const-pointer.t
tests/general/100-reference-parameter-to-value-call.t
tests/general/100-string-literal-array-type.t
tests/general/100-variadic-call-fixed-prefix.t
tests/general/100-xvalue-reference-value-conversion.t
tests/general/200-adl-inline-namespace-members.t
tests/general/200-bad-noncallable-variable.t
tests/general/200-bad-overloaded-function-id.t
tests/general/200-block-scope-declaration-suppresses-adl.t
tests/general/200-block-scope-using-declaration.t
tests/general/200-braced-narrowing-integer-constant-bad.t
tests/general/200-builtin-constant-p-propagated-expression.t
tests/general/200-by-value-parameter-top-level-cv-redeclaration.t
tests/general/200-canonical-function-parameter-calls.t
tests/general/200-condition-declaration.t
tests/general/200-deep-pointer-qualification-conversion.t
tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t
tests/general/200-float-argument-promotes-to-double.t
tests/general/200-function-decay-deref-call.t
tests/general/200-function-pointer-array-deduced-bound.t
tests/general/200-function-reference-call.t
tests/general/200-inline-namespace-unqualified-call.t
tests/general/200-local-anonymous-union-variable.t
tests/general/200-overloaded-function-argument.t
tests/general/200-paren-argument-list-call.t
tests/general/200-qualified-lookup-inline-namespace-overloads.t
tests/general/200-qualified-overloaded-function-argument.t
tests/general/200-reference-return-call-binding.t
tests/general/200-signed-char-array-string-initializer.t
tests/general/200-switch-case-declaration.t
tests/general/200-switch-default-declaration.t
tests/general/200-using-declaration-call.t
tests/general/200-using-directive-call.t
tests/general/300-array-xvalue-subscript.t
tests/general/300-bad-deep-pointer-qualification-conversion.t
tests/general/300-bad-indirect-call-too-few-arguments.t
tests/general/300-bad-indirect-call-too-many-arguments.t
tests/general/300-block-anonymous-union-injected-members.t
tests/general/300-block-elaborated-enum-type-use.t
tests/general/300-builtin-abort-arity-bad.t
tests/general/300-builtin-abort-semantics.t
tests/general/300-builtin-constant-p-call.t
tests/general/300-compound-assignment-reference-lhs.t
tests/general/300-default-argument-nonstatic-member-bad.t
tests/general/300-deferred-demand-closure.t
tests/general/300-deref-function-returning-pointer-reference.t
tests/general/300-elaborated-local-struct-copy-init.t
tests/general/300-function-definition-parameter-renaming.t
tests/general/300-indirect-call-argument-conversion-bad.t
tests/general/300-local-extern-function-declaration.t
tests/general/300-member-cv-overload-identity.t
tests/general/300-member-function-pointer-return-pointer-const.t
tests/general/300-member-function-pointer-type-alias-and-function.t
tests/general/300-member-pointer-type-alias-and-function.t
tests/general/300-most-vexing-local-function-member-call-bad.t
tests/general/300-multidimensional-array-const-reference-binding.t
tests/general/300-namespace-function-body-later-anonymous-overload.t
tests/general/300-nullptr-equality.t
tests/general/300-overload-no-global-best-bad.t
tests/general/300-pointer-bool-conversion.t
tests/general/300-pointer-conversion-ranking.t
tests/general/300-pointer-to-void-drops-cv-bad.t
tests/general/300-postfix-reference-return-call-inc.t
tests/general/300-prefix-reference-return-call-inc.t
tests/general/300-qualified-direct-function-hides-using-directive.t
tests/general/300-qualified-lookup-owner-bad.t
tests/general/300-qualified-operator-prefix-identifier-call.t
tests/general/300-qualified-using-directive-function-call.t
tests/general/300-reference-binding-pointee-const-pointer.t
tests/general/300-reference-binding-ranking.t
tests/general/300-reopened-unnamed-namespace-call.t
tests/general/300-simple-assignment-reference-lhs.t
tests/general/300-static-cast-member-overload-prefers-nontemplate.t
tests/general/300-static-cast-overloaded-function-template-argument.t
tests/general/300-subscript-commuted-expression.t
tests/general/300-subscript-unscoped-enum-index.t
tests/general/300-switch-scoped-enum-condition.t
tests/general/300-unnamed-namespace-qualified-call.t
tests/general/300-unnamed-namespace-unqualified-call.t
tests/general/300-volatile-reference-overload.t
tests/general/300-zero-arg-functional-cast-alias.t
tests/spec/100-bad-no-match.t
tests/spec/100-integral-conversions.t
tests/spec/100-overload-exact.t
tests/spec/100-overload-ranking.t
tests/spec/100-pointer-qualification-conversion.t
tests/spec/100-qualified-namespace-call.t
tests/spec/100-reference-binding.t
tests/spec/100-simple-call.t
tests/spec/200-array-decay-call.t
tests/spec/200-function-pointer-call.t
tests/spec/200-if-control-flow.t
tests/spec/200-switch-statement.t
tests/spec/300-bad-ambiguous-overload.t
tests/spec/300-block-scope-namespace-alias-qualified-call.t
tests/spec/300-condition-declaration-scope.t
tests/spec/300-ellipsis-worse-than-pointer-overload.t
tests/spec/300-enumerator-is-not-null-pointer-constant-bad.t
tests/spec/300-nullptr-t-from-zero-overload.t
tests/spec/300-nullptr-t-vs-long-overload-bad.t
tests/spec/300-ranked-prefix-before-ellipsis-slot.t
```
