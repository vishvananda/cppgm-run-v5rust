# PA9 port ledger

Stage base commit: `867f7a05e3a646f89773bd11ef7194fdb7f01056`.
Last audited commit: INITIAL (implementation does not advance this field).

## Local function scopes, closures and substitution checkpoint — 2026-10-13

Entry HEAD: `b7157ce279dc11279447508f2c2d22178c326f18`, clean checkout.
The previous goal turn was **progress**: its committed dependent-expression
translation established the verified 108/119 baseline. No interrupted
compiler/checker process was live at entry. Stage base and last-audited commit
above remain unchanged; implementation does not advance audit state.

This coherent local-context group moves the unchanged fixed manifest from
**108/119 to 119/119**, reducing failures **11 → 0** and preserving **108/108**
previously passing identities. The owning course suite also passes 119/119.
This is an implementation checkpoint, not full PA9 acceptance: statistics
and later compiler-producer interfaces still need translation and review.
Ralph owns checkpoint/final audits and stage advancement.

### C++ owners, typed flow and architecture review

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the owning PA9 handout,
rust-clone-plan.md and the stage ledger. Consulted the frozen production
implementation directly in `/home/vishvananda/work/references/v5rust-cpp/`:
`dev/src/abi/facts/abi_fact_definitions.cpp` (`read_context_definition`,
`local_entity`, `closure_entity`, generated-lambda recognition),
`abi_fact_reader.cpp`/`abi_fact_case.h` (function targets, context records,
terminal selection and function-entity construction), and
`dev/src/abi/itanium/abi_graph.{h,cpp}`/`abi_encoder.{h,cpp}` (scope/name/closure
identity, contexts, local/nested names, whole-entity discriminators, prefix
sharing and closure signatures). Protected raw traces showed all eleven
failures stopped at a reader status-86 path, before encoding.

| C++ responsibility | Rust owner and invariant |
| --- | --- |
| Context definitions and local/closure type facts | `abi/fact_contexts.rs`, `abi/facts.rs`, `abi/fact_types.rs`: one parse at the explicit tool boundary; function and opaque raw scopes enter the existing Unstarted/InProgress/Complete resolution, including forward binders and cycles. Local occurrences are validated indices; closure number and ordered signature facts remain typed. A sole void parameter is normalized away. |
| Local/lambda function targets and component records | `abi/fact_functions.rs`: C++ arity, terminal-before-context validation and diagnostic order; structured context-kind/name-component restrictions; default call terminal only for a `$_` name followed entirely by decimal digits. Ordinary function qualifiers, parameters, results, tags and template arguments reuse the existing builders. |
| Canonical scope, local-name and closure ownership | `abi/graph.rs`: immutable function/raw scope nodes, default-argument identity, local occurrence, closure number/variadic bit and graph-owned ordered signature spans. Canonical equality and substitution use compact IDs; neither binder spelling nor a rendered context identifies a semantic fact. Tagging preserves occurrence fields. No owning recursive edges or recursive drop. |
| Local/context/nested-name emission | `abi/encoder.rs`: nested function encodings share the enclosing name's dictionary. Function scopes never become prefix substitution candidates. Local discriminators follow the entire entity/member terminal; short and long forms split at number 10. Known prefixes permit ordinary nested names. Prefix/type chains remain iterative; closure parameter types participate in exact substitution order. Default-argument and variadic-closure typed fields are encoded directly. The C++ decltype prefix-root production is also translated. |

`fact_contexts.rs` owns the local scope/type fact responsibility, rather than
splitting the function reader mechanically. `source-map.tsv` records all
translated standalone fact productions and keeps statistics/producer APIs
partial. The graph still owns flat node/text/edge stores, private compact IDs
and checked spans. Case resolution and encoding scratch die at their original
boundaries; no text transport, semantic reparse, global scan, new dependency,
unsafe/FFI code, compiler fallback or generated-code optimization was added.
The new ownership test checks canonical scope identity, default-argument and
occurrence distinctions, closure signature stability after graph growth, and
variadic/default-argument emission. Source limits and architecture review
remain part of the companion audit.

