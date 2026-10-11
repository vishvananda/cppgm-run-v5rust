#!/bin/sh

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd) || exit 1
. "$SCRIPT_DIR/cppgm-debugger-common.sh"

if [ "$1" != "-o" ] || [ $# -lt 3 ]; then
  echo "usage: $0 -o <outfile> <input>..." >&2
  exit 1
fi

cppgm_select_debugger || exit 1

outfile=$2
input=$3
tmpdir=$(mktemp -d /tmp/cppgm-lldb-o0.XXXXXX) || exit 1
src="$tmpdir/source.cpp"
exe="$tmpdir/sample"
cp "$input" "$src" || exit 1

cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT INT TERM

"${CPPGM_CPPGM_APP:-../dev/cppgm++}" -gline-tables-only -O0 -o "$exe" "$src" || exit 1

cppgm_run_debugger_frame_and_vars "$exe" "$src" 3 'x y' > "$outfile"
