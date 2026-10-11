#!/usr/bin/env bash
set -euo pipefail

target=$(basename "$0")
if [[ "$target" != *-ref ]]; then
  echo "run_reference_binary: expected to be invoked through a *-ref wrapper" >&2
  exit 2
fi
target=${target%-ref}

if [[ "$0" == */* ]]; then
  search_dir=$(cd "$(dirname "$0")" && pwd)
else
  search_dir=$(pwd)
fi

repo_root=$search_dir
while [[ "$repo_root" != "/" ]]; do
  if [[ -f "$repo_root/scripts/ensure_reference_binaries.pl" ]]; then
    break
  fi
  repo_root=$(dirname "$repo_root")
done

if [[ ! -f "$repo_root/scripts/ensure_reference_binaries.pl" ]]; then
  echo "run_reference_binary: unable to find repository root" >&2
  exit 2
fi

perl "$repo_root/scripts/ensure_reference_binaries.pl" "$target"
exec "$repo_root/reference-binaries/$target" "$@"