A genuine stack byte budget also guards recursive context encoding. No probe
calibrates a C++ exhaustion cutoff or diagnostic position. Supported nested
context probes (8/32/128 levels) all succeed and match. Resource-limited
source-driver probes remain governed by the protected adapter's `match` and
`stack_qualification` receipt. The supported ABI probes here did not exhaust
native stack and required no qualification.

Optimizer, register allocator, backend and source-driver implementations were
untouched. PA9 produces ABI names; these checks make no object-byte or PA34
bootstrap claim. PA35 code-generation improvement targets remain out of scope.

### Validation and retained evidence

Evidence root:
`/home/vishvananda/work/private/v5rust/artifacts/pa9-contexts-20261013/`.
The implementation commit is the commit containing this entry; its hash and
clean status are recorded in external `handoff.json`. `source-hashes.json`
records final Rust and reviewed C++ source identities. Generated artifacts,
logs, fixtures for supplemental probes and binaries are not committed.

| Command/evidence | Result |
| --- | --- |
| `make build` | PASS; optimized workspace build, `build.log`/`build-final.log`. |
| `make test-pa9` / required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa9` | PASS **119/119**, `stage.log`, `stage-final.log`, `stage.json`. `turn-start-results.json` and `progress.json` prove the same 119 identities and manifest SHA, eleven gains and zero lost identities. |
| `make course-pa9` | PASS **119/119**, exit 0, no failure marker; `course.log`, `gate-snapshot/course-pa9/test/course-result.json` and `counts.txt`. |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa9` | PASS; PA1 58/58, PA2 26/26, PA3 20/20, PA4 106/106, PA5 196/196, PA6 121/121, PA7 214/214, PA8 125/125; all owning course suites PASS. `prior.log`, `prior-summary.json`, retained per-stage results and course receipts. |
| `make lint` / protected `clone_quality.py` | PASS; rustfmt, strict all/pedantic Clippy, debug/release workspace tests (202 library tests), file/layout checks. `lint-final.log`. |
| `make audit-rust` | PASS, 161 files, zero findings; `audit-final.log`. |
| Protected `clone_check.py preflight pa9` / `preserve pa9` | PASS; `preflight.log`, `preserve.log`. Identity comparison independently proves all 108 entry passes retained. |
| Supplemental protected-adapter probes | **119/119 exact matches**, `probes.py`, `probe-*.facts`, `probes.log`, `probes.trace.jsonl`: local/raw/lambda contexts, short/long discriminators, index bounds, closures/signatures, generated-name defaults, repeated/equivalent/distinct contexts, nested member/template substitutions, entities, qualifier/terminal restrictions, cycles and malformed-record error precedence. |

`gate-snapshot/cases-pa9/` retains every fixed per-case course log and protected
receipt with raw streams and output hashes; traces also remain under
`/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa9/`.
`stage.json` maps identities to those paths. The existing empty-output adapter
defect is documented in the earlier checkpoint; no checker/oracle/policy
change was made here.

### Compile-time, RSS and scaling

`scaling.py`, fixed generated `.facts` inputs, `scaling.trace.jsonl`,
`scaling.log` and `scaling.json` retain digests, two oracle warmups and paired
ABBA runs. **9/9 protected comparisons** and all **72 measured invocations**
succeed with byte-identical outputs. Values are candidate median process
latency (ms) / maximum RSS (KiB), from four observations per row; they include
process/timing-wrapper costs and are observations rather than equality gates.

| Supported fact count | Distinct local types, each used twice | Closure signature parameters | Member prefixes under a local type |
| --- | --- | --- | --- |
| 4,000 | 9.434 / 4,316 | 6.156 / 2,268 | 8.280 / 3,632 |
| 8,000 | 17.564 / 6,416 | 9.829 / 2,960 | 11.104 / 5,016 |
| 16,000 | 34.162 / 10,652 | 16.069 / 3,644 | 21.993 / 7,164 |

