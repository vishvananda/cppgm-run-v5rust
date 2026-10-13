# PA8 port ledger

Stage base commit: `2bf11a2456047bffa7598407a9d32533f8f4fdd5`.
Last audited commit: INITIAL (no PA8 acceptance audit; unchanged by implementation).

## Atomic, EH and runtime instruction checkpoint — 2026-10-13

Current coherent partial handoff: **110/125 fixed cases pass; 15 fail**, down
from **24 failures** at clean entry `50b3fc7`. All **101/101** entry passing
identities survive; nine existing failures become passes, with identical
manifest hash and case identities. This supersedes the remaining lists below
and does not claim PA8 full-stage acceptance. Stage base and last-audited
commit above remain unchanged. The implementation commit contains this entry;
its hash is recorded afterward in the external `handoff.json`.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa8-runtime-20261013/`.
The previous goal turn is progress: the committed globals group reduced fixed
failures from 52 to 24. Entry state was inspected directly; no interrupted
process was assumed live or restarted. `entry.json` retains the full entry HEAD.

### Owners, completed behavior and typed data flow

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, PA8 README,
docs/rust-clone-plan.md and this ledger. Consulted frozen C++ commit
`883778fdfa45d523cf9e135d15169af976f451a8`: `lowir/text_parser_body.cpp`
value/memory/effect/control/EH grammar, `text_parser.cpp` signed target words,
`text_writer.cpp` value/atomic/EH/effect emission, `validator.cpp` operand
rules, result types, layout/targets/uses/phi and runtime validation,
`lowir_model.h` opcode/order vocabulary and `lowir/instruction.h` payloads.

- `vocabulary.rs` owns typed atomic updates, fences and MemoryOrder, preserving
  the six GNU/Clang numeric order values. C++ accepts all six on loads, stores,
  updates and success/failure orders; no extra ordering restriction is added.
- `model.rs` owns RuntimeInstruction payloads: atomic load/store/update/CAS,
  fences, va_start/va_arg, stack_alloc, exception/exception_selector, EH region
  facts, catch/filter/catch-all selectors, throw and resume. EH selectors are
  optional signed i64 values, preserving absence versus an explicit zero.
  Handler targets use BlockId and are separate from ordinary CFG successors.
  Filters use checked Arguments spans in the owning body's operand store.
  All temporary operands participate in checked incremental use counting,
  including repeated CAS address/expected uses. Results preserve C++'s i64
  selectors/CAS and ptr stack allocations independently of written types.
- New `runtime.rs` owns the related runtime grammar. It consumes the existing
  one-token borrowed lexer once, shares parser operand scratch, and records
  typed facts directly. Catch/filter symbol references preserve their distinct
  before/after-advance C++ location rules, including forward references across
  files. Selector-only filters accept both input spellings and serialize with
  the C++ writer's comma omission. `instructions.rs` now owns the shared signed
  target-word scanner formerly local to globals; addends, selectors and memory
  orders all reuse its explicit wrapping/truncation without reparsing text.
- `validator.rs` builds a dense per-function handler marker array, independent
  of ordinary predecessor sets. Handler phis reject before type/predecessor
  checks, including handlers with ordinary incoming branches. Runtime operand
  use ordering remains the common body check. va_start checks the variadic
  boundary before pointer type; va_arg checks pointer before supported scalar
  type; stack_alloc accepts integer literals or typed integer values exactly
  as C++. This tool does not enable later host-object va_arg class extensions.
  C++ adds no EH region balancing or extra atomic operand-type rule here.
- `writer.rs` streams runtime records directly from typed fields with exact
  operand contexts, number spelling, selector separators and ordering.
  Throw/resume end blocks; EH markers and fences do not. No optimizer,
  register allocator or code generator is changed. `builder.rs` adds a
  construction invariant test for repeated CAS operands and retention after
  constructing another function; there is no textual construction path.

Architecture review: one unit model, private checked compact IDs, body-owned
operands/instructions/literals and function-local validation scratch. No new
pointer graph, replay, second representation, speculative program work,
unsafe/FFI, dependency, environment read or compiler subprocess implementation.
General construction link/name census, host-object facts and later mutation
APIs remain at their consuming dependency boundaries.

### Validation and retained evidence

- `make build`: pass (`build-handoff.log`, release compilation 15.50 s).
- `make test-pa8` invokes the required
  `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa8`:
  **110 pass, 15 fail** (`stage-final.log`, `stage-final.json`).
  `preservation.json` proves **24→15** fixed failures, 101 retained passes,
  nine gained identities, zero regressions and equal manifest/coverage.
  Manifest SHA-256:
  `3cc0c62abe169d83ff54140690c5c8798d9223f005cbb44a2cb0d7f08bb84e6b`.
  `fixed-case-evidence/` snapshots every case's logs/receipts and links raw
  diagnostics/artifact mismatch directories. All 15 remaining failures reach
  explicit status 86 boundaries; ordinary rejection is not substituted.
- `make course-pa8`: fails with **107/122 specification comparisons and
  3/3 exercises** (`course-final.log`, `course-course-result.json`,
  `course-course.log`, `course-trace.jsonl`, and external
  `artifacts/clone/course/pa8/test/`). The inner keep-going make exits 0 with
  `.test_failed`; the protected course runner correctly fails the lane.
- `... clone_check.py prior pa8`: pass, all PA1–PA7 differential and course
  lanes (`prior-final.log`, `prior-summary.json`, `prior-pa1.json` through
  `prior-pa7.json`, `prior-paN-course.json`, and external
  `artifacts/clone/course/paN/test/`). Fixed counts: **58/58, 26/26, 20/20,
  106/106, 196/196, 121/121 and 214/214**.
- `make lint` (protected `clone_quality.py`): pass (`lint-final.log`), including
  rustfmt, strict all-feature Clippy, debug/release tests (191 library tests),
  supported feature configurations and source audit. `make audit-rust`: pass,
  **148 files, zero findings** (`audit-final.log`). No lint suppression added.
- `... clone_check.py preflight pa8`: pass (`preflight-final.log`).
  `... clone_check.py preserve pa8`: pass, **101/101** historical identities
  (`preserve-final.log`), supplemented by entry/final fixed preservation above.
- `probe.py`, `probes-final.log`, `probe-results.json`, `probes.trace.jsonl`:
  **111/111** protected receipts match. Coverage includes all memory orders,
  wrapped orders/widths, malformed orders/selectors, variadic boundary and
  pointer/type diagnostic priorities, slots/globals/literals, stack sizes,
  early temporary uses, result types, terminator placement, optional EH facts,
  forward symbol references and handler-phi rejection priorities.
  `probes-first.log` / `probes-first.trace.jsonl` preserve the four initial
  mismatches (filter provenance and handler phis) resolved in this group.
- `measure.py`, `scaling.log`, `scaling.trace.jsonl`, `measurements.json`:
  **6/6** scaling receipts match, three warm Rust timings per supported input,
  all exits 0 and stable output hashes. `source-hashes.json` retains compiler
  source hashes; lowir binary SHA-256:
  `1ffd6d0e3d104aa6afff832f5eb56347ba1f448e28b771621d8cfd25017a2442`.

Every supplemental comparison uses the protected adapter receipt's `match`.
These iterative LowIR paths did not exhaust native stack; no native exhaustion
cutoff or diagnostic position is calibrated. Future resource-limited driver
probes remain subject to CLONE_CONTRACT.md's protected stack qualification.
PA8 oracle-backend course execution is not a Rust backend or PA34 proof.

Measurements use wall timing and `/usr/bin/time` on the shared host while
other validation was active. They are observations, not a performance parity
or PA35 generated-code improvement claim. The atomic workload has one CAS
per count; the EH workload has one two-symbol filter per count.

| Workload | Count | Source bytes | Wall seconds | Peak RSS KiB |
| --- | --- | --- | --- | --- |
| atomic | 1,000 | 53,976 | 0.0045–0.0050 | 2,204–2,220 |
| atomic | 10,000 | 548,976 | 0.0181–0.0227 | 5,020–5,032 |
| atomic | 100,000 | 5,588,976 | 0.1643–0.1659 | 27,824–27,832 |
| EH filter | 1,000 | 22,976 | 0.0031–0.0041 | 2,148–2,204 |
| EH filter | 10,000 | 238,976 | 0.0096–0.0125 | 2,908–2,920 |
| EH filter | 100,000 | 2,488,976 | 0.0680–0.0703 | 13,468–13,480 |

### Protected accounting discrepancy and next boundary

The read-only `coverage-accounting-reproducer.py` and `coverage-accounting.json`
again show 125 manifest cases, 125 result cases and 125 Ralph targets with equal
identities, while Ralph declares 143 aggregate targets. Its prior external
stageProgress log rejects exact failure accounting with 18 unrun targets.
`preservation.json` independently proves this turn's fixed-manifest reduction.
No checker, policy, target count, fixture, reference, oracle or Ralph state is
changed. This handoff does not claim automated stageProgress acceptance;
Ralph owns the checkpoint accounting/audit.

`remaining-clusters.json` records every first-divergence receipt: **13 floating
literal boundaries and two debug-location boundaries**. The next coherent
group is floating bits and spelling shared by instructions, scalar/structured
globals and call contexts. It requires decimal/hex rounding, binary32/binary64/
x87, special/subnormal/signaling-NaN forms and exact 20/21-digit output; consult
`literals.cpp` and the shared numeric owners rather than approximating x87 with
host f64. Debug scanning/locations and statistics remain separate owners.
Host-object records, generated names and the full construction census remain
with their consuming producers. No ordinary mismatch remains in this group's
111 probes; no oracle instability was found.

Review questions: audit before/after-advance filter/catch provenance, signed
selector wrap, handler marking before phi validation, ordinary versus handler
edges, runtime result types, body operand-span lifetimes and duplicate use
counts, varargs diagnostic priority and exact C++ permissive atomic orders.

Exact remaining fixed case IDs (prefix `pa8/`):

```text
tests/spec/100-f32-global.t
tests/spec/100-f64-arith-cmp.t
tests/spec/100-function-and-instruction-debug-locations.t
tests/spec/100-signaling-nan-literal-roundtrip.t
tests/spec/100-subnormal-floating-literal.t
tests/spec/200-bad-function-debug-zero-line.t
tests/spec/200-f64-direct-call.t
tests/spec/200-f80-direct-call.t
tests/spec/200-f80-global.t
tests/spec/200-f80-unary-binary-cmp.t
tests/spec/200-float-width-conversions.t
tests/spec/200-integral-float-conversions.t
tests/spec/200-structured-float-data.t
tests/spec/300-bad-nowrap-on-float.t
tests/spec/300-instruction-facts.t
```

## Globals, structured data, aliases and TLS checkpoint — 2026-10-13

Current coherent partial handoff: **101/125 fixed cases pass; 24 fail**, down
from **52 failures** at clean entry `8d8a57c`. All **73/73** entry passing
identities survive, and 28 existing failures become passes. The fixed manifest
hash and all 125 identities are unchanged. This supersedes the older remaining
case lists below; it does not claim PA8 full-stage acceptance. Stage base and
last-audited commit above are unchanged. The implementation commit is the commit
containing this entry, recorded afterward in the external `handoff.json`.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa8-globals-20261013/`.
The previous goal turn is progress: its committed symbol/call group reduced
fixed failures from 74 to 52. Current work was inspected from the clean checkout;
no interrupted process was assumed to be live or restarted.

