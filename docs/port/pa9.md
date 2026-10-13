# PA9 port ledger

Stage base commit: `867f7a05e3a646f89773bd11ef7194fdb7f01056`.
Last audited commit: INITIAL (implementation does not advance this field).

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
