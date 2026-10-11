#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"

cppgm_bin="${CPPGM_CMAKE_COMPILER:-$repo_root/dev/cppgm++}"
host_query_cxx="${CPPGM_CMAKE_QUERY_CXX:-${CPPGM_HOST_CXX:-${CXX:-g++}}}"

if [[ ! -x "$cppgm_bin" ]]; then
  echo "cppgm-cmake-wrapper: compiler not found: $cppgm_bin" >&2
  exit 1
fi

if [[ $# -eq 0 ]]; then
  exec "$cppgm_bin"
fi

case "${1:-}" in
  --version|-v|-dumpmachine|-dumpversion|-print-search-dirs)
    exec "$host_query_cxx" "$@"
    ;;
esac

declare -a args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -c|-E)
      args+=("$1")
      shift
      ;;
    -o|-I|-L|-l|--target)
      if [[ $# -lt 2 ]]; then
        echo "cppgm-cmake-wrapper: missing argument after $1" >&2
        exit 1
      fi
      args+=("$1" "$2")
      shift 2
      ;;
    -I*|-L*|-l*)
      args+=("$1")
      shift
      ;;
    -isystem)
      if [[ $# -lt 2 ]]; then
        echo "cppgm-cmake-wrapper: missing path after -isystem" >&2
        exit 1
      fi
      args+=("$1" "$2")
      shift 2
      ;;
    -isystem*)
      args+=("$1")
      shift
      ;;
    -isysroot)
      if [[ $# -lt 2 ]]; then
        echo "cppgm-cmake-wrapper: missing path after -isysroot" >&2
        exit 1
      fi
      shift 2
      ;;
    -D)
      if [[ $# -lt 2 ]]; then
        echo "cppgm-cmake-wrapper: missing macro after -D" >&2
        exit 1
      fi
      args+=("$1" "$2")
      shift 2
      ;;
    -D*)
      args+=("$1")
      shift
      ;;
    -U)
      if [[ $# -lt 2 ]]; then
        echo "cppgm-cmake-wrapper: missing macro after -U" >&2
        exit 1
      fi
      args+=("$1" "$2")
      shift 2
      ;;
    -U*)
      args+=("$1")
      shift
      ;;
    -include)
      if [[ $# -lt 2 ]]; then
        echo "cppgm-cmake-wrapper: missing file after -include" >&2
        exit 1
      fi
      args+=("$1" "$2")
      shift 2
      ;;
    -std=*|-stdlib=*|-pthread|-pipe|-no-pie|-g|-g[0-9]|-O|-O[0-9sSzZ]|-w)
      shift
      ;;
    -W*)
      shift
      ;;
    -f*)
      shift
      ;;
    -m*)
      shift
      ;;
    --sysroot=*)
      shift
      ;;
    --*)
      echo "cppgm-cmake-wrapper: unsupported option: $1" >&2
      exit 1
      ;;
    -*)
      echo "cppgm-cmake-wrapper: unsupported option: $1" >&2
      exit 1
      ;;
    *)
      args+=("$1")
      shift
      ;;
  esac
done

exec "$cppgm_bin" "${args[@]}"
