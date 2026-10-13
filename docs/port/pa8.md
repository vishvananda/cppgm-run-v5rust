# PA8 port ledger

Stage base commit: `2bf11a2456047bffa7598407a9d32533f8f4fdd5`.
Last audited commit: INITIAL (no PA8 acceptance audit; unchanged by implementation).

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