### Owners, typed flow and completed behavior

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the complete PA8 README,
docs/rust-clone-plan.md and prior PA8 ledger. Consulted the frozen C++ snapshot
`883778fdfa45d523cf9e135d15169af976f451a8` directly: `lowir/program.{cpp,h}`,
`text_parser.{cpp,h}` top-level/storage/data/alias/metadata/literal completion,
`text_writer.cpp` phase order/global/address/metadata output,
`validator.cpp` symbol/TLS/call checks and construction links, `literals.cpp`
integer scanner, `ir_symbol_model.h` and `lowir_model.h` vocabulary.

- `model.rs` owns private checked GlobalIds and the unresolved/function/global
  SymbolDetail enum. First-reference symbol identity/provenance remains separate
  from declaration order. A global owns its structured data vector; scalar
  integer bits, zero, symbolic address plus signed addend and declaration are
  distinct GlobalInit variants. DataItem variants retain scalar type/bits,
  symbolic address or signed zero count. Object aliases retain interned NameId
  and SymbolId relationships in order, with a separate duplicate-name index.
  A global's optional storage type distinguishes untyped structured storage
  from an explicitly written type. Signature and function-owned stores remain
  independent, and a global callee never masquerades as a direct function.
- New `globals.rs` owns global/data/alias grammar and symbolic address addends;
  `reader.rs` owns shared metadata context/placement. Each file is parsed once
  with the existing one-token borrowed lexer, then released; no original
  initializer text, replay or second semantic representation is retained.
  Compatibility `readonly`/`thread_local` words canonicalize into Storage facts;
  explicit storage metadata can supersede that compatibility word. Section
  grammar, global roles and function-only flags preserve C++ error priority.
  Alias duplicates are diagnosed before target parsing, including across files.
