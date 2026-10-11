#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

resolve_host_cxx() {
  local candidate
  for candidate in \
    "${CPPGM_HOST_CXX:-}" \
    "${HOST_CXX:-}" \
    /usr/local/opt/llvm/bin/clang++ \
    /opt/homebrew/opt/llvm/bin/clang++ \
    clang++-22 \
    clang++ \
    c++; do
    if [[ -z "$candidate" ]]; then
      continue
    fi
    if [[ "$candidate" == */* ]]; then
      if [[ -x "$candidate" ]]; then
        printf '%s\n' "$candidate"
        return 0
      fi
      continue
    fi
    if command -v "$candidate" >/dev/null 2>&1; then
      command -v "$candidate"
      return 0
    fi
  done
  echo "Unable to resolve host clang++" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Usage: scripts/bench_ppexpr.sh [options]

Benchmark dev/ppexpr on pa3/tests/300-triple.t with cached build roots.

Options:
  --work-root PATH       Cache/build root (default: /tmp/cppgm-ppexpr-bench)
  --levels CSV           Optimization levels to run (default: O0,O1,O2)
  --include-o3           Append O3 to the selected levels
  --host-cxx PATH        Host compiler for control builds and cppgm++ bootstrap
  --self-compiler PATH   Use an existing cppgm++ instead of caching one here
  --host-only            Run only host clang++ control builds
  --self-only            Run only self-hosted cppgm++ builds
  --rebuild-compiler     Force rebuilding cached host-built cppgm++
  --rebuild-host         Force rebuilding host ppexpr binaries
  --rebuild-self         Force rebuilding self-hosted ppexpr binaries
  --no-verify            Skip stdout verification against pa3/tests/300-triple.ref
  --test-input PATH      Override benchmark input
  --ref-output PATH      Override expected stdout
  --help                 Show this message

Notes:
  - The compiler only has distinct O0/O1/O2 modes internally; O3 aliases O2.
  - Cached binaries are reused until a relevant dev/*.cpp|*.h|*.mk|*.inc file
    is newer than the cached binary, or a rebuild flag is passed.
EOF
}

work_root="${WORK_ROOT:-/tmp/cppgm-ppexpr-bench}"
levels_csv="${LEVELS:-O0,O1,O2}"
run_self=1
run_host=1
rebuild_compiler=0
rebuild_host=0
rebuild_self=0
verify_output=1
test_input="$repo_root/pa3/tests/300-triple.t"
ref_output="$repo_root/pa3/tests/300-triple.ref"
host_cxx="${HOST_CXX:-}"
self_compiler=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --work-root)
      work_root="$2"
      shift 2
      ;;
    --levels)
      levels_csv="$2"
      shift 2
      ;;
    --include-o3)
      levels_csv="${levels_csv},O3"
      shift
      ;;
    --host-cxx)
      host_cxx="$2"
      shift 2
      ;;
    --self-compiler)
      self_compiler="$2"
      shift 2
      ;;
    --host-only)
      run_self=0
      run_host=1
      shift
      ;;
    --self-only)
      run_self=1
      run_host=0
      shift
      ;;
    --rebuild-compiler)
      rebuild_compiler=1
      shift
      ;;
    --rebuild-host)
      rebuild_host=1
      shift
      ;;
    --rebuild-self)
      rebuild_self=1
      shift
      ;;
    --no-verify)
      verify_output=0
      shift
      ;;
    --test-input)
      test_input="$2"
      shift 2
      ;;
    --ref-output)
      ref_output="$2"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

host_cxx="${host_cxx:-$(resolve_host_cxx)}"
mkdir -p "$work_root/bin" "$work_root/results"

normalize_levels() {
  local raw="$1"
  local part
  local level
  local seen=","
  local out=()
  raw="${raw// /,}"
  IFS=',' read -r -a parts <<< "$raw"
  for part in "${parts[@]}"; do
    [[ -z "$part" ]] && continue
    level="$(printf '%s' "$part" | tr '[:lower:]' '[:upper:]')"
    case "$level" in
      O0|O1|O2|O3) ;;
      *)
        echo "Unsupported optimization level: $part" >&2
        exit 1
        ;;
    esac
    if [[ "$seen" != *",$level,"* ]]; then
      out+=("$level")
      seen+="$level,"
    fi
  done
  if [[ ${#out[@]} -eq 0 ]]; then
    echo "No optimization levels selected" >&2
    exit 1
  fi
  printf '%s\n' "${out[@]}"
}

levels=()
while IFS= read -r level; do
  levels+=("$level")
done < <(normalize_levels "$levels_csv")

if printf '%s\n' "${levels[@]}" | grep -qx 'O3'; then
  echo "Note: compiler -O3 aliases internal -O2; keeping O3 only because it was requested." >&2
fi

source_is_newer_than() {
  local target="$1"
  find "$repo_root/dev" -type f \
    \( -name '*.cpp' -o -name '*.h' -o -name '*.mk' -o -name '*.inc' \) \
    -newer "$target" -print -quit | grep -q .
}

meta_matches() {
  local meta_file="$1"
  local expected="$2"
  [[ -f "$meta_file" && "$(cat "$meta_file")" == "$expected" ]]
}

file_fingerprint() {
  local path="$1"
  if [[ ! -e "$path" ]]; then
    return 1
  fi
  if stat -f '%N|%z|%m' "$path" >/dev/null 2>&1; then
    stat -f '%N|%z|%m' "$path"
    return 0
  fi
  stat -c '%n|%s|%Y' "$path"
}

ensure_obj_root_matches() {
  local obj_root="$1"
  local meta_file="$2"
  local expected="$3"
  if [[ -d "$obj_root" ]] && ! meta_matches "$meta_file" "$expected"; then
    rm -rf "$obj_root"
  fi
  mkdir -p "$obj_root"
}

needs_rebuild() {
  local target="$1"
  local force="$2"
  local extra_dep="${3:-}"
  if [[ "$force" == "1" || ! -x "$target" ]]; then
    return 0
  fi
  if [[ -n "$extra_dep" && "$extra_dep" -nt "$target" ]]; then
    return 0
  fi
  if source_is_newer_than "$target"; then
    return 0
  fi
  return 1
}

cc_flags_for() {
  local opt_flag="$1"
  local obj_root="$2"
  mkdir -p "$obj_root"
  local obj_abs
  obj_abs="$(cd "$obj_root" && pwd)"
  printf "%s" "-std=gnu++11 -Wall ${opt_flag} ${CPPGM_STDLIB_FLAGS:-} -DCPPGM_DEFAULT_HOST_CXX='\"${host_cxx}\"' -DCPPGM_DEFAULT_OBJECT_ROOT='\"${obj_abs}\"'"
}

build_cppgm_if_needed() {
  if [[ -n "$self_compiler" ]]; then
    if [[ ! -x "$self_compiler" ]]; then
      echo "Provided --self-compiler is not executable: $self_compiler" >&2
      exit 1
    fi
    printf '%s\n' "$self_compiler"
    return 0
  fi

  local compiler_bin="$work_root/bin/cppgm++"
  local obj_root="$work_root/obj-cppgm-host"
  local obj_meta="${obj_root}/.bench.meta"
  local meta_file="${compiler_bin}.meta"
  local host_id
  host_id="$(file_fingerprint "$host_cxx")"
  local cc_flags
  cc_flags="$(cc_flags_for -O2 "$obj_root")"
  local build_key
  build_key="tool=cppgm++|compiler=${host_id}|host=${host_id}|obj=${obj_root}|flags=${cc_flags}"
  ensure_obj_root_matches "$obj_root" "$obj_meta" "$build_key"
  if needs_rebuild "$compiler_bin" "$rebuild_compiler" || ! meta_matches "$meta_file" "$build_key"; then
    echo "Building cached cppgm++ with host compiler" >&2
    rm -f "$repo_root/dev/cppgm++"
    make -s -C "$repo_root/dev" cppgm++ \
      "CXX=$host_cxx" \
      "CPPGM_HOST_CXX=$host_cxx" \
      "CPPGM_TEST_RUNNER=0" \
      "OBJ=$obj_root" \
      "CC_FLAGS=$cc_flags" \
      "ENTRY_CC_FLAGS=$cc_flags" >&2
    cp "$repo_root/dev/cppgm++" "$compiler_bin"
    printf '%s\n' "$build_key" > "$meta_file"
    printf '%s\n' "$build_key" > "$obj_meta"
  else
    echo "Reusing cached cppgm++" >&2
  fi
  printf '%s\n' "$compiler_bin"
}

build_ppexpr_if_needed() {
  local mode="$1"
  local level="$2"
  local compiler="$3"
  local force="$4"
  local extra_dep="${5:-}"
  local bin="$work_root/bin/ppexpr-${mode}-${level}"
  local obj_root="$work_root/obj-${mode}-${level}"
  local obj_meta="${obj_root}/.bench.meta"
  local meta_file="${bin}.meta"
  local compiler_id
  compiler_id="$(file_fingerprint "$compiler")"
  local host_id
  host_id="$(file_fingerprint "$host_cxx")"
  local cc_flags
  cc_flags="$(cc_flags_for "-${level}" "$obj_root")"
  local build_key
  build_key="tool=ppexpr|mode=${mode}|level=${level}|compiler=${compiler_id}|host=${host_id}|obj=${obj_root}|flags=${cc_flags}"
  ensure_obj_root_matches "$obj_root" "$obj_meta" "$build_key"

  if needs_rebuild "$bin" "$force" "$extra_dep" || ! meta_matches "$meta_file" "$build_key"; then
    echo "Building ${mode} ppexpr ${level}" >&2
    rm -f "$repo_root/dev/ppexpr"
    make -s -C "$repo_root/dev" ppexpr \
      "CXX=$compiler" \
      "CPPGM_HOST_CXX=$host_cxx" \
      "CPPGM_TEST_RUNNER=0" \
      "OBJ=$obj_root" \
      "CC_FLAGS=$cc_flags" \
      "ENTRY_CC_FLAGS=$cc_flags" >&2
    cp "$repo_root/dev/ppexpr" "$bin"
    printf '%s\n' "$build_key" > "$meta_file"
    printf '%s\n' "$build_key" > "$obj_meta"
  else
    echo "Reusing cached ${mode} ppexpr ${level}" >&2
  fi

  printf '%s\n' "$bin"
}

verify_case() {
  local bin="$1"
  local prefix="$2"
  "$bin" < "$test_input" > "${prefix}.stdout" 2> "${prefix}.stderr"
  if ! cmp -s "${prefix}.stdout" "$ref_output"; then
    echo "stdout mismatch for $bin" >&2
    diff -u "$ref_output" "${prefix}.stdout" || true
    exit 1
  fi
}

time_case() {
  local bin="$1"
  local prefix="$2"
  /usr/bin/time -lp -o "${prefix}.time" "$bin" < "$test_input" > /dev/null 2>/dev/null
}

record_summary_line() {
  local mode="$1"
  local level="$2"
  local prefix="$3"
  local real user sys
  real="$(awk '$1=="real" {print $2}' "${prefix}.time")"
  user="$(awk '$1=="user" {print $2}' "${prefix}.time")"
  sys="$(awk '$1=="sys" {print $2}' "${prefix}.time")"
  printf '%s %s real=%s user=%s sys=%s\n' "$mode" "$level" "$real" "$user" "$sys"
}

summary_file="$work_root/results/summary.txt"
: > "$summary_file"

cached_cppgm=""
if [[ "$run_self" == "1" ]]; then
  cached_cppgm="$(build_cppgm_if_needed)"
fi

for level in "${levels[@]}"; do
  if [[ "$run_self" == "1" ]]; then
    prefix="$work_root/results/self-${level}"
    self_bin="$(build_ppexpr_if_needed self "$level" "$cached_cppgm" "$rebuild_self" "$cached_cppgm")"
    if [[ "$verify_output" == "1" ]]; then
      verify_case "$self_bin" "$prefix"
    fi
    time_case "$self_bin" "$prefix"
    record_summary_line self "$level" "$prefix" | tee -a "$summary_file"
  fi

  if [[ "$run_host" == "1" ]]; then
    prefix="$work_root/results/host-${level}"
    host_bin="$(build_ppexpr_if_needed host "$level" "$host_cxx" "$rebuild_host")"
    if [[ "$verify_output" == "1" ]]; then
      verify_case "$host_bin" "$prefix"
    fi
    time_case "$host_bin" "$prefix"
    record_summary_line host "$level" "$prefix" | tee -a "$summary_file"
  fi
done

echo "Summary written to $summary_file"
