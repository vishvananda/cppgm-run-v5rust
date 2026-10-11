#!/bin/sh

if [ "$1" != "-o" ] || [ $# -lt 3 ]; then
  echo "usage: $0 -o <outfile> <input>..." >&2
  exit 1
fi

outfile=$2
shift 2

exec "${CPPGM_LOWIR2NATIVE_APP:-../dev/lowir2native}" --dump-machine-ir "$outfile" "$@"
