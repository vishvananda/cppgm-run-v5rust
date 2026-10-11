#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
pa_dir="$(cd "$script_dir/.." && pwd)"

docker run --rm \
  -v "$pa_dir:/work" \
  -w /work/grammar \
  ubuntu:22.04 \
  bash -lc "sed '/^[[:space:]]*#/d' ../pa14.gram | ./gramparse"