Growth is approximately linear on these supported input families. Local type
resolution uses direct binder lookup; prefix construction refers to already
resolved predecessors and the encoder walks the final chain iteratively.
This does not claim arbitrary deep-forward-demand or nested-context scaling.
The initial optimized workspace build after the group took 18.41 seconds in
`build.log`; ABI measurements above observe the new supported hot paths.

### Exact fixed gains, remaining work and next boundary

No fixed PA9 case IDs remain failing. The exact eleven recovered IDs, relative
to `pa9/`, are:

- `tests/abi/200-lambda-closure-type.t`
- `tests/abi/200-local-class-call-operator.t`
- `tests/abi/200-local-context-destructor-discriminator.t`
- `tests/abi/200-local-context-destructor-terminal.t`
- `tests/abi/200-local-context-lambda-substitution-order.t`
- `tests/abi/200-member-lambda-call-operator.t`
- `tests/abi/200-raw-lambda-context-function.t`
- `tests/abi/200-raw-local-context-function.t`
- `tests/abi/600-function-local-class-template-arg.t`
- `tests/abi/600-function-template-local-class-arg.t`
- `tests/abi/600-function-template-local-lambda-arg.t`

The **standalone local-context group is done**, including related signature,
discriminator and shared-prefix rules beyond the fixed failures. The next
coherent boundary is **ABI tool statistics**: translate `abi_stats.h`, reader
and encoder work counts and storage observations through typed optional
telemetry to `driver/abimangle.rs`; `--stats` still returns explicit status 86.
Retain real Rust allocation observations. Review the protected comparator's
`peak_graph_bytes` treatment before comparing implementation-specific storage,
without fabricating C++ sizes or changing acceptance policy.

Later compiler-only graph/encoder interfaces remain partial: implicit/hidden
ABI tags and tag memos, standard-abbreviation derivation, data-member closure
prefixes, unnamed/decomposition names, template-parameter declarations and
advanced expression/literal/target forms previously listed in this ledger.
They are distinct from the standalone reader's translated productions and
must have owners and exact evidence before their producer milestones. Full
PA9 audit should inspect statistics, typed producer coverage and the raw I/O
boundaries before advancement; fixture counts alone do not certify the stage.

## Dependent expression identity and encoding checkpoint — 2026-10-13

Entry HEAD: `8f54cbd0ba680cb0f7f6120edbc3564f72b31297`, clean checkout.
The preceding goal turn was **progress**: its committed nonlocal ABI group
established the verified 93/119 baseline. No interrupted checker/compiler
process was live at this entry. The stage base and last-audited fields above
remain unchanged; implementation does not advance audit state.

This coherent partial group moves the same frozen manifest from **93/119 to
108/119**, reducing failures **26 → 11** and preserving **93/93** previously
passing identities. PA9 full-stage acceptance is still incomplete. Ralph owns
checkpoint/final audits and advancement.

### C++ owners, implementation and architecture review

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the complete owning handout,
rust-clone-plan.md and this ledger. Consulted the frozen production owners in
`/home/vishvananda/work/references/v5rust-cpp/dev/src/abi/`: the
`facts/abi_fact_definitions.cpp` expression/type/argument readers and ordered
resolution boundary, `itanium/abi_graph.cpp` expression constructors and
identity fields, `itanium/abi_encoder.cpp` expression/type/argument/entity
encoding, and `itanium/abi_mangle.h` operation/shape vocabulary. Consulted raw
protected receipts and outputs; remaining cases reach explicit local-context
status 86 before encoding, while all expression-group cases now match.

