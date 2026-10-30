# Rust workspace build

Base: `faf16dbe`. Seven internal component crates build the existing phase
modules; the `cppgm-rust` package keeps the driver, all nine binaries and its
existing public module paths. Implementation files and source-ledger paths stay
under `dev/src/`; component manifests live under `dev/components/`.

| Component | Modules | Dependencies |
| --- | --- | --- |
| core | support, target types, ABI | standard library |
| ir | LowIR model, builder, readers/writers | core |
| syntax | lexer, preprocessor, parser/AST | core |
| sema | semantic graph and analysis | core, syntax |
| lower | source lowering | core, syntax, sema, ir |
| opt | optimizer | core, ir |
| native | native selection, objects and linking | core, ir |

The driver depends on these components directly. Native/optimizer edits no
longer rebuild semantic analysis or lowering. Cargo pipelines metadata and
code generation: IR and syntax start independently; sema, optimizer and native
code generation overlap; lowering starts when semantic metadata is available.
The semantic crate uses 64 release codegen units. Release optimization level
and LTO policy retain their defaults.

Component interfaces expose the existing typed facts and stores across crate
boundaries. ID representations and store internals stay private. Small shared
accessors retain inlining hints; profiling also identified LowIR link traversal
and semantic type-view lookup as cross-crate inlining boundaries. Workspace
lint policy is inherited unchanged by every package. A syntax-aware review
compared 3,316 existing function bodies: the only changed body was the test
whose diagnostic assertions moved into the driver's test owner. Three added
`is_empty` queries accompany newly public collection interfaces.

Telemetry features propagate through component dependencies. Test-only graph
and invariant helpers use explicit `test-support` dev-dependency features;
production builds do not enable those features. The original assertion helpers
retain narrow, justified Clippy expectations in their test-only scope. Their
assertion bodies are unchanged. The six conditional-diagnostic cases moved
from the preprocessor storage test to the existing preprocessor driver test
module, preserving coverage without a dependency back into the driver.

Cargo quality checks use `obj/rust-quality`; production builds use `obj/rust`.
This prevents the test-support feature set from replacing candidate binaries
or invalidating content-bound compiler receipts. The quality script still runs
formatting, strict Clippy, debug/release workspace tests and the file audit.

## Measurements

Rust 1.98.0, all nine release tools, cached dependency downloads, fresh target
directories. Measurements and raw Cargo timing reports are under
`/home/vishvananda/work/maintenance/v5rust-current-perf-20261030T013029Z/`.

| Build | Monolithic | Components |
| --- | ---: | ---: |
| Clean release, initial baseline/final repeat | 123.78 s | 79.13 s |
| Native-only source invalidation | 106.17 s | 13.59 s |

The component native rebuild compiled only `cppgm-native` and the driver
facade. Shared-host load varied substantially: another monolithic clean build
took 167.69 s during a load spike. Treat wall-time percentages as observations,
not an isolated-host calibration. `crates-build-3.time` records another component
clean build at 82.23 s; `crates-build-clean-verified.time` records the final repeat.

Four warm alternating before/after samples on one CPU retained complete frozen
workload object identity at O0 and O3. User instructions changed from 9.5628 B
to 9.6276 B at O0 (+0.68%) and 22.9472 B to 23.1903 B at O3 (+1.06%). Raw samples
are in `crate-runtime-fixed/performance.json`; shared-host timing variation is
larger than these instruction differences.

The final four-sample comparison against the frozen C++ reference used the
same CPU and 16 MiB stack, with warmups and alternating order. Complete O0/O3
object bytes matched. Medians are in `final-frozen/performance.json`:

| Frozen workload | Rust wall | C++ wall | Rust/C++ instructions |
| --- | ---: | ---: | ---: |
| O0 | 3.52 s | 1.20 s | 3.016x |
| O3 | 7.13 s | 2.625 s | 2.947x |

