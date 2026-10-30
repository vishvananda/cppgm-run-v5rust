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
