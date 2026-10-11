#!/bin/bash

set -euo pipefail

root="${1:-tests/preproc}"

if [ ! -d "$root" ]; then
  echo "missing preproc test root: $root" >&2
  exit 1
fi

search_tool=(rg -n)
if ! command -v rg >/dev/null 2>&1; then
  search_tool=(grep -nE)
fi

if "${search_tool[@]}" \
  'mac_platform|linux_platform|__apple_build_version__|__PTRDIFF_TYPE__|__SIZE_TYPE__|__FLT_MIN__|__DBL_MIN__|__LDBL_MIN__|literal 1\.17549435e-38F|literal 2\.2250738585072014e-308|literal 3\.36210314311209350626e-4932L' \
  "$root"/*.ref; then
  echo "default PA29 preprocessor refs must stay host-agnostic; move host-pinned snapshots out of $root" >&2
  exit 1
fi