- `instructions.rs` exposes its existing integer parser to the owning global
  grammar. Integer data uses the same target-width scan/spelling as instructions.
  Small address addends retain C++ narrow modulo-64 scanning and explicit
  wrapping sign application, including the minimum i64 value and double minus.
  Structured positive zero counts reinterpret their u64 bits as signed i64,
  matching the C++ record and writer rather than imposing a new range rule.
- `validator.rs` visits unit symbols in first-reference order before bodies.
  Roles use typed keys; rtti_data remains non-singleton. TLS SymbolId targets
  are resolved after every file is read, checked for thread-local global storage,
  and checked once per wrapper with a dense symbol-indexed array. C++ diagnostic
  detail names and ordering are preserved. No new global alignment/span/type
  restriction is introduced: the C++ reader accepts some unusual initializers
  and does not check a structured payload against the stated storage span here.
  Undefined alias/address/TLS targets use the shared completion diagnostics.
- `writer.rs` streams four phases (global declarations, function declarations,
  global definitions, function definitions), each in declaration order, then
  aliases. Blank lines between nonempty phases, canonical storage/section/role/
  binding/object/TLS/flag order, signed addend spelling and structured item order
  match C++ bytes. All existing instruction/construction output remains exact.

Architecture review: one typed unit and stable IDs, direct indexed relations,
owned flat global data and per-function bodies/scratch, no pointer graph,
reparsing, fallback, unsafe, new dependency or environment read. This group
adds no optimizer/codegen/allocator transformation. Full-unit validation is
explicit LowIR tool work; no new speculative source-to-object hot-path work is
introduced. General constructor/link census, forward streaming producer memos
and host-object extensions remain with their consuming owners when due.

### Validation and retained evidence

- `make build`: pass (`build-final.log`; incremental release compilation 9.56 s,
  after a separate recorded build-directory lock wait).
- `make test-pa8` invokes the required
  `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa8`:
  101 pass, 24 fail (`stage-final.log`, `stage-final.json`). `preservation.json`
  proves 52→24 fixed failures, 73 retained passes, 28 exact gained identities,
  zero regressions and equal manifests/coverage. Manifest SHA-256:
  `3cc0c62abe169d83ff54140690c5c8798d9223f005cbb44a2cb0d7f08bb84e6b`.
  `fixed-case-evidence/` snapshots all per-case traces/course logs; receipts
  link the protected raw mismatch directories. All remaining cases reach 86
  at explicit missing owners, never an accepted ordinary rejection.
- `make course-pa8`: remains failed with **98/122 specification comparisons
  and 3/3 exercises**. `course-final.log`, `course-course-result.json`,
  `course-course.log`, `course-trace.jsonl` and the underlying
  `artifacts/clone/course/pa8/test/` retain evidence. Keep-going make returns 0
  but creates `.test_failed`; the protected runner correctly fails this lane.
- `... clone_check.py prior pa8`: pass, all PA1–PA7 differential and course
  lanes (`prior-final.log`, `prior-summary.json`, `prior-pa1.json` through
  `prior-pa7.json` and external `artifacts/clone/course/paN/test/`). Fixed
  counts: 58/58, 26/26, 20/20, 106/106, 196/196, 121/121 and 214/214.
- `make lint` / protected `clone_quality.py`: pass (`lint-first.log`): rustfmt,
  strict all-feature Clippy, debug/release tests (190 library tests), supported
  feature configurations and companion file audit. `make audit-rust`: pass,
  147 files, zero findings (`audit-final.log`). No lint suppression added.
- `... clone_check.py preflight pa8`: pass (`preflight-final.log`).
  `... clone_check.py preserve pa8`: pass, **73/73** historical identities
  (`preserve-final.log`), complemented by full entry/final preservation evidence.
- `probe.py`, `probes-final.log`, `probe-results.json`, `probes.trace.jsonl`:
  **102/102** protected adapter receipts match, including cross-file storage,
  all output phases, typed/untyped structured data, C++ permissive initializers,
  symbol/function collisions, global metadata contexts, repeated roles,
  alias diagnostics, TLS target/wrapper priority, zero-count boundary values,
  signed/wrapped address addends and indirect calls through global symbols.
- `measure.py`, `scaling.log`, `scaling.trace.jsonl`, `measurements.json`:
  **6/6** supported scaling receipts match; three warm candidate timings per
  workload, every exit 0 with stable output hashes. `source-hashes.json`
  records reviewed Rust files. Candidate lowir SHA-256:
  `599865b2666de3d10004f3c1c91246043811f1777d2625f36bfa3dc8bcf5a30c`.

These iterative LowIR probes do not exhaust native stack. Every comparison
uses the protected adapter receipt's `match`; any native stack qualification
remains the protected adapter's responsibility. No C++ resource cutoff or
exhaustion diagnostic position is calibrated. PA8 oracle-backend course
execution is frontend/model evidence, not Rust backend or PA34 proof.

Measurements use `/usr/bin/time` and wall timing on the shared host; earlier
validation was also active. These are observations, not a paired performance
comparison or a PA35 improvement target. The data workload has three items per
count (i64, symbolic ptr with addend, zero); the relation workload has one TLS
global, wrapper declaration and object alias per count.