`crates-bootstrap-probe.json` records all 227 objects and all nine rebuilt tools
matching the frozen C++ self-build exactly with the component compiler. Compiler
SHA256: `411a8f6b04649b8e64bb9ee4eac48738526eacc406834fc30dec0ac04a47467a`.
Validation passed 9,218 differential cases and all 33 original course gates
through PA33, plus the PA8/PA32/PA33 debug-info lanes. Formatting, strict Clippy,
debug and release workspace tests, and the 667-file audit all passed. The
production compiler hash remained unchanged throughout the isolated quality
checks. `main-validation-summary.json` records the stage counts.

The worker and its goal remain paused. This packaging change does not close
pending C++ source responsibilities or claim final PA34 source-audit acceptance.

## Next throughput work

Analysis at `9f902c9e`, without compiler changes, is recorded under
`/home/vishvananda/work/maintenance/v5rust-next-perf-20261030/`.
`summary.json` indexes the findings; `callgrind-analysis.json` retains exact
call counts and instruction costs. O0/O3 output objects stayed byte-identical
for every profiling and build-option experiment. The worker remains paused.

The within-10% wall-time targets from the warm reference samples are 1.32 s
at O0 and 2.8875 s at O3. Reaching those from 3.52/7.13 s requires about
62.5%/59.5% less elapsed time. Small inlining changes alone cannot do that.
An isolated ThinLTO build reduced hardware instructions by only 1.21%/1.57%;
no production build setting was changed. The timing experiment's first
samples overlapped instrumentation and are excluded in
`lto-warm-summary.json`; instruction counts were stable across all samples.

| Measured responsibility | Rust | C++ |
| --- | ---: | ---: |
| O0 malloc calls | 4,731,745 | 392,848 |
| O3 malloc calls | 14,280,114 | 2,071,949 |
| O3 whole optimizer, Callgrind instructions | 12.761 B | 4.193 B |
| O3 operand-substitution calls | 1,534,346 | 1,534,346 |
| Instructions inside those substitutions, including callees | 779.651 M | 124.469 M |
| LowIR instruction record | 112 bytes | 64 bytes |

These are diagnostic Callgrind measurements, distinct from native hardware
counter totals. Inclusive costs overlap: substitution is inside simplification
and the optimizer. They must not be added. C++ O0 instrumentation reuses the
identical frozen binary/input/options from the preceding study; O3 and both
Rust profiles are fresh. Native Rust stack unwinding was incomplete, so
call attribution uses instrumented call edges and source inspection.

The next coherent changes should preserve execution order and replace the
extra ownership work:

1. **Restore front-end query lifetimes and common paths.** In
   `sema/dependence.rs:527`, each dependence query starts a Vec and HashSet.
   C++ `sema/class_templates.cpp:1506` memoizes stable compound-type answers
   and walks simple chains without hash allocation. Preserve its class/leaf
   validity rules rather than caching mutable class facts indiscriminately.
   Probe canonical type keys before cloning parameter vectors. Split cached
   parser lookahead and ordinary lexer character advancement from fallible
   refill/decoding paths so their frequent success cases can inline. O0
   parsing currently consumes 2.569 B instrumented instructions versus
   1.211 B in C++; initial analysis plus deferred semantic demands also remain
   expensive. The earlier builtin registry and identity hash fixes are present.
2. **Restore direct LowIR editing and compact records.** In
   `opt/value_reuse.rs:179`, substitution clones the instruction, allocates an
   operand vector, then rewrites the original. C++ edits operands directly.
   Split borrowed instruction/operand/fact stores to permit safe in-place
   edits. Move variable operands and cold call ABI metadata into owned pools,
   bringing the record toward the reference's 64 bytes. Keep function-owned
   instruction links instead of rebuilding five LinkEditor arrays and copying
   block lists for each editing pass. The latter constructor runs 50,927 times
   at O3 and costs 777.671 M inclusive instructions.
3. **Retain scratch and invalidate analyses precisely.** C++ keeps capacity
   for optimizer contexts, CFGs, value simplification, constant propagation,
   slot promotion and memory forwarding. Rust constructs fresh containers.
   Match those lifetimes with an explicit optimizer-owned scratch workspace;
   logical state must still reset between functions. Investigate the extra
   full use-count scans: Rust's Body count runs 75,417 times at O3 versus
   14,230 calls to C++ optimizer count_uses. Several Rust passes recount
   unconditionally. Remove a scan only after proving the next consumer's
   counts remain valid. Dense optional TempId/BlockId entries also occupy
   eight bytes versus the reference's four-byte sentinel IDs; safe compact
   encodings can reduce that traffic without changing absence semantics.

