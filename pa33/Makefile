TARGET = lowir2native
LOWIR2NATIVE_APP ?= ../dev/$(TARGET)
CPPGM_DRIVER_APP ?= ../dev/cppgm++
LOWIR2NATIVE_REF_APP ?= $(LOWIR2NATIVE_APP)-ref
TEST_DEPS ?= dev-shared-target
HOST_UNAME_S = $(shell uname -s)
LLVM_CXX_LOCAL = /usr/local/opt/llvm/bin/clang++
LLVM_CXX_HOMEBREW = /opt/homebrew/opt/llvm/bin/clang++
ifeq ($(HOST_UNAME_S),Darwin)
ifeq ($(wildcard $(LLVM_CXX_LOCAL)),)
ifneq ($(wildcard $(LLVM_CXX_HOMEBREW)),)
LLVM_CXX_DEFAULT = $(LLVM_CXX_HOMEBREW)
endif
else
LLVM_CXX_DEFAULT = $(LLVM_CXX_LOCAL)
endif
ifdef LLVM_CXX_DEFAULT
HOST_CXX_DEFAULT = $(LLVM_CXX_DEFAULT)
else
HOST_CXX_DEFAULT = clang++
endif
else
HOST_CXX_DEFAULT = g++
endif

ifeq ($(origin CXX), default)
CXX := $(HOST_CXX_DEFAULT)
endif
ifeq ($(origin CPPGM_HOST_CXX), undefined)
CPPGM_HOST_CXX := $(CXX)
endif
export CPPGM_HOST_CXX
CPPGM_STDLIB_FLAGS ?=
REPORT_EMPTY_OPTIONAL_TEST_ROOTS ?= 0
O1_TEST_ROOT ?= tests/o1
O2_TEST_ROOT ?= tests/o2
O3_TEST_ROOT ?= tests/o3
DEBUGINFO_TEST_ROOT ?= tests/debuginfo
DEBUGINFO_O1_TEST_ROOT ?= $(DEBUGINFO_TEST_ROOT)/o1
DEBUGINFO_O2_TEST_ROOT ?= $(DEBUGINFO_TEST_ROOT)/o2
DEBUGINFO_O3_TEST_ROOT ?= $(DEBUGINFO_TEST_ROOT)/o3
BEHAVIOR_TEST_ROOT ?= tests/behavior
BEHAVIOR_O1_TEST_ROOT ?= $(BEHAVIOR_TEST_ROOT)/o1
BEHAVIOR_O2_TEST_ROOT ?= $(BEHAVIOR_TEST_ROOT)/o2
BEHAVIOR_O3_TEST_ROOT ?= $(BEHAVIOR_TEST_ROOT)/o3
DRIVER_TEST_ROOT ?= tests/driver

all: dev-shared-target

define run_native_bucket
	@if [ -d "$(1)" ] && find "$(1)" -type f -name '*.t' | grep -q .; then \
		CPPGM_APP_ARGS='$(2)' perl ../scripts/run_lowir_native_tests_worker.pl $(LOWIR2NATIVE_APP) my $(1); \
		scripts/compare_results.pl ref my $(1); \
	else \
		if [ "$(REPORT_EMPTY_OPTIONAL_TEST_ROOTS)" = "1" ]; then echo "$(TARGET) $(1): no tests"; fi; \
	fi
endef

# Behavior reference regeneration records program outcomes without MIR dumps.
# Existing expectation sidecars still apply to the student's MIR.
define run_native_behavior_bucket_ref
	@if [ -d "$(1)" ] && find "$(1)" -type f -name '*.t' | grep -q .; then \
		CPPGM_NATIVE_BEHAVIOR_ONLY=1 CPPGM_APP_ARGS='$(2)' perl ../scripts/run_lowir_native_tests_worker.pl $(LOWIR2NATIVE_REF_APP) ref $(1); \
	else \
		if [ "$(REPORT_EMPTY_OPTIONAL_TEST_ROOTS)" = "1" ]; then echo "$(TARGET) $(1): no tests"; fi; \
	fi
endef