| Workload | Count | Source bytes | Wall seconds | Peak RSS KiB |
| --- | --- | --- | --- | --- |
| data | 1,000 | 35,809 | 0.0040–0.0068 | 2,192–2,196 |
| data | 10,000 | 377,809 | 0.0097–0.0124 | 3,596–3,620 |
| data | 100,000 | 3,977,809 | 0.0697–0.0730 | 15,572–15,584 |
| relations | 1,000 | 118,340 | 0.0075–0.0088 | 2,900 |
| relations | 10,000 | 1,243,340 | 0.0453–0.0565 | 12,756 |
| relations | 100,000 | 13,033,340 | 0.4803–0.5015 | 102,536–102,688 |

Storage remains 28 GiB used of 98 GiB, below the 100 GiB ceiling.

### Protected coverage accounting defect

The read-only `coverage-accounting-reproducer.py` and its JSON still show
125 manifest cases, 125 result cases and 125 Ralph targets with equal identities,
while Ralph reports 143 aggregate targets and 18 unrun. Its external
`checks/last-stageProgress.log` therefore rejects exact failure accounting,
although entry/final fixed-manifest reduction is demonstrated above. No checker,
policy, fixture, oracle, reference, Ralph state or target total is changed.
This partial handoff does not claim automated stageProgress acceptance; Ralph
owns its correction/checkpoint audit. The prior ledger's reproducer description
remains applicable with the current 101/125 result.

### Next dependency boundary and exact remaining cases

`remaining-clusters.json` groups the 24 current fixed failures by earliest
missing owner: floating literals 13; atomic rvalues 5; EH/fence effects 4;
debug locations 2. These are first boundaries, not every downstream feature.
The next coherent foundation is **floating literal bits and spelling** shared
by instructions, scalar/structured globals and call operands: decimal/hex
rounding, binary32/binary64/x87, special/subnormal/signaling-NaN spellings,
context conversion and C++ 20/21-digit output rules. Consult `literals.cpp`
and reuse/extend the owning numeric algorithms without host f64 approximations
for x87. Then translate atomic/varargs/dynamic-stack/EH records, debug scanning/
locations, stats and the full remaining construction checks. Host-object facts,
frontend forward memos and generated/inlined names belong to later consumers.

Review questions: verify optional global storage types and C++ initializer
permissiveness, signed zero/addend arithmetic, SymbolDetail direct/indirect
call demand, first-use versus phase emission order, cross-file alias/TLS
ownership and all metadata error-priority paths. Ensure future numeric work
records bits once and makes both body/global writers consume typed literals.

Exact remaining fixed case IDs (prefix `pa8/`):

```text
tests/spec/100-eh-cleanup-resume.t
tests/spec/100-eh-end-normal.t
tests/spec/100-eh-same-function-catch.t
tests/spec/100-f32-global.t
tests/spec/100-f64-arith-cmp.t
tests/spec/100-function-and-instruction-debug-locations.t
tests/spec/100-signaling-nan-literal-roundtrip.t
tests/spec/100-subnormal-floating-literal.t
tests/spec/200-atomic-add-fetch.t
tests/spec/200-atomic-compare-exchange-failure.t
tests/spec/200-atomic-compare-exchange-success.t
tests/spec/200-atomic-exchange.t
tests/spec/200-atomic-fences.t
tests/spec/200-atomic-load-store.t
tests/spec/200-bad-function-debug-zero-line.t
tests/spec/200-f64-direct-call.t
tests/spec/200-f80-direct-call.t
tests/spec/200-f80-global.t
tests/spec/200-f80-unary-binary-cmp.t
tests/spec/200-float-width-conversions.t
tests/spec/200-integral-float-conversions.t
tests/spec/200-structured-float-data.t
tests/spec/300-bad-nowrap-on-float.t
tests/spec/300-instruction-facts.t
```

## Symbol/call boundary and construction checkpoint — 2026-10-13

Current coherent partial handoff: **73/125 fixed cases pass; 52 fail**, down
from **74 failures** at the turn's clean entry (`4fe8f03`). The 125 case identities
and fixed manifest hash remain unchanged. Every prior passing identity is
retained (**51/51**); 22 existing failures become passes. This is implementation
progress, not PA8 full-stage acceptance. The older core checkpoint below is
retained as historical evidence; its remaining-case list is superseded here.
Stage base and last-audited commit above are unchanged. The implementation
commit is the commit containing this entry; its hash is recorded in the
external `handoff.json`, without changing Ralph's audit state.

Artifact root:
`/home/vishvananda/work/private/v5rust/artifacts/pa8-call-boundaries-20261013/`.
The prior goal turn is classified as progress: the authoritative entry commit
implements the integer core and its recorded fixed-manifest gains; no live
process from that work was needed to resume this group.

### Owners, completed behavior and architecture review

Read the required repository/assignment documents and consult the frozen
C++ snapshot directly. This group translates `lowir/program.{cpp,h}` symbol
identity and boundary lookup; `text_parser.{cpp,h}` signatures/classes and
completion; `text_parser_body.cpp` symbol values, call grammar and deferred
block endings; `validator.{cpp,h}` symbol-order, layout and call checks;
`text_writer.cpp` call/parameter/metadata output; and
`builder.{cpp,h}`, `exercises.{cpp,h}` construction. Also reviewed
`ir_symbol_model.h`, `lowir_model.h`, `lowir/types.h`, `instruction.h`,
`local_names.cpp`, and `dev/lowir.cpp`. C++ sources, generated answers,
fixtures, oracle binaries and protected policy remain unchanged.

- `model.rs`: private checked SymbolId, SignatureId and SourceId stores;
  unresolved versus declared/defined function identity; first-reference
  symbol order independent of definition/declaration emission order. Each
  symbol retains compact first-use source/line provenance. Signature-owned
  parameters/result/boundary facts are separate from symbol metadata.
  Functions reference signatures by ID; direct/explicit call demand reads
  those records directly. Arguments are checked slices in each owning body's
  operand vector; append counts callee and argument temporary uses.
- `reader.rs` / `instructions.rs`: parse each file once, retain one current
  token, intern spellings, reuse argument scratch, and append typed operands.
  Shared signature grammar accepts explicit indirect boundaries with exact
  metadata context/placement and introductory object ABI classes/result
  classes. Call elision grammar preserves duplicate/placement diagnostics.
  The parser allows a final call provisionally; the validator resolves its
  noreturn boundary after all files have been read.
