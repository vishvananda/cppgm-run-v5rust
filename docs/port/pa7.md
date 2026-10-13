# PA7 port ledger

Stage base commit: `9a851dcedb1e56bbc6585c84c5990f6ef1a1c372`.
Current implementation group base: `37c435809aa28ae021003fed46b5ef8c2efda824`.
Last audited commit: INITIAL (unchanged; Ralph owns checkpoint/final audit and advancement).

## Current checkpoint

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

## Exact remaining fixed identities

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