define run_native_bucket_ref
	@if [ -d "$(1)" ] && find "$(1)" -type f -name '*.t' | grep -q .; then \
		CPPGM_APP_ARGS='$(2)' perl ../scripts/run_lowir_native_tests_worker.pl $(LOWIR2NATIVE_REF_APP) ref $(1) && \
		perl ../scripts/canonicalize_lowir_native_refs.pl $(1); \
	else \
		if [ "$(REPORT_EMPTY_OPTIONAL_TEST_ROOTS)" = "1" ]; then echo "$(TARGET) $(1): no tests"; fi; \
	fi
endef

test: test-course

test-course: $(TEST_DEPS)
	$(call run_native_bucket,$(O1_TEST_ROOT),-O1)
	$(call run_native_bucket,$(O2_TEST_ROOT),-O2)
	$(call run_native_bucket,$(O3_TEST_ROOT),-O3)
	$(call run_native_bucket,$(BEHAVIOR_O1_TEST_ROOT),-O1)
	$(call run_native_bucket,$(BEHAVIOR_O2_TEST_ROOT),-O2)
	$(call run_native_bucket,$(BEHAVIOR_O3_TEST_ROOT),-O3)
	@perl ../scripts/check_cppgm_native_programs.pl $(CPPGM_DRIVER_APP) "$(DRIVER_TEST_ROOT)" $(LOWIR2NATIVE_APP)

test-debuginfo: $(TEST_DEPS)
	$(call run_native_bucket,$(DEBUGINFO_O1_TEST_ROOT),-O1)
	$(call run_native_bucket,$(DEBUGINFO_O2_TEST_ROOT),-O2)
	$(call run_native_bucket,$(DEBUGINFO_O3_TEST_ROOT),-O3)

ref-test:
ifneq ($(strip $(TEST)),)
	@$(REF_TEST_RUN)
else
	$(call run_native_bucket_ref,$(O1_TEST_ROOT),-O1)
	$(call run_native_bucket_ref,$(O2_TEST_ROOT),-O2)
	$(call run_native_bucket_ref,$(O3_TEST_ROOT),-O3)
	$(call run_native_behavior_bucket_ref,$(BEHAVIOR_O1_TEST_ROOT),-O1)
	$(call run_native_behavior_bucket_ref,$(BEHAVIOR_O2_TEST_ROOT),-O2)
	$(call run_native_behavior_bucket_ref,$(BEHAVIOR_O3_TEST_ROOT),-O3)
endif

ref-test-debuginfo:
	$(call run_native_bucket_ref,$(DEBUGINFO_O1_TEST_ROOT),-O1)
	$(call run_native_bucket_ref,$(DEBUGINFO_O2_TEST_ROOT),-O2)
	$(call run_native_bucket_ref,$(DEBUGINFO_O3_TEST_ROOT),-O3)