- `validator.rs`: traverse bodies in first-reference symbol order, preserving
  C++ diagnostic priority independently of emission order. Calls check exact
  result/arity, fixed direct slot types versus storage passing, and direct
  void copy-elision requirements. The C++ owner's permissive temporary and
  callee operand rules are retained, with no additional type rule. Shared
  phi handling now covers symbol operands. Construction audits stored result
  links/types before shared semantic/layout validation.
- `writer.rs`: stream explicit signatures, parameter and boundary metadata,
  object classes, calls and elision facts directly from typed records. Call
  integer operands retain their untyped 128-bit spelling rather than being
  prematurely narrowed to parameter widths; no optimizer transformation is
  introduced. All three exercise outputs match C++ bytes exactly.
- `builder.rs` / `exercises.rs`: demand-sized typed construction over the same
  stores, with checked parameter/block/temp uniqueness and incremental use
  counts. Port the C++ sum closed form, load-before-store alias-safe swap,
  and two ordered indirect calls. The C++ recipes use explicit plain names;
  general generated/inlined-name collision handling remains a later consumer
  boundary. No textual bodies, token replay, cached reference answers or
  alternative construction model are used.
- `mod.rs` / `driver/lowir.rs`: construction uses the shared model/writer and
  is audited before writing. An invalid constructed exercise is distinguished
  as internal status 70; unfinished features stay status 86. No dependency,
  unsafe/FFI, environment read, fallback compiler or backend change is added.

Each call's operands and every function's validation scratch remain local to
that function. The program owns only cross-function names/symbols/signatures,
file provenance and declaration order. No call validation scans unrelated
functions. Ordinary text reader/writer validation is explicit tool work;
there is no new whole-program scan on a source-to-object production path.
The optimizer, instruction selector and allocator are unaffected.

### Validation and raw evidence

- `make build`: pass (`build-final.log`, incremental release build 15.43 s).
- `make test-pa8` invokes the required
  `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa8`:
  all 125 fixed cases execute, 73 pass, 52 fail (`stage-final.log`,
  `stage-final.json`). `preservation.json` proves equal identity sets/hash,
  51 retained passes, 22 gains, zero regressions and failure reduction 74→52.
  JSON entries link each protected raw trace and course log; mismatch records
  link retained raw streams/artifacts. Manifest SHA-256 remains
  `3cc0c62abe169d83ff54140690c5c8798d9223f005cbb44a2cb0d7f08bb84e6b`.
- `make course-pa8`: required full course run remains failed, with
  **70/122 specification comparisons and 3/3 exercises**. Its underlying
  keep-going make returns 0 but creates `.test_failed`; the protected runner
  correctly reports failure (`course-final.log`,
  `artifacts/clone/course/pa8/test/course-result.json`, `course.log` and
  `trace.jsonl` under the external artifact root). No exception is requested.
- `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa8`:
  pass, all PA1–PA7 differential and course lanes (`prior-final.log`,
  `prior-summary.json`, external `artifacts/clone/results/pa1.json` through
  `pa7.json` and `artifacts/clone/course/paN/test/`). Differential counts are
  58/58, 26/26, 20/20, 106/106, 196/196, 121/121 and 214/214.
- `make lint` (protected `clone_quality.py`): pass (`lint-final.log`);
  rustfmt, all-feature strict Clippy, debug/release tests (190 library tests,
  including two construction/operand ownership tests) and companion audit.
  `make audit-rust`: pass, 146 files, zero findings (`audit-final.log`). No
  lint expectation/suppression is added.
- `... clone_check.py preflight pa8`: pass (`preflight-final.log`).
  `... clone_check.py preserve pa8`: pass, **51/51** historical required
  identities (`preserve-final.log`). The independent final-result comparison
  additionally proves preservation against the actual turn-entry JSON.
- `probe.py`, `probes-final.log`, `probe-results.json`, `probes.trace.jsonl`:
  **64/64** protected adapter receipts match. Cover cross-file forward calls
  and missing symbols, duplicate declarations, signature/slot/arity/elision
  diagnostics, symbol validation priority, permissive C++ operands, noreturn
  endings, object classes and all three exact exercise outputs. `measure.py`
  and `scaling.trace.jsonl` add **6/6** matching scaling receipts.

All supplemental comparisons inspect the protected receipt's `match`, retaining
raw streams/output and any `stack_qualification` proof. These iterative LowIR
probes do not exhaust native stack; no C++ frame cutoff or diagnostic location
is calibrated. Prior-stage checks also use the protected adapter. This is
PA8 text/typed construction and oracle-backend behavior evidence, not a claim
that Rust native code generation or PA34 fixed-point equality is complete.

Candidate lowir SHA-256:
`654751a79f5be4e4b123153d336338e779d875ddc919e0bdcac26032e358dd91`.
`source-hashes.json` records the reviewed compiler sources.

Measurements run after the gates finished, three warm candidate invocations
per fixed generated input, using `/usr/bin/time` and wall-clock timing. The
call workload is one function with a direct-call chain; the function workload
is a forward-reference cycle of individually owned small functions. Every
output matches the frozen writer through the protected adapter.

| Workload | Count | Source bytes | Wall seconds | Peak RSS KiB |
| --- | --- | --- | --- | --- |
| calls | 1,000 | 27,901 | 0.0041–0.0044 | 2,184–2,192 |
| calls | 10,000 | 297,903 | 0.0146–0.0174 | 4,996–5,004 |
| calls | 100,000 | 3,177,905 | 0.1164–0.1214 | 26,596–26,600 |
| functions | 100 | 8,880 | 0.0029–0.0042 | 2,184–2,184 |
| functions | 1,000 | 90,780 | 0.0068–0.0086 | 3,592–3,604 |
| functions | 10,000 | 927,780 | 0.0406–0.0414 | 19,080–19,080 |

`measurements.json` retains all runs and binary hash. These observations show
supported LowIR processing/scaling costs; they impose no C++ runtime equality
or PA35 generated-code improvement target. Shared storage was 28 GiB used of
98 GiB at handoff preparation, below the 100 GiB ceiling.

### Coverage accounting defect (read-only reproducer)

