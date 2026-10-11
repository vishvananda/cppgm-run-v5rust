#!/bin/sh
exec "${CPPGM_LOWIROPT_APP:-../dev/lowiropt}" -O1 "$@"