REF_TEST_RUN = case "$(TEST)" in \
	$(DRIVER_TEST_ROOT)/*) echo "$(TEST): self-checking program has no reference sidecar" ;; \
	$(BEHAVIOR_O1_TEST_ROOT)/*) CPPGM_NATIVE_BEHAVIOR_ONLY=1 CPPGM_APP_ARGS='-O1' perl ../scripts/run_lowir_native_tests_worker.pl $(LOWIR2NATIVE_REF_APP) ref "$(TEST)" ;; \
	$(BEHAVIOR_O2_TEST_ROOT)/*) CPPGM_NATIVE_BEHAVIOR_ONLY=1 CPPGM_APP_ARGS='-O2' perl ../scripts/run_lowir_native_tests_worker.pl $(LOWIR2NATIVE_REF_APP) ref "$(TEST)" ;; \
	$(BEHAVIOR_O3_TEST_ROOT)/*) CPPGM_NATIVE_BEHAVIOR_ONLY=1 CPPGM_APP_ARGS='-O3' perl ../scripts/run_lowir_native_tests_worker.pl $(LOWIR2NATIVE_REF_APP) ref "$(TEST)" ;; \
	$(O1_TEST_ROOT)/*|$(DEBUGINFO_O1_TEST_ROOT)/*|$(BEHAVIOR_O1_TEST_ROOT)/*) CPPGM_APP_ARGS='-O1' perl ../scripts/run_lowir_native_tests_worker.pl $(LOWIR2NATIVE_REF_APP) ref "$(TEST)" && perl ../scripts/canonicalize_lowir_native_refs.pl "$(TEST)" ;; \
	$(O2_TEST_ROOT)/*|$(DEBUGINFO_O2_TEST_ROOT)/*|$(BEHAVIOR_O2_TEST_ROOT)/*) CPPGM_APP_ARGS='-O2' perl ../scripts/run_lowir_native_tests_worker.pl $(LOWIR2NATIVE_REF_APP) ref "$(TEST)" && perl ../scripts/canonicalize_lowir_native_refs.pl "$(TEST)" ;; \
	$(O3_TEST_ROOT)/*|$(DEBUGINFO_O3_TEST_ROOT)/*|$(BEHAVIOR_O3_TEST_ROOT)/*) CPPGM_APP_ARGS='-O3' perl ../scripts/run_lowir_native_tests_worker.pl $(LOWIR2NATIVE_REF_APP) ref "$(TEST)" && perl ../scripts/canonicalize_lowir_native_refs.pl "$(TEST)" ;; \
	*) echo "Usage: make ref-test TEST=$(O1_TEST_ROOT)/<file>.t | $(O2_TEST_ROOT)/<file>.t | $(O3_TEST_ROOT)/<file>.t"; exit 1 ;; \
esac


CHECK_RUN = perl ../scripts/run_routed_test_spec.pl "$(TEST)" \
	'$(DRIVER_TEST_ROOT)/*::perl ../scripts/check_cppgm_native_programs.pl $(CPPGM_DRIVER_APP) {tests} $(LOWIR2NATIVE_APP)' \
	'$(O1_TEST_ROOT)/*|$(DEBUGINFO_O1_TEST_ROOT)/*|$(BEHAVIOR_O1_TEST_ROOT)/*::CPPGM_APP_ARGS=-O1 scripts/run_all_tests.pl $(LOWIR2NATIVE_APP) check {tests}' \
	'$(O2_TEST_ROOT)/*|$(DEBUGINFO_O2_TEST_ROOT)/*|$(BEHAVIOR_O2_TEST_ROOT)/*::CPPGM_APP_ARGS=-O2 scripts/run_all_tests.pl $(LOWIR2NATIVE_APP) check {tests}' \
	'$(O3_TEST_ROOT)/*|$(DEBUGINFO_O3_TEST_ROOT)/*|$(BEHAVIOR_O3_TEST_ROOT)/*::CPPGM_APP_ARGS=-O3 scripts/run_all_tests.pl $(LOWIR2NATIVE_APP) check {tests}'
CHECK_COMPARE = perl ../scripts/run_routed_test_spec.pl "$(TEST)" \
	'$(DRIVER_TEST_ROOT)/*::true' \
	'$(O1_TEST_ROOT)/*|$(DEBUGINFO_O1_TEST_ROOT)/*|$(BEHAVIOR_O1_TEST_ROOT)/*|$(O2_TEST_ROOT)/*|$(DEBUGINFO_O2_TEST_ROOT)/*|$(BEHAVIOR_O2_TEST_ROOT)/*|$(O3_TEST_ROOT)/*|$(DEBUGINFO_O3_TEST_ROOT)/*|$(BEHAVIOR_O3_TEST_ROOT)/*::scripts/compare_results.pl ref check {tests}'
include ../scripts/pa_run_check_targets.mk

.PHONY: dev-shared-target ref-test ref-test-debuginfo test-debuginfo
../dev/$(TARGET):
	$(MAKE) -C ../dev $(TARGET) CXX=$(CXX) CPPGM_HOST_CXX=$(CPPGM_HOST_CXX)

dev-shared-target:
	@if [ "$(CPPGM_SKIP_DEV_REBUILD)" = "1" ]; then true; else $(MAKE) -C ../dev $(TARGET) lowiropt cppgm++ CXX=$(CXX) CPPGM_HOST_CXX=$(CPPGM_HOST_CXX); fi

clean:
	-rm -f $(TARGET) $(TARGET)-debug

-include maintainer/Makefile