Ralph's last-stageProgress log rejects exact counts because it records
**143** targets and **18 unrun**, while the fixed manifest, final result JSON
and Ralph's own target list each contain exactly **125**, with equal identities.
`coverage-accounting-reproducer.py` reads those authoritative files without
mutation; run it with Python. `coverage-accounting-reproducer.json` records
125/125/143/125 and equal target sets. The same discrepancy existed at entry.
The external log remains
`/home/vishvananda/work/.ralph/v5rust-gpt-6.1-sol-high/checks/last-stageProgress.log`.
No state, manifest, acceptance policy, test coverage or aggregate total is
changed. Actual fixed-manifest failure reduction is proven independently;
Ralph owns correction of the aggregate and checkpoint acceptance. This
handoff does not claim that its automated stageProgress check is green.

### Next dependency boundary and exact remaining cases

All **52** final failing cases reach status 86 at an explicit missing owner;
there is no ordinary exit/diagnostic/output mismatch left in this group's
fixed cases. `remaining-clusters.json` records first divergent boundaries:
globals/object aliases 36; floating literals 9; effect records 3; debug
locations 2; TLS/symbol relations 2. These are earliest-boundary clusters,
not a complete census of every feature downstream in those cases.

The next coherent group is unit globals/data/aliases and their symbol/TLS
metadata: extend symbol kind/detail identity with typed global records,
scalar/structured zero/data/address initializers, aliases and final target
checks, preserve emission phases/order, and validate storage/TLS wrapper
relations. Missing floating literal parsing/rounding/spelling (including
integer operands demanding a floating call parameter's serialization) remains
an explicit 86 boundary. Then complete atomics/varargs/dynamic-stack/EH,
debug records, stats and later constructed field checks. Retained
frontend-forward boundary memos, host-object extensions and generated/inlined
names belong to their actual consuming producers rather than speculative
hot-path work here.

Audit questions: review the checked body-owned argument spans and incremental
uses, final direct/explicit boundary selection, first-reference validation
versus declaration emission order, and distinct metadata contexts. Verify the
C++ slot and permissive temporary rules before strengthening anything. Confirm
construction never obtains its bodies from text; its algorithm/order/plain
names match the consulted C++ owner.

Exact remaining fixed case IDs (prefix `pa8/`):

```text
tests/spec/100-copyobj.t
tests/spec/100-eh-cleanup-resume.t
tests/spec/100-eh-end-normal.t
tests/spec/100-eh-same-function-catch.t
tests/spec/100-f32-global.t
tests/spec/100-f64-arith-cmp.t
tests/spec/100-function-and-instruction-debug-locations.t
tests/spec/100-global-variable.t
tests/spec/100-legacy-runtime-hooks.t
tests/spec/100-object-abi-lowered.t
tests/spec/100-signaling-nan-literal-roundtrip.t
tests/spec/100-startup-shutdown-hooks.t
tests/spec/100-structured-global-data.t
tests/spec/100-subnormal-floating-literal.t
tests/spec/100-zeroinit.t
tests/spec/200-atomic-add-fetch.t
tests/spec/200-atomic-compare-exchange-failure.t
tests/spec/200-atomic-compare-exchange-success.t
tests/spec/200-atomic-exchange.t
tests/spec/200-atomic-fences.t
tests/spec/200-atomic-load-store.t
tests/spec/200-bad-duplicate-object-alias.t
tests/spec/200-bad-duplicate-tls-wrapper.t
tests/spec/200-bad-function-debug-zero-line.t
tests/spec/200-bad-storage-op-alignment.t
tests/spec/200-bad-tls-for-non-thread-local-global.t
tests/spec/200-bad-undefined-object-alias-target.t
tests/spec/200-bad-unknown-global-storage-metadata.t
tests/spec/200-binding-metadata-smoke.t
tests/spec/200-direct-object-return-boundary-smoke.t
tests/spec/200-f64-direct-call.t
tests/spec/200-f80-direct-call.t
tests/spec/200-f80-global.t
tests/spec/200-f80-unary-binary-cmp.t
tests/spec/200-float-width-conversions.t
tests/spec/200-global-pointer-address-init.t
tests/spec/200-global-storage-metadata-smoke.t
tests/spec/200-integral-float-conversions.t
tests/spec/200-linkage-metadata-smoke.t
tests/spec/200-structured-float-data.t
tests/spec/200-structured-global-zero-padding.t
tests/spec/200-thread-local-global-storage-metadata-smoke.t
tests/spec/200-thread-local-wrapper-metadata-smoke.t
tests/spec/200-unused-declarations.t
tests/spec/300-bad-copy-elision-indirect-callee.t
tests/spec/300-bad-global-section-unsafe-name.t
tests/spec/300-bad-nowrap-on-float.t
tests/spec/300-bad-replaceable-without-weak.t
tests/spec/300-instruction-facts.t
tests/spec/300-object-abi-metadata.t
tests/spec/300-object-alias-smoke.t
tests/spec/300-symbol-object-metadata-smoke.t
```

## Integer/function core checkpoint — 2026-10-13

This is a coherent **partial PA8 implementation**, not full-stage acceptance.
The fixed differential manifest now passes **51/125** cases, with **74 failures**,
versus zero passes and 125 failures at entry. No manifest or coverage change.
The initial core's 48 passing identities are all retained. Ralph reported a
143-target aggregate at entry; the inspected differential
manifest contains 125 cases. This entry records the full 125-case command
and does not claim broader aggregate acceptance. The clean entry is PA7's
acceptance commit (`2bf11a2`); PA8 had only its status-86 scaffold. This
continuation implements and validates the next stage's function core.

The implementation commit is the commit containing this entry; its hash is
recorded after commit in `handoff.json` below. Implementation does not advance
Ralph's last-audited commit or stage state.

Artifact root (all generated files remain outside the checkout):
`/home/vishvananda/work/private/v5rust/artifacts/pa8-function-core-20261013/`.

## Ownership and typed flow

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, docs/rust-clone-plan.md, the
complete PA8 README and prior ledger. Consulted frozen C++
`dev/lowir.cpp`, `lowir_model.h`, `ir_symbol_model.h`,
`lowir/{text_lexer,text_parser,text_parser_body,text_reader,text_writer,
validator,builder,program,name_table,local_names,literals}.{cpp,h}` where
applicable, plus `types.h`, `instruction.h`, and the LowIR tool help owner.
The snapshot remains commit `883778fdfa45d523cf9e135d15169af976f451a8`.

Rust responsibilities:

- `dev/bin/lowir.rs` collects argv; `dev/src/driver/lowir.rs` owns invocation,
  status and diagnostic rendering. Statistics and exercise construction remain
  explicit 86 boundaries; help and batch-scaffold behavior match C++.
- `lowir/mod.rs` owns bulk file reads, file-order unit lifetime, one validation
  and output I/O. `lexer.rs` borrows one immutable input buffer and keeps one
  current token. The buffer is released between files; retained names are
  interned by the unit's private `Names` API.
- `vocabulary.rs` owns typed scalar/object types and operation/metadata
  vocabulary. Its small macros declare paired vocabulary enums/spellings and
  ID wrappers only; they contain no parser, validator or emission algorithms.
- `model.rs` owns unit order/index, typed signatures/parameters/metadata and
  function-local stores. Name, function, temporary, slot, block, instruction
  and literal IDs are distinct u32 identities with checked allocation.
  Each body owns its literal pool, instruction store, block layout and lookup
  indexes; operand visits increment checked temporary use counts at append.
  No pointer graph, text transport, second tree or token replay is introduced.
- `reader.rs` owns signatures, parameter/function facts, types and body/local
  declaration ordering. `instructions.rs` builds typed instruction payloads
  once, including forward local identities, exact ordered fact grammar and
  wrapping target integer scanning. Structured Results retain source/line
  diagnostics; unsupported features remain distinct from normal rejection.
- `validator.rs` consumes those stored facts. It builds function-local arrays
  for layout positions and deduplicated ordinary predecessors, then performs
  the exact C++ checks. It deliberately does not strengthen C++'s operand
  rules. `writer.rs` reads typed IDs/facts directly and streams through a
  buffered sink, preserving phase/order/spacing and target-width literal text.

## Completed behavior group and production invariants

The group covers ordinary named function declarations/definitions and
helper-only units; parameters and boundary/symbol metadata not requiring
other symbols or object classes; scalar/object-storage types; slots; forward
local temp/block identity; duplicate and undefined local checks; block layout,
terminator placement and instruction line ownership; constants/copies,
integer unary/binary/comparison/conversion operations, select, phi, jump,
branch, switch, return and unreachable; slot addresses, scalar load/store,
index projections, copyobj and zeroinit spans.

Integer literals scan decimal/octal/hex modulo 128 bits and use C++'s narrow
64-bit sign extension outside i128/untyped contexts. Serialization truncates
at the operand width and prints signed/unsigned values exactly. The positive
integer grammar preserves C++'s conservative overflow test, including its
rejection of the largest otherwise representable u64 decimal spellings.

Comparisons produce i64. Ordinary temporary uses follow layout order rather
than a new dominance rule; phi forward uses are allowed, with exact placement,
predecessor uniqueness/completeness and incoming-type checks. Instruction
facts retain C++'s grammar order and placement restrictions. Function roles
retain legacy-entry demand and singleton validation. Explicit spellings are
preserved; generated-name collision handling belongs with construction.

Architecture review: one shared model, per-function stores and validation
scratch, no optimizer/codegen transformation, no environmental reads in phase
code, no external compiler implementation calls, no unsafe/FFI or new
libraries. The optimizer/backend owners are unaffected by this group.
General mutation/construction APIs will extend the same private stores when
those consumers are due; there is no textual rebuild path.

## Validation evidence

- `make build`: pass (`build-final.log`).
- `make test-pa8`, equivalently
  `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa8`:
  full fixed manifest executed; 51 pass, 74 fail. `stage-handoff.log` and
  `stage-result.json` retain exact identities, binary hashes and raw trace paths.
  The fixed manifest SHA-256 is
  `3cc0c62abe169d83ff54140690c5c8798d9223f005cbb44a2cb0d7f08bb84e6b`.
- `make course-pa8`: still fails at the documented missing groups;
  `course-handoff.log` and
  `/home/vishvananda/work/private/v5rust/artifacts/clone/course/pa8/test/`.
  It is required for full acceptance and is not waived by this checkpoint.
- `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa8`:
  PA1–PA7 differential and course lanes pass (`prior-handoff.log`;
  `artifacts/clone/results/pa1.json` through `pa7.json` and corresponding
  `artifacts/clone/course/paN/test/` under the external artifact root).
- `make lint`: pass, including rustfmt, strict Clippy, debug/release Rust
  tests, supported telemetry feature configurations and the source audit
  (`lint-final.log`). `make audit-rust`: pass, 144 files, zero findings
  (`audit-final.log`). No lint suppressions added.
- `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa8`:
  pass (`preflight.log`). `... clone_check.py preserve pa8`: pass, 0/0
  historical required PA8 identities (`preserve.log`). `preservation.json`
  additionally proves all 48 initial-core passing case identities survive.
- Supplemental generated workloads: `probe.py`, `probes-handoff.trace.jsonl`,
  `probes-handoff.log`, and `measurements.json`. All nine protected-adapter
  receipts match (wrapping/narrow integer values, cyclic forward phi, bulk
  memory, early-use rejection, positive overflow, vertical-tab whitespace,
  and three scaling inputs).
- Malformed ordinary grammar/metadata probes: `grammar-probe.py`,
  `grammar-final-binary.trace.jsonl`, `grammar-final-binary.log`; all 25
  receipts match, including missing literal/value diagnostics, unknown
  operations, multiline metadata key errors, role/value constraints and
  retained output files on rejection. Initial mismatches and their raw
  outputs remain in the artifact root for source-review evidence.

These iterative LowIR paths did not trigger native stack exhaustion. Every
supplemental comparison uses the protected adapter and its receipt `match`;
no native cutoff or diagnostic-position calibration was performed. Future
resource-limited C++ driver probes must use stack qualification as the clone
contract specifies. PA34 object/tool fixed-point proof remains a later gate.

Candidate lowir SHA-256: `1987f31b5edd0f375ae89a845de2de9e729b6f5b96dd36ec6da292c09d89dbf7`.
`source-hashes.json` records the reviewed compiler source hashes.

Supported hot path: one named function with 1,000 / 10,000 / 100,000 binary
operations (32,865 / 347,867 / 3,677,869 source bytes), three warm repetitions
using `/usr/bin/time` plus wall-clock timing. Final observations:

| Operations | Wall seconds | Peak RSS KiB |
| --- | --- | --- |
| 1,000 | 0.0035–0.0038 | 2,192–2,216 |
| 10,000 | 0.0134–0.0144 | 5,012–5,056 |
| 100,000 | 0.1176–0.1393 | 26,696–27,128 |

All three scaling outputs match the frozen reader byte-for-byte. These are
Rust throughput/RSS observations, not a calibrated performance comparison
or a PA35 code-generation improvement claim. `measurements.json` retains
all runs and the measured binary hash.

## Next dependency boundary and remaining groups

All 74 remaining per-case failures reach status 86, with no unresolved
ordinary diagnostic/output mismatch in the implemented core. Earliest
missing boundaries in the actual traces: globals/aliases 35; floating
literals 9; calls/atomic/exception rvalues 8; effect instructions 6;
symbol operands 3; symbol/object boundary metadata 4; object ABI classes 4;
debug locations 2; construction exercises 3. These are first-boundary
clusters, not a completeness census of everything each case eventually needs.

The next coherent group is **unit symbol identity and call boundaries**:
add unresolved/declared/defined symbol records in file order, direct and
explicitly signed indirect calls, call arguments and result/arity checks,
slot passing and copy-elision validation, noreturn block-ending demand and
shared construction APIs/generated names for all three exercises. This
extends the current typed core and unlocks otherwise implemented loops,
select and metadata cases without reparsing text. Then complete globals/data,
floating literals (including f80 and special bits), object ABI and TLS facts,
atomic/varargs/dynamic-stack/EH records, debug tokens/locations, aliases,
stats and remaining CLI/constructed-validation details. Some of these
cross symbol and signature boundaries and must be translated together.

Independent audit questions: verify retained-body ownership and checked use
counts; compare operand permissiveness with the C++ validator, including
phi forward uses; review literal width/sign/overflow behavior and byte writer
order; verify next-group symbol demand/ordering before enabling calls;
review generated-name and constructed-program validation when exercises land.
No oracle instability or harness defect was found or policy changed.

Exact remaining fixed case IDs (prefix `pa8/`):

```text
tests/behavior/100-sum.t
tests/behavior/200-swap.t
tests/behavior/300-call.t
tests/spec/100-copyobj.t
tests/spec/100-eh-cleanup-resume.t
tests/spec/100-eh-end-normal.t
tests/spec/100-eh-same-function-catch.t
tests/spec/100-f32-global.t
tests/spec/100-f64-arith-cmp.t
tests/spec/100-function-and-instruction-debug-locations.t
tests/spec/100-global-variable.t
tests/spec/100-indirect-call.t
tests/spec/100-legacy-runtime-hooks.t
tests/spec/100-object-abi-lowered.t
tests/spec/100-signaling-nan-literal-roundtrip.t
tests/spec/100-simple-call.t
tests/spec/100-small-direct-object-argument.t
tests/spec/100-startup-shutdown-hooks.t
tests/spec/100-structured-global-data.t
tests/spec/100-subnormal-floating-literal.t
tests/spec/100-while-break.t
tests/spec/100-zeroinit.t
tests/spec/200-atomic-add-fetch.t
tests/spec/200-atomic-compare-exchange-failure.t
tests/spec/200-atomic-compare-exchange-success.t
tests/spec/200-atomic-exchange.t
tests/spec/200-atomic-fences.t
tests/spec/200-atomic-load-store.t
tests/spec/200-bad-call-signature-symbol-metadata.t
tests/spec/200-bad-duplicate-object-alias.t
tests/spec/200-bad-duplicate-tls-wrapper.t
tests/spec/200-bad-function-debug-zero-line.t
tests/spec/200-bad-missing-indirect-call-signature.t
tests/spec/200-bad-storage-op-alignment.t
tests/spec/200-bad-tls-for-non-thread-local-global.t
tests/spec/200-bad-undefined-object-alias-target.t
tests/spec/200-bad-unknown-global-storage-metadata.t
tests/spec/200-binding-metadata-smoke.t
tests/spec/200-direct-object-return-boundary-smoke.t
tests/spec/200-f64-direct-call.t
tests/spec/200-f80-direct-call.t
tests/spec/200-f80-global.t
tests/spec/200-f80-unary-binary-cmp.t
tests/spec/200-float-width-conversions.t
tests/spec/200-function-effect-metadata-smoke.t
tests/spec/200-global-pointer-address-init.t
tests/spec/200-global-storage-metadata-smoke.t
tests/spec/200-indirect-call-signature.t
tests/spec/200-integral-float-conversions.t
tests/spec/200-linkage-metadata-smoke.t
tests/spec/200-parameter-alias-metadata-smoke.t
tests/spec/200-structured-float-data.t
tests/spec/200-structured-global-zero-padding.t
tests/spec/200-thread-local-global-storage-metadata-smoke.t
tests/spec/200-thread-local-wrapper-metadata-smoke.t
tests/spec/200-unused-declarations.t
tests/spec/200-variadic-function-arity.t
tests/spec/300-bad-classes-eightbyte-count.t
tests/spec/300-bad-classes-on-scalar.t
tests/spec/300-bad-classes-unknown.t
tests/spec/300-bad-copy-elision-arity.t
tests/spec/300-bad-copy-elision-indirect-callee.t
tests/spec/300-bad-copy-elision-nonvoid-call.t
tests/spec/300-bad-global-section-unsafe-name.t
tests/spec/300-bad-nowrap-on-float.t
tests/spec/300-bad-replaceable-without-weak.t
tests/spec/300-bad-result-classes-memory.t
tests/spec/300-bad-result-classes-scalar.t
tests/spec/300-instruction-facts.t
tests/spec/300-object-abi-metadata.t
tests/spec/300-object-alias-smoke.t
tests/spec/300-select.t
tests/spec/300-symbol-object-metadata-smoke.t
tests/spec/400-bad-slot-argument-type.t
```