| C++ responsibility | Rust owner and invariants |
| --- | --- |
| `read_expression_definition` and operation validation | `abi/fact_expressions.rs`, `abi/vocabulary.rs`: all standalone expression productions, complete frozen operation table with semantic enum names and typed operand shape, exact arity checks and left-to-right operand/error order. Unary/binary/conditional edges use a fixed three-ID temporary; variable calls and trait lists use local vectors published to graph-owned spans. |
| `decltype_of`, expression array bounds and expression arguments | `abi/fact_types.rs`, `abi/graph.rs`: canonical nodes retain id-expression category, operation kind, ordered child identities, literal bits, type-trait operands and array-bound expression identity. Expression literals deliberately retain the reader's target long-long bit pattern, unlike range-checked typed argument values. |
| Definition resolution | `abi/facts.rs`: expressions join the existing file-order, Unstarted/InProgress/Complete resolution and forward-reference/cycle rules. Binder spelling never identifies an expression; separately defined equivalent structures share one canonical ID. |
| `encode_expression` and expression consumers | `abi/encoder.rs`: expressions are not substitution entries. Their type operands use the enclosing name's dictionary in encoding order; decltype and array types are recorded after operands. Template-parameter expressions encode directly without substituting the matching type parameter. Entity references reuse the existing typed entity encoder and its dictionary, including internal-variable linkage and opaque external symbols. |

`source-map.tsv` records these translated responsibilities as partial PA9
owners rather than claiming later compiler producers complete. Graph nodes,
edges and text remain flat, immutable and case-owned; substitution comparisons
are ID comparisons. No recursive owning pointers, rendered-text keys, phase
reparsing, unsafe code, extra dependencies, whole-program scans or fallback
compiler paths were introduced. Encoding scratch still dies per name. The
new graph test verifies expression identity distinctions and span stability
under store growth; existing tests continue to check per-name lifetime,
canonical qualifiers, typed owners and forward/cyclic resolution.

Optimizer, register allocation, native backend and source-driver code were
untouched. PA9 produces names, not objects; no object-byte or PA34 completion
claim is inferred from these checks. PA35 optimization targets remain outside
this port.

### Validation and retained evidence

Evidence root:
`/home/vishvananda/work/private/v5rust/artifacts/pa9-expressions-20261013/`.
The implementation commit is the commit containing this entry; its hash,
checkout status and candidate hashes are recorded in external `handoff.json`.
`source-hashes.json` records the reviewed C++ and final Rust source identities.

| Command/evidence | Result |
| --- | --- |
| `make build` | PASS; final release rebuild 18.42 s, `build-final.log`. |
| `make test-pa9` / required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa9` | Still FAIL, **108/119**. `stage-final.log`, `stage.json`; `progress.json` proves identical manifest digest and case IDs, fifteen gains, zero losses. |
| `make course-pa9` | Still FAIL, **108/119**, same eleven local cases; `course-final.log`, `gate-snapshot/course-pa9/test/counts.txt` and course receipt. |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa9` | PASS; PA1–PA8 fixed manifests and every owning course suite green. `prior-final.log`, `gate-snapshot/results/pa1.json` through `pa8.json` and course receipts. |
| `make lint` / protected `clone_quality.py` | PASS; rustfmt, strict Clippy, all targets/features, debug and release library tests **201/201**, layout/file checks; `lint-final.log`. |
| `make audit-rust` | PASS, **160 files, zero findings**; `audit.log`. |
| Protected `clone_check.py preflight pa9` | PASS; `preflight.log`. |
| Protected `clone_check.py preserve pa9` | PASS, **93/93**; `preserve.log`. |
| Additional expression probes through protected adapter | **126/126 exact matches**; `probes.py`, `probe-*.facts`, `probes.trace.jsonl`, `probes.log`. All unary/binary/cast/access vocabulary shapes, malformed forms, error precedence, forward/cyclic dependencies, raw entity references, expression-array substitutions, distinct/equivalent facts, and target integral boundaries. |

`gate-snapshot/cases-pa9/` retains the fixed per-case receipts, raw mismatch
files and course logs. Live protected paths remain under
`artifacts/clone/cases/pa9/` outside the checkout; `stage.json` maps exact case
IDs to those paths. `turn-start-results.json` preserves the entry evidence.
Scratch inputs and outputs were moved to the evidence root at handoff;
receipts retain the original invocation paths. No generated files or logs
are committed, and no fixtures/references/checker policy were edited.

### Compile-time, RSS and scaling