Validate each coherent batch with frozen O0/O3 complete object equality and
alternating warm hardware-counter/wall samples. Keep pass choices, ordering,
diagnostics, reference files and gate limits fixed. Run affected differential
and debug lanes during iteration, then full course, quality and 227-object /
nine-tool bootstrap checks at a stable milestone. The 10% target is an
acceptance objective, not a speedup established by this analysis.

## Throughput changes and remeasurement

The next batch restores memoized compound-type dependence and bare-type
identities, inlines cached token lookahead and character advancement, and
separates refill/decoding from those success paths. Mutable class/enum leaves
are not memoized. The dependence walk remains iterative and uses retained
scratch, without a query-local HashSet.

Operand substitution now edits the owning operand store without cloning an
instruction or allocating a temporary operand list. Value simplification moves
its updated record back once. An explicit optimizer-owned workspace retains
CFG, value-simplification, memory-forwarding, slot-promotion and constant-
propagation capacity across functions while resetting their logical state.
Unchanged passes avoid unnecessary use-count refreshes; required refreshes reuse
buffers. LinkEditor initialization fills links directly, and commits refill
existing block lists without allocating another list for each block.

LowIR types now occupy eight bytes, using the reference's 48-bit object-size
payload without unsafe packing. Optional temporary, block, instruction and
signature IDs occupy four bytes, preserving external indices, hashes and debug
spelling. Sparse immutable call ABI lists are shared; empty lists allocate
nothing. Instruction records shrink from 112 to 72 bytes, and calls from 72
to 40. Pass choices, traversal order, diagnostics and serialized output remain
covered by the frozen comparisons.

Evidence is under
`/home/vishvananda/work/maintenance/v5rust-throughput-20261030/`.
`performance-summary.json` indexes the raw results. Four warm alternating
samples per compiler/mode used one CPU, a shared 16 MiB stack and hardware
counters at 100% running time. Every output object matched completely.

| Frozen workload | Before | After | C++ | After / C++ | Instruction reduction |
| --- | ---: | ---: | ---: | ---: | ---: |
| O0 follow-up | 3.665 s | 3.635 s | 1.310 s | 2.77x | 4.86% |
| O3 | 7.200 s | 6.370 s | 2.700 s | 2.36x | 13.30% |

O3 wall time decreased 11.5%. O0 wall time is approximately unchanged: the
primary O0 sample was slower (3.64 to 4.09 s), which did not reproduce in the
four-compiler follow-up. Both studies retain the instruction improvement.
The follow-up enabled statistics for every compiler to compare phase clocks
and added cache/branch counters; the primary comparison did not. Raw evidence
is in `final/performance.json` and `repeat-O0/performance.json`. Shared-host
variation remains material; these results do not establish the 1.10x target.

Callgrind malloc counts fell from 4,731,745 to 4,044,465 at O0 (14.5%) and
14,280,114 to 10,722,198 at O3 (24.9%). LinkEditor construction fell from
777.7 M to 222.7 M inclusive instructions, so rebuilding its remaining small
view is now about 1% of total O3 instructions. Operand substitution fell from
779.7 M to 359.8 M inclusive instructions, with the same 1,534,346 calls.
Front-end storage and query costs remain the main obstacle at O0.

Validation passed 9,218 differential cases, all 33 original course gates,
PA8/PA32/PA33 debug-info lanes, formatting, strict Clippy, debug/release Rust
tests, and the 667-file audit with zero findings. All 227 bootstrap objects and
nine tools matched the frozen C++ fixed point. Candidate SHA256:
`f0d62f1d9197801b0f19a22cf768e0e4b0d3addea4fbc21293704409e01f24c8`.
The paused worker, source-owner census, reference and gate policy are unchanged.
