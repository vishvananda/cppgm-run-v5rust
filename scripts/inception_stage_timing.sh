#!/bin/bash
# The gold-standard self/host measurement: time the two producer stages of
# inception on identical inputs with identical parallelism.  The selfhost
# stage compiles the compiler with the host-built dev/cppgm++; the inception
# stage compiles it again with the self-built compiler.  The wall ratio of
# the second over the first is the number PLAN-LIBCXX-PERFORMANCE.md calls
# the gold standard, and it is only a fair ratio when both stages get the
# same job count, which pa34 does not do by default (the inception stage is
# capped at 8).
#
#   scripts/inception_stage_timing.sh <default|clang|clang-libcxx> <1|3> [jobs] [logfile]
#
# The three pa34 roots of the cell are moved aside first (never deleted), so
# the run starts from nothing, and dev/cppgm++ is rebuilt for the cell since
# it is one path shared by all cells.
set -eu
cd "$(dirname "$0")/.."
cell=$1; opt=$2; jobs=${3:-$(nproc)}; log=${4:-/dev/stdout}
case "$cell" in
  default)
    root=obj; devargs=(); hostcxx=g++
    cellargs=(CPPGM_HOST_CXX=g++) ;;
  clang)
    root=obj-clang; hostcxx=clang++
    devargs=(CXX=clang++ CPPGM_HOST_CXX=clang++ OBJ=../obj-clang)
    cellargs=(CPPGM_HOST_CXX=clang++ OBJ=../obj-clang INCEPTION_OBJ_ROOT_BASE=../obj-clang/pa34) ;;
  clang-libcxx)
    root=obj-clang-libcxx; hostcxx=clang++
    devargs=(CXX=clang++ CPPGM_HOST_CXX=clang++ CPPGM_STDLIB_FLAGS=-stdlib=libc++ OBJ=../obj-clang-libcxx)
    cellargs=(CPPGM_HOST_CXX=clang++ CPPGM_STDLIB_FLAGS=-stdlib=libc++ OBJ=../obj-clang-libcxx INCEPTION_OBJ_ROOT_BASE=../obj-clang-libcxx/pa34) ;;
  *) echo "unknown cell: $cell" >&2; exit 2 ;;
esac
make -C dev cppgm++ -j"$jobs" "${devargs[@]}" >/dev/null
ts=$(date +%s)
for d in selfhost inception bin; do
  if [ -d "$root/pa34/$d" ]; then mv "$root/pa34/$d" "$root/pa34/$d.timing.$ts"; fi
done
common=(CXX=../dev/cppgm++ "${cellargs[@]}" INCEPTION_SELFHOST_OPT_LEVEL="$opt" INCEPTION_BUILD_JOBS="$jobs" INCEPTION_DEFAULT_BUILD_JOB_CAP=0)
printf '=== cell=%s opt=O%s jobs=%s host=%s tree=%s ===\n' "$cell" "$opt" "$jobs" "$hostcxx" "$(git rev-parse --short HEAD)" >> "$log"
/usr/bin/time -f "selfhost   wall %e s  user %U s  sys %S s" -a -o "$log" \
  make -C pa34 "${common[@]}" cppgm++-self -j"$jobs" >/dev/null
/usr/bin/time -f "inception  wall %e s  user %U s  sys %S s" -a -o "$log" \
  make -C pa34 "${common[@]}" cppgm++-inception -j"$jobs" >/dev/null
make -C pa34 "${common[@]}" compare-cppgm++-inception -j"$jobs" 2>&1 | grep -E 'MATCH|mismatch' >> "$log"
awk '/^selfhost/ {s=$3} /^inception/ {i=$3} END {if (s > 0) printf "ratio      wall %.3fx\n", i / s}' "$log" >> "$log"