`scaling.py`, `scaling.json`, `scaling.log` and `scaling.trace.jsonl` retain
fixed generated inputs/digests, two oracle warmups, two paired ABBA blocks,
raw latency/RSS observations and output hashes. All **9/9 protected adapter
comparisons** and **72/72 measured invocations** succeed with identical output
bytes. Candidate median ms / maximum KiB below are four observations per row,
including launch/timing-wrapper overhead; no C++ resource-equality claim.

| Size | Expression-bound array modifiers | Distinct cast/decltype parameters, repeated | Type-trait expression arguments |
| --- | --- | --- | --- |
| 4,000 | 4.758 / 2,900 | 17.172 / 6,452 | 13.944 / 5,760 |
| 8,000 | 6.554 / 3,636 | 32.572 / 10,688 | 28.269 / 9,264 |
| 16,000 | 11.023 / 4,340 | 66.164 / 18,348 | 57.153 / 17,704 |

The supported expression/identity/substitution workloads scale approximately
linearly. These shallow expression graphs exercise large flat stores and
modifier lists, not arbitrary recursive depth. The existing genuine stack
byte budget remains in resolution and expression/type encoding; no native
exhaustion cutoff or diagnostic position was calibrated. Resource-limited
source-driver probes must use protected `stack_qualification` receipts and
judge their `match`, as required by CLONE_CONTRACT.md.

### Exact remaining IDs and next dependency boundary

All remaining IDs are relative to `pa9/`, with oracle success versus explicit
candidate status 86 at the local context/type/function boundary:

- `tests/abi/200-lambda-closure-type.t`
- `tests/abi/200-local-class-call-operator.t`
- `tests/abi/200-local-context-destructor-discriminator.t`
- `tests/abi/200-local-context-destructor-terminal.t`
- `tests/abi/200-local-context-lambda-substitution-order.t`
- `tests/abi/200-member-lambda-call-operator.t`
- `tests/abi/200-raw-lambda-context-function.t`
- `tests/abi/200-raw-local-context-function.t`
- `tests/abi/600-function-local-class-template-arg.t`
- `tests/abi/600-function-template-local-class-arg.t`
- `tests/abi/600-function-template-local-lambda-arg.t`

The **standalone dependent-expression group is done**, including related
expression array/decltype consumers beyond the previously failing cases.
The next coherent implementation group is **local function/lambda contexts**:
`read_context_definition`, `local_entity`, `closure_entity`, local/lambda
function records, typed function/raw scopes and closure signatures,
`encode_context`, `encode_local_name`, discriminators and known-prefix sharing.
It has distinct ownership and substitution/context state and owns all eleven
remaining fixed failures.

`--stats` remains status 86 and needs faithful counters plus actual Rust
storage measurements before full-stage acceptance; `peak_graph_bytes`
comparison needs the previously recorded policy review, without fabricated
C++ allocation sizes. Later compiler-only expression constructors/encodings
(function-parameter levels/this, prefixed increments, unresolved prefixes,
operator callees, expression lists/allocation, floating/nullptr literals),
implicit ABI tags and other producer interfaces remain partial source-map
responsibilities. The standalone reader cannot construct those forms; this
checkpoint does not claim them complete. The earlier empty-output adapter
defect remains documented below; this group needed no checker/oracle changes.

## Nonlocal ABI graph, type and entity encoding checkpoint — 2026-10-13

This coherent partial implementation moves the frozen manifest from **0/119
to 93/119**, reducing failures from 119 to 26 without changing case coverage
or losing any passing identity. PA9 full-stage acceptance remains incomplete.
Ralph owns the checkpoint/final audits and stage advancement.

At entry the checkout was clean at the recorded base, no interrupted compiler
or checker process was live, the ABI implementation was a status-86 stub,
and this ledger had no implementation evidence. The preceding interrupted
PA9 continuation was classified as no durable progress; the stub baseline was
revalidated and the next available translation group implemented.

