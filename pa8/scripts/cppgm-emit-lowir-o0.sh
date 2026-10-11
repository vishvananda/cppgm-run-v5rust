#!/bin/sh
exec "${CPPGM_CPPGM_APP:-../dev/cppgm++}" --emit-lowir -gline-tables-only -O0 "$@"
