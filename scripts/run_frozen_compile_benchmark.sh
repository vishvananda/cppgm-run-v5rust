#!/usr/bin/env bash
# Frozen-compile A/B: the quick standing perf check.
#
# The corpus is a pinned source AND its pinned headers, so the workload is
# constant and the only variable is the compiler.  Compare any two compiler
# binaries -- typically a pre-change build against the working tree, or the
# gcc-built compiler against the self-hosted one to read the self/gcc ratio
# the parity plans track.
#
# Usage:
#   scripts/run_frozen_compile_benchmark.sh <compiler-a> <compiler-b> [blocks] [-O level]
#
# The checked-in corpus includes its pinned headers. CPPGM_FROZEN_ROOT can
# select another checkout holding the same benchmarks/self_compile/stable tree.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
frozen=${CPPGM_FROZEN_ROOT:-$root}
if [ ! -d "$frozen" ]; then
  echo "frozen corpus directory not found: $frozen" >&2
  exit 2
fi
frozen=$(cd "$frozen" && pwd -P)
source_file=${CPPGM_FROZEN_SOURCE:-$frozen/benchmarks/self_compile/stable/semantic_overload.cpp}
include_root=$frozen/benchmarks/self_compile/stable/include

if [ ! -f "$source_file" ]; then
  echo "frozen corpus not found: $source_file" >&2
  echo "set CPPGM_FROZEN_ROOT to the directory holding benchmarks/self_compile" >&2
  exit 2
fi
source_file=$(cd "$(dirname "$source_file")" && pwd -P)/$(basename "$source_file")

if [ ! -d "$include_root" ]; then
  echo "frozen headers not found: $include_root" >&2
  exit 2
fi

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <compiler-a> <compiler-b> [blocks] [-Olevel]" >&2
  exit 2
fi

a=$1
b=$2
blocks=${3:-6}
level=${4:--O1}

exec python3 "$root/scripts/run_ab_compile_benchmark.py" \
  --repo-root "$frozen" \
  --compiler-a "$a" \
  --compiler-b "$b" \
  --source "$source_file" \
  --include "$include_root" \
  --compiler-arg="$level" \
  --abba-blocks "$blocks"
