#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage: scripts/export_student_repo.sh [--force] DEST

Generate a student-facing cppgm repository at DEST.

Options:
  --force   Remove DEST before exporting.

Environment:
  CXX                  Compiler used to build exported reference binaries.
  CPPGM_HOST_CXX       Host compiler recorded in cppgm++ defaults.
  CPPGM_STDLIB_FLAGS   Extra standard-library flags for the host compiler.
  CPPGM_REFERENCE_BUNDLE_OUT
                       Path where the reference-binary bundle should be written.
  CPPGM_REFERENCE_BUNDLE_URL
                       URL embedded in the student manifest for auto-download.
  CPPGM_EXPORT_ALLOW_NON_LINUX
                       Set to 1 for local smoke exports on non-Linux hosts.
EOF
}

force=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --force)
      force=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      echo "export_student_repo: unknown option: $1" >&2
      usage
      exit 2
      ;;
    *)
      break
      ;;
  esac
done

if [ "$#" -ne 1 ]; then
  usage
  exit 2
fi

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/.." && pwd)
dest=$1
if [[ "$dest" != /* ]]; then
  dest=$(cd "$(dirname "$dest")" && pwd)/$(basename "$dest")
fi

host_uname=$(uname -s 2>/dev/null || echo unknown)
if [ "$host_uname" != "Linux" ] && [ "${CPPGM_EXPORT_ALLOW_NON_LINUX:-0}" != "1" ]; then
  echo "export_student_repo: student reference binaries must be generated on Linux" >&2
  echo "export_student_repo: set CPPGM_EXPORT_ALLOW_NON_LINUX=1 only for local smoke exports" >&2
  exit 2
fi

if [ -e "$dest" ]; then
  if [ "$force" -ne 1 ]; then
    echo "export_student_repo: destination exists: $dest" >&2
    echo "export_student_repo: pass --force to replace it" >&2
    exit 2
  fi
  rm -rf "$dest"
fi

mkdir -p "$dest"

copy_tracked_paths() {
  (
    cd "$repo_root"
    git ls-files -z -- "$@" |
      perl -0ne '
        BEGIN {
          open my $manifest, "<", shift @ARGV or die "student document manifest: $!";
          %documents = map { $_ => 1 } split /\n/, do { local $/; <$manifest> };
        }
        chomp;
        # Export reviewed student documents only, including public doc/ texts.
        next if (m{\.md$}i || m{^doc/}) && !$documents{$_};
        next if m{(^|/)[^/]+-ref$};
        next if m{\.py$} && !m{^scripts/(?:check_lowir_seams|lowir_seam_rewrite)\.py$};
        next if m{\.diff$};
        next if m{(^|/)[^/]+\.my(?:\.|$)};
        # Tests-first audit controls run only under the maintainer
        # scripts/check_pending_audit_regressions.py, which does not ship.
        next if m{^pa[0-9]+/tests/controls/[^/]*-audit-[^/]*$};
        print "$_\0";
      ' "$script_dir/student_export_documents.txt"
  ) | rsync -a --from0 --files-from=- "$repo_root/" "$dest/"
}

list_tracked_reference_outputs() {
  (
    cd "$repo_root"
    git ls-files -- 'pa[0-9]*' student.tests |
      perl -ne 'print if m{(^|/)[^/]+\.ref(?:\.|$)}'
  )
}

list_exported_reference_outputs() {
  (
    cd "$dest"
    find pa[0-9]* student.tests -type f -print 2>/dev/null |
      perl -ne 's{^\./}{}; print if m{(^|/)[^/]+\.ref(?:\.|$)}'
  )
}

reference_outputs_match() {
  local source_path="$1"
  local export_path="$2"
  local ref_path="$3"

  cmp -s "$source_path" "$export_path" && return 0

  case "$ref_path" in
    *.ref.mir)
      diff -q \
        <(perl "$repo_root/scripts/compare_results_common.pl" normalize-machine-ir "$source_path") \
        <(perl "$repo_root/scripts/compare_results_common.pl" normalize-machine-ir "$export_path") \
        >/dev/null
      return
      ;;
    *.ref.cmir)
      diff -q \
        <(perl "$repo_root/scripts/compare_results_common.pl" canonicalize-machine-ir "$source_path") \
        <(perl "$repo_root/scripts/compare_results_common.pl" canonicalize-machine-ir "$export_path") \
        >/dev/null
      return
      ;;
  esac

  perl -0777 -e '
    sub read_ref {
      my ($path) = @_;
      open(my $fh, "<", $path) or exit 2;
      local $/;
      my $data = <$fh>;
      close($fh) or exit 2;
      $data = "" if !defined($data);
      $data =~ s/\s+\z//;
      return $data;
    }
    exit(read_ref($ARGV[0]) eq read_ref($ARGV[1]) ? 0 : 1);
  ' "$source_path" "$export_path"
}

is_failed_stdout_diagnostic() {
  local root="$1"
  local ref_path="$2"
  case "$ref_path" in
    *.ref.stdout) ;;
    *) return 1 ;;
  esac

  local status_path="$root/${ref_path%.stdout}.exit_status"
  [ -f "$status_path" ] || return 1
  local status
  status=$(tr -d '[:space:]' < "$status_path")
  [ "$status" != "EXIT_SUCCESS" ]
}

verify_regenerated_reference_outputs() {
  echo "==> Verifying regenerated reference outputs match source"
  local tmp_dir
  tmp_dir=$(mktemp -d)
  local source_list="$tmp_dir/source-refs"
  local dest_list="$tmp_dir/export-refs"
  local missing_list="$tmp_dir/missing-refs"
  local extra_list="$tmp_dir/extra-refs"
  local unexpected_extra_list="$tmp_dir/unexpected-extra-refs"
  local exported_diagnostic_list="$tmp_dir/exported-diagnostic-refs"
  list_tracked_reference_outputs | LC_ALL=C sort > "$source_list"
  list_exported_reference_outputs | LC_ALL=C sort > "$dest_list"

  local path
  while IFS= read -r path; do
    if is_failed_stdout_diagnostic "$repo_root" "$path"; then
      echo "export_student_repo: failed-case stdout must be generated by the Linux export, not tracked in source: $path" >&2
      rm -rf "$tmp_dir"
      exit 1
    fi
  done < "$source_list"

  # comm needs its inputs in its own collation order: both lists were sorted
  # in the C locale, so compare them there too.
  LC_ALL=C comm -23 "$source_list" "$dest_list" > "$missing_list"
  if [ -s "$missing_list" ]; then
    local missing_count
    missing_count=$(wc -l < "$missing_list")
    echo "export_student_repo: $missing_count tracked reference output file(s) were not regenerated:" >&2
    sed -n '1,20p' "$missing_list" >&2
    if [ "$missing_count" -gt 20 ]; then
      echo "export_student_repo: ... and $((missing_count - 20)) more" >&2
    fi
    echo "export_student_repo: remove stale source refs or fix the ref-test target that skipped them" >&2
    rm -rf "$tmp_dir"
    exit 1
  fi

  LC_ALL=C comm -13 "$source_list" "$dest_list" > "$extra_list"
  if [ -s "$extra_list" ]; then
    : > "$unexpected_extra_list"
    : > "$exported_diagnostic_list"
    while IFS= read -r path; do
      if is_failed_stdout_diagnostic "$dest" "$path"; then
        printf '%s\n' "$path" >> "$exported_diagnostic_list"
      else
        printf '%s\n' "$path" >> "$unexpected_extra_list"
      fi
    done < "$extra_list"
    if [ -s "$unexpected_extra_list" ]; then
      local unexpected_extra_count
      unexpected_extra_count=$(wc -l < "$unexpected_extra_list")
      echo "==> Removing $unexpected_extra_count regenerated reference sidecar files not tracked in source"
      while IFS= read -r path; do
        rm -f "$dest/$path"
      done < "$unexpected_extra_list"
    fi
  fi

  local mismatch_count=0
  local shown_count=0
  while IFS= read -r path; do
    if ! reference_outputs_match "$repo_root/$path" "$dest/$path" "$path"; then
      echo "export_student_repo: reference output mismatch: $path" >&2
      if [ "$shown_count" -lt 20 ]; then
        diff -u --label "source/$path" --label "export/$path" "$repo_root/$path" "$dest/$path" >&2 || true
        shown_count=$((shown_count + 1))
      fi
      mismatch_count=$((mismatch_count + 1))
    fi
  done < "$source_list"

  local ref_count
  local exit_status_count
  local exported_diagnostic_count=0
  ref_count=$(wc -l < "$source_list")
  exit_status_count=$(grep -c '\.exit_status$' "$source_list" || true)
  if [ -f "$exported_diagnostic_list" ]; then
    exported_diagnostic_count=$(wc -l < "$exported_diagnostic_list")
  fi

  if [ "$mismatch_count" -ne 0 ]; then
    rm -rf "$tmp_dir"
    echo "export_student_repo: $mismatch_count regenerated reference output file(s) differ from source" >&2
    exit 1
  fi
  rm -rf "$tmp_dir"
  echo "==> Verified $ref_count portable reference output files ($exit_status_count exit status files)"
  echo "==> Retained $exported_diagnostic_count Linux-generated failed-case stdout examples"
}

# Student exit criteria judge the course contract. Keep the solution-specific
# regression target available for explicit maintainer/design investigation.
sanitize_student_makefile_defaults() {
  perl -0pi -e '
    s/^test: test-course test-regression$/test: test-course/m;
    s/GNUMAKE = \$\(firstword \$\(wildcard \/opt\/homebrew\/opt\/make\/libexec\/gnubin\/make \/usr\/local\/opt\/make\/libexec\/gnubin\/make\)\)\nifneq \(\$\(GNUMAKE\),\)\nMAKE := \$\(GNUMAKE\)\nendif\n//g;
    s/sysctl -n hw\.ncpu 2>\/dev\/null \|\| //g;
    s/LLVM_CXX_LOCAL = \/usr\/local\/opt\/llvm\/bin\/clang\+\+\nLLVM_CXX_HOMEBREW = \/opt\/homebrew\/opt\/llvm\/bin\/clang\+\+\n(?:ifeq \(\$\(HOST_UNAME_S\),Darwin\)\n)?ifeq \(\$\(wildcard \$\(LLVM_CXX_LOCAL\)\),\)\nifneq \(\$\(wildcard \$\(LLVM_CXX_HOMEBREW\)\),\)\nLLVM_CXX_DEFAULT = \$\(LLVM_CXX_HOMEBREW\)\nendif\nelse\nLLVM_CXX_DEFAULT = \$\(LLVM_CXX_LOCAL\)\nendif\n(?:ifdef LLVM_CXX_DEFAULT\nHOST_CXX_DEFAULT = \$\(LLVM_CXX_DEFAULT\)\nelse\nHOST_CXX_DEFAULT = clang\+\+\nendif\nelse\nHOST_CXX_DEFAULT = g\+\+\nendif\n|)//g;
    s/ifeq \(\$\(origin CXX\), default\)\n(?:ifdef LLVM_CXX_DEFAULT\nCXX := \$\(LLVM_CXX_DEFAULT\)\nelse\nCXX := (?:clang|g)\+\+\nendif|CXX := \$\(HOST_CXX_DEFAULT\))\nendif/ifeq (\$(origin CXX), default)\nCXX := g++\nendif/g;
    s/ifeq \(\$\(origin CPPGM_HOST_CXX\), undefined\)\nifeq \(\$\(abspath \$\(CXX\)\),\$\(abspath \.\.\/dev\/cppgm\+\+\)\)\n(?:ifdef LLVM_CXX_DEFAULT\n\tCPPGM_HOST_CXX := \$\(LLVM_CXX_DEFAULT\)\nelse\n\tCPPGM_HOST_CXX := clang\+\+\nendif|CPPGM_HOST_CXX := \$\(HOST_CXX_DEFAULT\))\nelse\n\t?CPPGM_HOST_CXX := \$\(CXX\)\nendif\nendif/ifeq (\$(origin CPPGM_HOST_CXX), undefined)\nifeq (\$(abspath \$(CXX)),\$(abspath ..\/dev\/cppgm++))\n\tCPPGM_HOST_CXX := g++\nelse\n\tCPPGM_HOST_CXX := \$(CXX)\nendif\nendif/g;
    s/INCEPTION_LINKER_DETERMINISM_FLAGS =\nifeq \(\$\(HOST_UNAME_S\),Darwin\)\nINCEPTION_LINKER_DETERMINISM_FLAGS \+= -Wl,-reproducible\nendif/INCEPTION_LINKER_DETERMINISM_FLAGS =/g;
  ' "$@"
}

# The root Makefile carries maintainer-only targets: the architecture audits
# (scripts/audit_*.pl), the harness unit tests and the telemetry-off build
# check (scripts/tests/*.py), whose scripts never ship, and test-cells, which
# walks the course's three toolchain cells.  Drop those targets with their
# comments, the audit prerequisite of test-report-nobuild, and their .PHONY
# names.  The with-clang-% and with-clang-libcxx-% prefixes stay: they run
# one target with a chosen toolchain.
STUDENT_ROOT_MAKEFILE_DROPPED_TARGETS='test-cells test-harness test-seams build-telemetry-off test-telemetry-off'
sanitize_student_root_makefile() {
  perl -0pi -e '
    s/^test-report-nobuild: audit-compiler-exceptions$/test-report-nobuild:/m;
    s/(?:^#[^\n]*\n)*^audit-[a-z-]+:[^\n]*\n\t\@perl scripts\/audit_[a-z_]+\.pl[^\n]*\n\n?//mg;
    s/^build-telemetry-off:\n(?:\t[^\n]*\n)+\n?//m;
    s/^test-telemetry-off: build build-telemetry-off\n(?:\t[^\n]*\n)+\n?//m;
    s/(?:^#[^\n]*\n)*^test-cells:\n(?:\t[^\n]*\n)+\n?//m;
    s/(?:^#[^\n]*\n)*^HARNESS_TESTS = \\\n(?:\t[^\n]*\n)+\n?^test-harness:\n(?:\t[^\n]*\n)+\n?//m;
    s/\b(?:audit-[a-z-]+|build-telemetry-off|test-telemetry-off|test-harness|test-cells) //g;
    s/^\tif \[ -d pa11\/tests\/general \]; then \\\n(?:\t[^\n]*\n)*?\tfi; \\\n//m;
  ' "$@"
}

# The PA11 seams lane checks the comparison harness against the contract text,
# which is maintainer work: drop the target and its scripts from the export.
sanitize_student_seams_makefile() {
  if [ -f "$1" ]; then
    perl -0pi -e '
      s/^#[^\n]*\n(?:#[^\n]*\n)*test-seams:\n(?:\t[^\n]*\n)+\n?//m;
      s/ test-seams\b//g;
    ' "$1"
  fi
}

# Regenerate and verify the solution references first, then remove PA33's
# private allocator regressions and profiling material from the student tree.
prune_student_pa33() {
  rm -rf "$dest/pa33/maintainer" "$dest/pa33/tests/regression"
  find "$dest/pa33/tests" -type f -name '*.ref.ir' -delete
  perl -0pi -e 's/\n-include maintainer\/Makefile\n/\n/' "$dest/pa33/Makefile"
  local script
  for script in check_cppgm_function_census \
    check_lowir_native_structural_controls check_lowir_native_parameter_addresses \
    check_lowir_native_tail_jump check_lowir_native_scalar_query \
    check_lowir_native_survivor_properties; do
    rm -f "$dest/scripts/$script.pl"
  done
}

# PA35's release check calibrates the course solution itself: drop its
# maintainer directory and the Makefile line that includes it.
prune_student_pa35() {
  if [ -f "$dest/pa35/Makefile" ]; then
    rm -rf "$dest/pa35/maintainer"
    perl -0pi -e 's/\n-include maintainer\/Makefile\n/\n/' "$dest/pa35/Makefile"
  fi
}

sanitize_linux_student_scripts() {
  local run_script
  run_script="$dest/scripts/run_cpphostinterop_tests_worker.pl"
  if [ -f "$run_script" ]; then
    perl -0pi -e '
      s/\treturn '"'"'macos'"'"' if \$os eq '"'"'darwin'"'"';\n//g;
      s/\tmy \$shared_ext = `uname -s`;\n\tchomp\(\$shared_ext\);\n\t\$shared_ext = \$shared_ext eq '"'"'Darwin'"'"' \? '"'"'dylib'"'"' : '"'"'so'"'"';/\tmy \$shared_ext = '"'"'so'"'"';/g;
      s/\|\\\.dylib//g;
      s/\|dylib//g;
      s@\$flag =~ m/\(\?:\\\.__SHARED_EXT__\|\\\.dylib\|\\\.so\)\$/@\$flag =~ m/(?:\\.__SHARED_EXT__|\\.so)$/@g;
    ' "$run_script"
    perl -pi -e '
      if ($skip_dylib_cmd) {
        if (/^\t\t\t\t: \(\@\{\$ctx->\{host_cc\}\}, '"'"'-shared'"'"', '"'"'-o'"'"', \$sharedlib, \$obj\);$/) {
          $_ = "\t\t\tmy \@cmd = (\@{\$ctx->{host_cc}}, '"'"'-shared'"'"', '"'"'-o'"'"', \$sharedlib, \$obj);\n";
          $skip_dylib_cmd = 0;
        } else {
          $_ = "";
        }
      } elsif (/^\t\t\tmy \@cmd = \$ctx->\{shared_ext\} eq '"'"'dylib'"'"'$/) {
        $skip_dylib_cmd = 1;
        $_ = "";
      }
    ' "$run_script"
  fi

  run_script="$dest/pa8/scripts/cppgm-debugger-common.sh"
  if [ -f "$run_script" ]; then
    perl -0pi -e '
      s/cppgm_sys=\$\(uname -s 2>\/dev\/null \|\| echo unknown\)\n      case "\$cppgm_sys" in\n        Darwin\)\n          cppgm_try_lldb \|\| cppgm_try_gdb \|\| \{\n            echo "missing debugger backend" >&2\n            return 1\n          \}\n          ;;\n        \*\)\n          cppgm_try_gdb \|\| cppgm_try_lldb \|\| \{\n            echo "missing debugger backend" >&2\n            return 1\n          \}\n          ;;\n      esac/cppgm_try_gdb || cppgm_try_lldb || {\n        echo "missing debugger backend" >&2\n        return 1\n      }/g;
    ' "$run_script"
  fi
}

pa_dirs=()
for makefile in "$repo_root"/pa[0-9]*/Makefile; do
  pa_dirs+=("$(basename "$(dirname "$makefile")")")
done

shared_scripts=(
  scripts/CppgmBatchWorker.pm
  scripts/check_object_expectations.pl
  scripts/canonicalize_lowir_native_refs.pl
  scripts/compare_results_common.pl
  scripts/cppgm-cmake-wrapper.sh
  scripts/cppgm_file_audit.pl
  scripts/dump_host_eh_object_facts.pl
  scripts/ensure_reference_binaries.pl
  scripts/pa_run_check_targets.mk
  scripts/run_all_tests_common.pl
  scripts/run_cpphostcompat_compile_worker.pl
  scripts/run_cpphostcompat_preproc_worker.pl
  scripts/run_cpphostinterop_tests_worker.pl
  scripts/run_cpptoolchain_tests_worker.pl
  scripts/run_lowir_link_tests_worker.pl
  scripts/run_lowir_native_tests_worker.pl
  scripts/run_lowir_program_tests_worker.pl
  scripts/run_routed_test_spec.pl
  scripts/run_reference_binary.sh
  scripts/run_with_timeout.pl
  scripts/write_unresolved_symbol_report.pl
  scripts/expect_ir.pl
)
# Copy control checkers for reference regeneration. PA33's private checkers
# are removed with its maintainer lanes after reference verification.
for check_script in "$repo_root"/scripts/check_*.pl; do
  # This compares maintainer compiler builds using a private audit table.
  [ "$(basename "$check_script")" = check_compiler_refactor_outputs.pl ] && continue
  shared_scripts+=("scripts/$(basename "$check_script")")
done

dev_public=(
  dev/.gitignore
  dev/gen_builtin_host_config.pl
)

# The shipped scaffold files: the vocabulary headers of the LowIR, MIR and
# ABI formats, the token-stream interface and dump, the help texts, the
# scaffolds' not-implemented status and the batch test runner.  The course
# solution builds on every one of them as shipped, except not_implemented.h,
# which only the scaffolds use (scripts/audit_scaffold_usage.pl).
dev_support_files=(
  dev/src/abi/itanium/abi_mangle.h
  dev/src/ir_symbol_model.h
  dev/src/lowir_model.h
  dev/src/mir_model.h
  dev/src/native/mir/registers.h
  dev/src/preprocess/tokens/DebugPPTokenStream.h
  dev/src/preprocess/tokens/IPPTokenStream.h
  dev/src/support/not_implemented.h
  dev/src/support/testing/test_runner.cpp
  dev/src/support/tool_help_text.h
)

copy_tracked_paths \
  .gitignore \
  Makefile \
  doc \
  student.tests \
  shared \
  "${pa_dirs[@]}" \
  "${shared_scripts[@]}" \
  "${dev_public[@]}" \
  "${dev_support_files[@]}"

for root_doc in "$repo_root"/docs/student-export-root/*.md; do
  install -m 0644 "$root_doc" "$dest/$(basename "$root_doc")"
done
install -m 0644 "$repo_root"/docs/student-export-root/LICENSE "$dest/LICENSE"
install -m 0644 "$repo_root"/docs/student-export-root/AUTHORS "$dest/AUTHORS"
install -m 0644 "$repo_root"/docs/student-export-root/NOTICE "$dest/NOTICE"

scaffold_pairs=(
  abimangle:abimangle-scaffold.cpp
  pptoken:pptoken-scaffold.cpp
  posttoken:posttoken-scaffold.cpp
  ppexpr:ppexpr-scaffold.cpp
  preproc:preproc-scaffold.cpp
  cppgm++:cppgm++-scaffold.cpp
  lowir:lowir-scaffold.cpp
  lowir2native:lowir2native-scaffold.cpp
  lowiropt:lowiropt-scaffold.cpp
)

for pair in "${scaffold_pairs[@]}"; do
  target=${pair%%:*}
  scaffold=${pair#*:}
  cp -p "$repo_root/dev/$scaffold" "$dest/dev/$target.cpp"
done

cat > "$dest/dev/frontend_source_sets.mk" <<'EOF'
# Per-tool implementation source lists for the compiler.
#
# Add dev/src/foo.cpp to the tools that use it by adding `foo` below. For
# subdirectories, use the path without `.cpp`, such as `parser/foo`.

FRONTEND_SOURCE_SET_TARGETS := abimangle pptoken posttoken ppexpr preproc cppgm++ lowiropt lowir lowir2native
FRONTEND_TEST_RUNNER_SOURCE_ID := support/testing/test_runner

FRONTEND_OBJ_BASENAMES_abimangle :=
FRONTEND_OBJ_BASENAMES_pptoken :=
FRONTEND_OBJ_BASENAMES_posttoken :=
FRONTEND_OBJ_BASENAMES_ppexpr :=
FRONTEND_OBJ_BASENAMES_preproc :=
FRONTEND_OBJ_BASENAMES_cppgm++ :=
FRONTEND_OBJ_BASENAMES_lowiropt :=
FRONTEND_OBJ_BASENAMES_lowir :=
FRONTEND_OBJ_BASENAMES_lowir2native :=
EOF

cat > "$dest/dev/Makefile" <<'EOF'
DEFAULT_BUILD_JOBS = $(shell getconf _NPROCESSORS_ONLN 2>/dev/null || nproc 2>/dev/null || echo 1)
ifeq ($(findstring -j,$(MAKEFLAGS)),)
MAKEFLAGS += -j$(DEFAULT_BUILD_JOBS)
endif
ifeq ($(MAKELEVEL),0)
CPPGM_MAKE_JOB_FLAGS := $(filter -j% --jobs=%,$(MAKEFLAGS))
CPPGM_MAKE_JOB_LIMIT := $(patsubst --jobs=%,%,$(patsubst -j%,%,$(lastword $(CPPGM_MAKE_JOB_FLAGS))))
ifneq ($(CPPGM_MAKE_JOB_LIMIT),)
CPPGM_MAKE_LOW_JOB_LIMIT = $(shell \
	limit='$(CPPGM_MAKE_JOB_LIMIT)'; cpus='$(DEFAULT_BUILD_JOBS)'; \
	if [ "$$limit" -gt 0 ] 2>/dev/null && [ "$$cpus" -gt "$$limit" ] 2>/dev/null; then echo 1; else echo 0; fi)
ifeq ($(CPPGM_MAKE_LOW_JOB_LIMIT),1)
$(warning make is limited to -j$(CPPGM_MAKE_JOB_LIMIT) on a $(DEFAULT_BUILD_JOBS)-core machine; large compiler builds, especially self-host and PA34/inception builds, will be very slow. Omit -j or use -j$(DEFAULT_BUILD_JOBS).)
endif
endif
endif

TARGETS = \
	abimangle \
	pptoken \
	posttoken \
	ppexpr \
	preproc \
	lowir \
	lowiropt \
	lowir2native \
	cppgm++

CXX ?= g++
V ?= 0
Q = $(if $(filter 1,$(V)),,@)
quiet = @if [ "$(V)" != "1" ]; then printf '  %-7s %s\n' '$(1)' '$(2)'; fi
CPPGM_TEST_RUNNER ?= 1
CPPGM_STDLIB_FLAGS ?=
CC_FLAGS ?= -std=gnu++11 -Wall -O3 $(CPPGM_STDLIB_FLAGS)
TEST_RUNNER_SHARED_FLAGS = $(if $(filter 1,$(CPPGM_TEST_RUNNER)),-DTEST_RUNNER_ENABLE,)
TEST_RUNNER_ENTRY_FLAGS = $(if $(filter 1,$(CPPGM_TEST_RUNNER)),-Dmain=test_runner_real_main,)
ENTRY_CC_FLAGS = $(CC_FLAGS) $(TEST_RUNNER_ENTRY_FLAGS)
SRC = src
OBJ ?= ../obj
OBJDIR = $(OBJ)/dev
DEPDIR = $(OBJDIR)/.d
INC = -I$(SRC)

include frontend_source_sets.mk
$(foreach target,$(TARGETS),$(if $(filter undefined,$(origin FRONTEND_OBJ_BASENAMES_$(target))),$(error missing FRONTEND_OBJ_BASENAMES_$(target) in frontend_source_sets.mk),))

frontend_obj_basenames = $(FRONTEND_OBJ_BASENAMES_$(1))
frontend_objs = $(addprefix $(OBJDIR)/,$(addsuffix .o,$(call frontend_obj_basenames,$(1))))
entry_obj = $(OBJDIR)/entry/$(1).o
runner_obj = $(if $(filter 1,$(CPPGM_TEST_RUNNER)),$(OBJDIR)/test_runner_enabled.o)
link_objs = $(call entry_obj,$(1)) $(call frontend_objs,$(1)) $(call runner_obj)
all_obj_basenames = $(sort $(foreach target,$(TARGETS),$(call frontend_obj_basenames,$(target))))
COMPILE_CONFIG_STAMP = $(OBJDIR)/.compile_config
RUNNER_STATE_STAMP = $(OBJDIR)/.test_runner_mode

all: $(TARGETS)

define FRONTEND_RULES
$(1): $(call link_objs,$(1)) $(RUNNER_STATE_STAMP) | $(OBJDIR)
	$(call quiet,LINK,$$@)
	$(Q)$(CXX) $(CC_FLAGS) $(INC) -o $$@.tmp $(call link_objs,$(1))
	@if ! cmp -s $$@.tmp $$@ 2>/dev/null; then mv -f $$@.tmp $$@; else rm -f $$@.tmp; fi

$(call entry_obj,$(1)): $(1).cpp $(COMPILE_CONFIG_STAMP)
	@mkdir -p $$(@D) $(DEPDIR)/entry
	$(call quiet,CXX,$$@)
	$(Q)$(CXX) $(ENTRY_CC_FLAGS) $(INC) -c -MT $$@ -MMD -MP -MF $(DEPDIR)/entry/$(1).Td -o $$@ $$<
	@mv -f $(DEPDIR)/entry/$(1).Td $(DEPDIR)/entry/$(1).d
endef

$(foreach target,$(TARGETS),$(eval $(call FRONTEND_RULES,$(target))))

$(OBJDIR):
	@mkdir -p $@

FORCE:

$(COMPILE_CONFIG_STAMP): FORCE | $(OBJDIR)
	@printf '%s\n%s\n' "$(CC_FLAGS)" "$(ENTRY_CC_FLAGS)" > $@.tmp
	@if ! cmp -s $@.tmp $@ 2>/dev/null; then mv -f $@.tmp $@; else rm -f $@.tmp; fi

$(RUNNER_STATE_STAMP): FORCE | $(OBJDIR)
	@current=$$(cat $@ 2>/dev/null || true); \
	if [ "$$current" != "$(CPPGM_TEST_RUNNER)" ]; then \
		printf '%s\n' '$(CPPGM_TEST_RUNNER)' > $@; \
	fi

$(OBJDIR)/test_runner_enabled.o: $(SRC)/$(FRONTEND_TEST_RUNNER_SOURCE_ID).cpp $(COMPILE_CONFIG_STAMP)
	@mkdir -p $(@D) $(DEPDIR)
	$(call quiet,CXX,$@)
	$(Q)$(CXX) $(CC_FLAGS) $(TEST_RUNNER_SHARED_FLAGS) $(INC) -c -MT $@ -MMD -MP -MF $(DEPDIR)/test_runner_enabled.Td -o $@ $<
	@mv -f $(DEPDIR)/test_runner_enabled.Td $(DEPDIR)/test_runner_enabled.d

$(OBJDIR)/%.o: $(SRC)/%.cpp $(COMPILE_CONFIG_STAMP)
	@mkdir -p $(@D) $(dir $(DEPDIR)/$*)
	$(call quiet,CXX,$@)
	$(Q)$(CXX) $(CC_FLAGS) $(INC) -c -MT $@ -MMD -MP -MF $(DEPDIR)/$*.Td -o $@ $<
	@mv -f $(DEPDIR)/$*.Td $(DEPDIR)/$*.d

clean:
	-rm -f $(TARGETS)
	-rm -rf $(OBJDIR)

$(DEPDIR)/%.d: ;
.PRECIOUS: $(DEPDIR)/%.d

-include $(addprefix $(DEPDIR)/,$(addsuffix .d,$(all_obj_basenames)))
-include $(addprefix $(DEPDIR)/entry/,$(addsuffix .d,$(TARGETS)))
-include $(DEPDIR)/test_runner_enabled.d

.PHONY: all clean FORCE
EOF

student_makefiles=(
  "$dest/Makefile"
  "$dest"/pa*/Makefile
)
existing_student_makefiles=()
for makefile in "${student_makefiles[@]}"; do
  if [ -f "$makefile" ]; then
    existing_student_makefiles+=("$makefile")
  fi
done
sanitize_student_makefile_defaults "${existing_student_makefiles[@]}"
sanitize_student_root_makefile "$dest/Makefile"
sanitize_student_seams_makefile "$dest/pa11/Makefile"
sanitize_linux_student_scripts

if [ -f "$dest/pa34/Makefile" ]; then
  perl -0pi -e '
    s/\$\(foreach checkpoint,\$\(CHECKPOINTS\),\$\(if \$\(strip \$\(FRONTEND_OBJ_BASENAMES_\$\(checkpoint\)\)\),,\$\(error missing FRONTEND_OBJ_BASENAMES_\$\(checkpoint\) in \.\.\/dev\/frontend_source_sets\.mk\)\)\)/\$(foreach checkpoint,\$(CHECKPOINTS),\$(if \$(filter undefined,\$(origin FRONTEND_OBJ_BASENAMES_\$(checkpoint))),\$(error missing FRONTEND_OBJ_BASENAMES_\$(checkpoint) in ..\/dev\/frontend_source_sets.mk),))/g;
  ' "$dest/pa34/Makefile"
fi


reference_targets=(
  abimangle
  pptoken
  posttoken
  ppexpr
  preproc
  cppgm++
  lowir
  lowir2native
  lowiropt
)

echo "==> Building reference binaries"
rm -rf "$repo_root/obj/export-reference-build"
make -s -C "$repo_root/dev" all \
  CXX="${CXX:-g++}" \
  CPPGM_HOST_CXX="${CPPGM_HOST_CXX:-${CXX:-g++}}" \
  CPPGM_STDLIB_FLAGS="${CPPGM_STDLIB_FLAGS:-}" \
  OBJ="$repo_root/obj/export-reference-build" \
  OBJECT_ROOT_DEFAULT_DEF=

mkdir -p "$dest/reference-binaries"
for target in "${reference_targets[@]}"; do
  if [ ! -x "$repo_root/dev/$target" ]; then
    echo "export_student_repo: missing built reference binary: dev/$target" >&2
    exit 1
  fi
  install -m 0755 "$repo_root/dev/$target" "$dest/reference-binaries/$target"
  ln -s "../reference-binaries/$target" "$dest/dev/$target-ref"
done

pa_ref_pairs=(
  pa1:pptoken
  pa2:posttoken
  pa3:ppexpr
  pa4:preproc
  pa5:cppgm++
  pa6:cppgm++
  pa7:cppgm++
  pa8:lowir
  pa9:abimangle
  pa10:cppgm++
  pa11:cppgm++
  pa12:cppgm++
  pa13:cppgm++
  pa14:cppgm++
  pa15:cppgm++
  pa16:cppgm++
  pa17:cppgm++
  pa18:cppgm++
  pa19:cppgm++
  pa20:cppgm++
  pa21:cppgm++
  pa22:cppgm++
  pa23:cppgm++
  pa24:lowir2native
  pa25:cppgm++
  pa26:cppgm++
  pa27:cppgm++
  pa28:cppgm++
  pa29:cppgm++
  pa30:cppgm++
  pa31:cppgm++
  pa32:lowiropt
  pa33:lowir2native
)

source_sha=$(git -C "$repo_root" rev-parse HEAD 2>/dev/null || echo unknown)
source_short_sha=$(git -C "$repo_root" rev-parse --short=12 HEAD 2>/dev/null || echo unknown)
reference_bundle_name=${CPPGM_REFERENCE_BUNDLE_NAME:-cppgm-reference-binaries-linux-x86_64-${source_short_sha}.tar.gz}
reference_bundle_url=${CPPGM_REFERENCE_BUNDLE_URL:-https://github.com/vishvananda/cppgm-extended/releases/download/reference-binaries/${reference_bundle_name}}
reference_bundle_out=${CPPGM_REFERENCE_BUNDLE_OUT:-$repo_root/obj/export-reference-bundles/${reference_bundle_name}}

write_reference_manifest() {
  local bundle_sha target target_sha
  bundle_sha=$(sha256sum "$reference_bundle_out" | awk '{print $1}')
  {
    printf '# cppgm reference binary manifest\n'
    printf 'version\t1\n'
    printf 'source_sha\t%s\n' "$source_sha"
    printf 'platform\tlinux-x86_64\n'
    printf 'bundle_name\t%s\n' "$reference_bundle_name"
    printf 'bundle_url\t%s\n' "$reference_bundle_url"
    printf 'bundle_sha256\t%s\n' "$bundle_sha"
    for target in "${reference_targets[@]}"; do
      target_sha=$(sha256sum "$dest/reference-binaries/$target" | awk '{print $1}')
      printf 'binary\t%s\t%s\n' "$target" "$target_sha"
    done
  } > "$dest/reference-binaries/manifest.tsv"
}

finalize_reference_binaries() {
  local pa target pair

  echo "==> Packaging reference binaries"
  mkdir -p "$(dirname "$reference_bundle_out")"
  (
    cd "$dest/reference-binaries"
    tar -czf "$reference_bundle_out" "${reference_targets[@]}"
  )
  write_reference_manifest
  cat > "$dest/reference-binaries/.gitignore" <<'EOF'
*
!.gitignore
!manifest.tsv
EOF

  for target in "${reference_targets[@]}"; do
    rm -f "$dest/reference-binaries/$target"
    rm -f "$dest/dev/$target-ref"
    ln -s "../scripts/run_reference_binary.sh" "$dest/dev/$target-ref"
  done

  for pair in "${pa_ref_pairs[@]}"; do
    pa=${pair%%:*}
    target=${pair#*:}
    rm -f "$dest/$pa/$target-ref"
    ln -s "../scripts/run_reference_binary.sh" "$dest/$pa/$target-ref"
  done

  echo "==> Wrote reference binary bundle to $reference_bundle_out"
}

for pair in "${pa_ref_pairs[@]}"; do
  pa=${pair%%:*}
  target=${pair#*:}
  ln -s "../reference-binaries/$target" "$dest/$pa/$target-ref"
done

echo "==> Regenerating reference outputs"
CPPGM_KEEP_FAILED_REFERENCE_STDOUT=1 make -s -C "$dest" ref-test \
  CXX="${CXX:-g++}" \
  CPPGM_HOST_CXX="${CPPGM_HOST_CXX:-${CXX:-g++}}" \
  CPPGM_STDLIB_FLAGS="${CPPGM_STDLIB_FLAGS:-}" \
  CPPGM_TEST_RUNNER=1
find "$dest" -type f \( -name '*.my' -o -name '*.my.*' -o -name '.test_counts' -o -name '.test_failed' \) -delete
make -s -C "$dest" ref-test-debuginfo \
  CXX="${CXX:-g++}" \
  CPPGM_HOST_CXX="${CPPGM_HOST_CXX:-${CXX:-g++}}" \
  CPPGM_STDLIB_FLAGS="${CPPGM_STDLIB_FLAGS:-}" \
  CPPGM_TEST_RUNNER=1

verify_regenerated_reference_outputs

prune_student_pa33
prune_student_pa35

finalize_reference_binaries

(
  cd "$dest"
  git init -q
  git add -A
  git \
    -c user.name="cppgm export" \
    -c user.email="cppgm-export@example.invalid" \
    commit -q -m "Initial student export"
)

# Every script an exported Makefile names must have shipped: a lane that
# `make test` runs from a script the export forgot fails for every student.
check_exported_script_references() {
  local missing=0 makefile dir ref path
  # A target the export strips must leave no caller and no name behind.
  local target
  for target in $STUDENT_ROOT_MAKEFILE_DROPPED_TARGETS; do
    if grep -q -e "$target" "$dest/Makefile" 2>/dev/null; then
      echo "export: the root Makefile still names the maintainer target $target" >&2
      exit 1
    fi
  done
  for makefile in "$dest/Makefile" "$dest"/pa*/Makefile; do
    [ -f "$makefile" ] || continue
    dir=$(dirname "$makefile")
    while IFS= read -r ref; do
      case "$ref" in
        ../*) path="$dir/$ref" ;;
        *) path="$dir/$ref" ;;
      esac
      if [ ! -e "$path" ]; then
        echo "export: $makefile names $ref, which did not ship" >&2
        missing=1
      fi
    done < <(sed 's/#.*//' "$makefile" | LC_ALL=C grep -o '[A-Za-z0-9_./-]*scripts/[A-Za-z0-9_./-]*\.\(pl\|py\|sh\|pm\|mk\)' | LC_ALL=C sort -u)
  done
  # The shipped scripts load one another by bare, quoted file name (the
  # expectation evaluator); those must have shipped as well.
  local script name
  for script in "$dest"/scripts/*.pl "$dest"/scripts/*.pm "$dest"/scripts/*.py "$dest"/scripts/*.sh; do
    [ -f "$script" ] || continue
    while IFS= read -r name; do
      [ "$name" = "$(basename "$script")" ] && continue
      if [ ! -e "$dest/scripts/$name" ]; then
        echo "export: $script names $name, which did not ship" >&2
        missing=1
      fi
    done < <(sed 's/#.*//' "$script" | LC_ALL=C grep -o '"[A-Za-z0-9_]*\.\(pl\|pm\|py\|sh\)"' | tr -d '"' | LC_ALL=C sort -u)
  done
  if [ "$missing" -ne 0 ]; then
    echo "export: exported files reference scripts that did not ship" >&2
    exit 1
  fi
}
check_exported_script_references

# The shipped headers are the vocabulary of their formats.  The course
# solution compiles them but never exercises every reader, so this check
# keeps them whole: with only the exported headers on the include path they
# compile together, and every spelling reads back as its own value.
check_exported_scaffold_vocabulary() {
  local scratch
  scratch=$(mktemp -d)
  if ! "${CXX:-g++}" -std=gnu++11 -Wall -Wextra -Werror ${CPPGM_STDLIB_FLAGS:-} -I "$dest/dev/src" \
       -o "$scratch/scaffold_vocabulary_round_trip" "$repo_root/scripts/scaffold_vocabulary_round_trip.cpp" ||
     ! "$scratch/scaffold_vocabulary_round_trip"; then
    echo "export: the shipped vocabulary headers do not build on their own or do not round-trip" >&2
    rm -rf "$scratch"
    exit 1
  fi
  rm -rf "$scratch"
}
check_exported_scaffold_vocabulary

# The tool scaffolds are the only exported code that includes the
# scaffold-only not_implemented.h, and the only users of the token stream and
# help texts in the export: each must compile against the exported headers.
check_exported_tool_scaffolds() {
  local pair target failed=0
  for pair in "${scaffold_pairs[@]}"; do
    target=${pair%%:*}
    if ! "${CXX:-g++}" -std=gnu++11 -Wall ${CPPGM_STDLIB_FLAGS:-} -I "$dest/dev/src" \
         -fsyntax-only "$dest/dev/$target.cpp"; then
      echo "export: the exported dev/$target.cpp scaffold does not compile" >&2
      failed=1
    fi
  done
  if [ "$failed" -ne 0 ]; then
    exit 1
  fi
}
check_exported_tool_scaffolds

# The file audit ships, and student runs call it on dev/src from the first
# assignment.  The export has no export script or scaffold manifest, so the
# audit treats the shipped headers and tool scaffolds as the student's code:
# they must pass it as shipped, warnings included.
check_exported_file_audit() {
  if ! (cd "$dest" && perl scripts/cppgm_file_audit.pl --warnings-fail --paths dev); then
    echo "export: the exported dev/ does not pass the shipped file audit" >&2
    exit 1
  fi
}
check_exported_file_audit

echo "==> Exported student repo to $dest"
