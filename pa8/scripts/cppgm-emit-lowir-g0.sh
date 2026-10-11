#!/bin/sh
exec "${CPPGM_CPPGM_APP:-../dev/cppgm++}" --emit-lowir -g0 -O0 "$@"
