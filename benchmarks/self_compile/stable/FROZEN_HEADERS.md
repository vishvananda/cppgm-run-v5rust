# Frozen Semantic-Overload Header Closure

`include/` contains the project headers used by the stable
`semantic_overload.cpp` performance workload. The closure was frozen from
`dev/src` at commit `56afe87c1766f41b1f610335d9005a1b1f0538ce` on 2026-07-18.

The 51-file closure was discovered with:

```sh
/usr/local/opt/llvm/bin/clang++ -std=gnu++11 -I dev/src -MM \
  benchmarks/self_compile/stable/semantic_overload.cpp
```

It was then verified by compiling the stable source with `cppgm++` while the
frozen directory was the only project include path. The default performance
gate uses:

```text
-I benchmarks/self_compile/stable/include
```

`PERF_EPOCH.json` makes this freeze executable. It records epoch commit
`9764b3835e3c6996b6b80803054f80e1cf50f98e`, the stable source digest, the
complete 51-file membership, every header digest, and the aggregate closure
digest. `scripts/validate_perf_regression.py` verifies that manifest before it
runs either `record` or `check`. It rejects missing, added, changed, or symlinked
headers, source drift, and any workload command other than this exact source and
include root (the output object path may still differ).

Baselines recorded before the manifest was added remain valid only when their
recorded head is the epoch commit itself and their normalized command is the
frozen command. This preserves the existing epoch baseline without remeasuring
its compiler or parent.

Do not refresh this directory as part of ordinary compiler work. A deliberate
source or header refresh creates a new benchmark epoch, must update this record,
`PERF_EPOCH.json`, and every digest, and requires a new performance baseline.
The closure freezes repository project headers only; host standard-library and
compiler-provided headers remain part of the fixed machine/toolchain environment.