### Contract, source owners and typed data flow

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the complete PA9 handout and
rust-clone-plan.md. Consulted the frozen C++ commit
`883778fdfa45d523cf9e135d15169af976f451a8` directly: `dev/abimangle.cpp`,
`dev/abimangle-scaffold.cpp`, `abi_mangle.h`, `abi_graph.{h,cpp}`,
`abi_encoder.{h,cpp}`, `abi_encoding_error.h`, `abi_stats.h`,
`abi_fact_case.h`, `abi_fact_reader.{h,cpp}`, `abi_fact_definitions.cpp`,
and the shared help and stack-budget owners. The production implementation,
including its evaluation and substitution order, takes precedence over the
student-facing scaffold. `docs/port/source-map.tsv` records partial owners
without claiming the remaining responsibilities complete.

| C++ responsibility | Rust owner and translated invariants |
| --- | --- |
| Tool invocation, help, file/error boundaries | `dev/bin/abimangle.rs`, `driver/abimangle.rs`: original argv/path bytes, help precedence, repeated/empty `-o`, option errors, output-before-input opening, 4-KiB output buffering and distinct reader-write/final-flush failures. Soft stack limit is collected in the executable and passed as a typed budget. |
| Fact input and definition resolution | `abi/facts.rs`: one active case, immutable case bytes and word slices, file/line/case order, comments/separators, file-local definition hash index, explicit Unstarted/InProgress/Complete states, file-order resolution and forward references/cycles, 64-KiB output flush. No facts are shared between cases. |
| Type and argument facts | `abi/fact_types.rs`: iterative compact/multiword wrappers; builtin/named/member/template/vendor/tagged types, function types, member pointers, array values, template parameters/template-template arguments, packs, typed integral and dependent-value arguments with deliberate target wrapping/range checks. |
| Function and entity facts | `abi/fact_functions.rs`: ordinary/structured names, standard and namespace-lambda components, canonical tags, all operator/special-member/conversion/literal/inherited-constructor terminals, qualifiers, signatures, result/parameter order, member and raw-symbol entity arguments. Binder/key spelling does not define ABI identity. |
| Canonical graph and vocabulary | `abi/graph.rs`, `abi/vocabulary.rs`: flat node/text/edge pools, checked u32 spans, private NonZeroU32 IDs (including four-byte optional IDs), hash-consing of typed records/edges, merged cv qualifiers and sorted unique tags, class-specific member-function type identity. No per-node owning pointer or recursive drop. |
| Itanium emission | `abi/encoder.rs`: graph IDs directly to bytes, per-name dense substitution slots, iterative prefix and modifier chains recorded inside-out, base-36 indices, nested/template names, canonical qualifier order, ordinary/template functions, typed values and nested entity references, RTTI/vtables/VTT/construction-vtables/TLS wrappers and all fixed/virtual/covariant thunk forms. |

Parsing is confined to the explicit PA9 input boundary. Later compiler
producers can construct these same graph facts in-process. The encoder never
parses spelling, fact text or rendered types. Flat stores own all edges/text,
definition binders refer only to this case, and encoder scratch dies at the
name boundary. Intern-table iteration never controls output. Four meaningful
ABI unit tests exercise canonical identity, owner-specific function types,
span stability after growth, forward resolution/cycles, and 10,000-link
iterative chains. Graph/vocabulary roles are registered with the stricter
model/interface caps; no audit limits or policy were relaxed.

No optimizer, native backend, source-language parser or code-generation
behavior was changed. PA35 optimization targets are outside this checkpoint.
PA34 remains the eventual frozen-source object/tool byte-equality proof.

