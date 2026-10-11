# Frozen Self-Compile Benchmark Corpus

This directory holds stable self-compile benchmark inputs used for
performance comparisons across compiler checkpoints.

Why these files exist:

- compiling `dev/src/*.cpp` directly is useful for real-world signal, but the
  source text changes as the compiler evolves
- that makes before/after timing comparisons noisy, because the benchmark
  input changes at the same time as the compiler
- these snapshots keep the benchmark source text fixed
- the primary `semantic_overload.cpp` regression gate also freezes its
  project-header closure, so compiler implementation changes are its only
  repository-side workload variable

The `stable/include/` tree is specifically the frozen project-header closure
for `stable/semantic_overload.cpp`. It must be selected instead of `dev/src`
for that regression gate. The other corpus entries remain source-only snapshots
unless their command explicitly selects a separately verified frozen closure.
See `stable/FROZEN_HEADERS.md` for provenance and refresh policy. Host
standard-library headers remain a machine/toolchain input, so comparisons still
require the same host environment.

The primary gate validates `stable/PERF_EPOCH.json` before executing. The
manifest pins the source, exact 51-header membership, individual SHA-256
digests, and aggregate closure digest for epoch `9764b3835`. Live `dev/src`
headers and unmanifested changes are rejected before measurement.

Per-slice frozen corpus:

- `stable/constant_value.cpp`
- `stable/semantic_model.cpp`
- `stable/semantic_overload.cpp`
- `stable/recog_token_buffer.cpp`
- `stable/parser_trace.cpp`
- `stable/rtti_names.cpp`
- `stable/cppast_dump.cpp`

Checkpoint-only frozen corpus:

- `stable/callsemantic_frozen.cpp`

`callsemantic_frozen.cpp` is intentionally not part of the normal
`--include-self-compile` benchmark set. It is large enough that it should be
used as a checkpoint anchor at major semantic-analysis phase boundaries, not
as a per-slice benchmark.

Frozen LowIR corpus for optimizer-only benchmarking:

- `frozen_lowir/semantic_overload.o0.lowir`

Why the frozen LowIR corpus exists:

- optimizer hotspots can shift on larger source files
- freezing `--emit-lowir -O0` output removes front-end and codegen noise from
  optimizer-only measurements
- these inputs can be fed directly to `scripts/profile_frozen_optimizer_phase.py`
  with `--lowir ... --source ...`

Most files were frozen from:

- source paths under `dev/src/`
- repository `HEAD` at the time this corpus was created

`callsemantic_frozen.cpp` was frozen from `dev/src/callsemantic.cpp` of the
compiler's earlier source layout, immediately before the reachability probe
implementation, at commit `a850082570c40d1b34bbc98c3c0919baaa1ce7cd` of the
history that predates this repository. Neither that file nor that commit
exists here; the frozen copy arrived with this repository's initial commit.
Freezing it there keeps the benchmark source fixed without benchmarking the
probe code itself.

Use `scripts/run_frozen_self_compile_benchmarks.sh` to run the corpus against
either the current compiler or a compiler binary from another worktree.

Example optimizer-only run on the larger frozen LowIR input:

```sh
scripts/profile_frozen_optimizer_phase.py \
  --repo-root . \
  --source benchmarks/self_compile/stable/semantic_overload.cpp \
  --lowir benchmarks/self_compile/frozen_lowir/semantic_overload.o0.lowir \
  --optimizer ./dev/lowiropt \
  --repeat 3 \
  --output-prefix /tmp/cppgm-optimizer-phase-semantic_overload
```