### Validation and immutable evidence

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/pa9-names-20261013/`.
The checkpoint's implementation commit is the commit containing this entry;
its exact hash and clean status are recorded externally in `handoff.json`.
`source-hashes.json` records Rust/C++ source identities. Candidate release
`abimangle` SHA-256: `52cc8d311b86e5e5c371c4144317961d5202eb41b091ed6dfb19c727639b786b`.

| Command/evidence | Result |
| --- | --- |
| `make build` | PASS; `build-final.log`, optimized workspace build. |
| `make test-pa9` (required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa9`) | Required check still FAIL, **93/119**, same frozen manifest; `stage.log`, `stage.json`, `progress.json`, `turn-start-results.json`. Every remaining failure is explicit status 86 against oracle success. |
| `make course-pa9` | Still FAIL, **93/119**; owning course target returns zero but emits a failure marker/counts, correctly rejected by protected wrapper. `course.log`, `gate-snapshot/course-pa9/course-result.json` and `counts.txt`. |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa9` | PASS; PA1 58/58, PA2 26/26, PA3 20/20, PA4 106/106, PA5 196/196, PA6 121/121, PA7 214/214, PA8 125/125; every owning course suite PASS. `prior.log`, `gate-snapshot/pa1.json` through `pa8.json`, course receipts/logs. |
| `make lint` / protected `clone_quality.py` | PASS; rustfmt, strict all/pedantic Clippy, all-target/all-feature checks, debug and release tests (200 library tests), compiler layout/file checks. `lint-final.log`. |
| `make audit-rust` | PASS, 159 files, zero findings; `audit.log`. |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa9` | PASS; routing/input/oracle integrity, `preflight.log`. |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa9` | PASS, 0/0 prior passing identities; `preserve.log`. `progress.json` additionally proves identical 119 case IDs and 93 gains from the turn-start snapshot. |
| Additional probes through the protected adapter | **82/82 exact matches**; `probes.py`, input files, `probes.trace.jsonl`, `probes.log`: CLI, malformed facts, forward/cyclic definitions, integer bounds, raw byte names, member function owner identity, canonical qualifiers/tags, structural substitutions including multi-digit base-36 indices. |
| Direct I/O/empty-output CLI probes | **2/2 exact matches** (including output hashes); `io-probes.py`, `empty-output.json`, `device-write.json` and raw stdout/stderr. Direct execution avoids the adapter defect described below and avoids treating a character device as a restorable file. |

`gate-snapshot/cases-pa9/` retains all fixed per-case receipts, course logs,
and protected mismatches with raw oracle/candidate diagnostics and output
bytes. Live gate traces also remain at
`/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa9/`;
`stage.json` maps every case ID to its trace and course log. Supplemental
scratch was collected under the checkout's `artifacts/` directory and moved
in bulk to the evidence root at handoff; receipt arguments retain the original
invocation paths. Generated files, logs, binaries and Cargo targets are not
committed.

### Compile-time, memory and scaling observations

`scaling.py`, `scaling.json`, `scaling.log` and `scaling.trace.jsonl` retain
fixed generated inputs, input/output/binary digests, A/A warmup, paired ABBA
runs and raw `/usr/bin/time` RSS. All **9/9 protected comparisons** and all
72 measured compiler invocations succeeded with byte-identical outputs.
Numbers below are candidate median process latency in ms / maximum RSS in KiB
for four observations per row; they include launch and timing-wrapper costs.
They are observations, not a C++ resource-equality requirement.

| Supported input size | Pointer modifiers | Qualified name prefixes | Sequential resolved definitions |
| --- | --- | --- | --- |
| 16,000 | 9.013 / 4,348 | 10.396 / 3,640 | 16.832 / 7,164 |
| 32,000 | 13.588 / 5,728 | 18.845 / 5,752 | 32.698 / 12,092 |
| 64,000 | 23.842 / 9,976 | 35.278 / 8,548 | 66.752 / 21,760 |

Growth is approximately linear on these supported chains and forward-owned
stores. Definitions refer to already-resolved predecessors, matching C++'s
file-order read; this does not claim deep forward-recursion performance.
The release workspace build after the implementation took 18.14 s in the
retained `build.log`; ordinary compiler-source inputs above measure the new
ABI hot paths separately.

A genuine byte budget guards remaining recursive calls using stack addresses
and the host's typed soft limit. Iterative chains cost no stack depth. None
of these probes exhausted native stack, so no exhaustion receipt was needed.
For future resource-limited source-driver probes use the protected adapter's
receipt's `match` and `stack_qualification` evidence; never tune native exhaustion cutoffs or
compare exhaustion diagnostic locations. Existing PA1–PA8 protected receipts
are preserved with their applicable qualification semantics.

### Exact remaining fixed case IDs

All IDs below are relative to `pa9/`; none were removed or weakened:

- `tests/abi/200-lambda-closure-type.t`
- `tests/abi/200-local-class-call-operator.t`
- `tests/abi/200-local-context-destructor-discriminator.t`
- `tests/abi/200-local-context-destructor-terminal.t`
- `tests/abi/200-local-context-lambda-substitution-order.t`
- `tests/abi/200-member-lambda-call-operator.t`
- `tests/abi/200-raw-lambda-context-function.t`
- `tests/abi/200-raw-local-context-function.t`
- `tests/abi/300-internal-variable-entity-expression.t`
- `tests/abi/400-dependent-alias-type-id.t`
- `tests/abi/500-dependent-bitset-words.t`
- `tests/abi/500-dependent-call-expression.t`
- `tests/abi/500-dependent-cast-expression.t`
- `tests/abi/500-dependent-expression-type-substitution-order.t`
- `tests/abi/500-dependent-function-parameter-decltype-param.t`
- `tests/abi/500-dependent-object-member-expression.t`
- `tests/abi/500-dependent-pack-expression.t`
- `tests/abi/500-dependent-sizeof-type-expression.t`
- `tests/abi/500-dependent-type-trait-expression.t`
- `tests/abi/500-distinct-integral-decltype-substitution.t`
- `tests/abi/500-distinct-type-trait-expression-substitution.t`
- `tests/abi/500-equivalent-dependent-expr-substitution.t`
- `tests/abi/500-equivalent-integral-decltype-substitution.t`
- `tests/abi/600-function-local-class-template-arg.t`
- `tests/abi/600-function-template-local-class-arg.t`
- `tests/abi/600-function-template-local-lambda-arg.t`

### Handoff and next dependency boundary

The nonlocal graph/type/function/template/entity/special-symbol group is done;
all 93 cases supported by it pass the exact comparator and course checks.
The next coherent group is **dependent expressions**: add typed graph nodes,
translate `read_expression_definition`, expression-array bounds/decltype/
expression arguments, and `encode_expression` with the same substitution
ordering. This owns 15 of the remaining failures (300/400/500 families).
The separate **local function/lambda context** group owns the other 11
(200/600 families): typed function/raw scopes, closure signatures, local
occurrences/discriminators and nested prefix/context substitution sharing.
These are new productions with distinct dataflow; no unexplained divergence
remains in the translated group.

`--stats` remains explicitly status 86 outside the fixed fixture coverage.
Translate reader/encoder work counters and actual Rust storage observations
before full-stage acceptance; audit the protected comparator's treatment of
`peak_graph_bytes` rather than fabricating C++ allocation sizes. Later compiler
producer graph constructors (implicit tag derivation, abbreviation derivation,
additional expression/closure/default-argument forms, later special targets)
are also listed as partial in the source map and need ownership review at
those milestones. No fixture-count result proves those interfaces complete.
Independent audit should review constructor and conversion substitution
ordering, tags and member-function owner identities, the flat-store/span API,
forward-reference demand state, tool flush/error boundaries and all deferred
status-86 paths before stage advancement.

### Harness defect reported, without changing policy

The protected adapter treats an empty output operand as `Path('.')`.
Its `restore(before)` calls `unlink('.')` and raises `IsADirectoryError` for
this valid C++ invocation:

```text
abimangle -o '' -o /tmp/pa9-empty-output.my pa9/tests/abi/100-global-function.t
```

Reproduce with `clone_adapter.compare_invocation('abimangle',
['-o', '', '-o', '/tmp/pa9-empty-output.my',
'pa9/tests/abi/100-global-function.t'], b'', {})` and a writable `V5RUST_TRACE`.
The retained `probes-first.log` contains the traceback. Direct paired execution
in `io-probes.py` proves both tools succeed with identical streams and file
hashes; fixed tests are unaffected. No protected checker, reference, fixture,
or oracle was edited. This is an adapter output-tracking defect, not a
permission to waive a contract case.
