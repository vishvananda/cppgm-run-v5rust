# Solution regression and profiling lanes; omitted from the student export.
LOWIROPT_APP ?= ../dev/lowiropt
REGRESSION_O1_TEST_ROOT ?= tests/regression/o1
REGRESSION_O2_TEST_ROOT ?= tests/regression/o2
REGRESSION_O3_TEST_ROOT ?= tests/regression/o3
REGRESSION_BEHAVIOR_O1_TEST_ROOT ?= tests/regression/behavior/o1
REGRESSION_BEHAVIOR_O2_TEST_ROOT ?= tests/regression/behavior/o2
REGRESSION_BEHAVIOR_O3_TEST_ROOT ?= tests/regression/behavior/o3
PERF_TOLERANCE_PERCENT ?= 10
DRIVER_CENSUS_TEST_ROOT ?= tests/regression/driver
STRUCTURAL_CONTROL_TEST_ROOT ?= tests/regression/controls
# The course solution's selection policies at the native level (the strlen
# prefix probe, the dynamic copy of an unused-result memcpy), outside the
# course contract.
NATIVE_CONTROL_TEST_ROOT ?= tests/regression/controls/native

define run_driver_census_bucket
	@if [ -d "$(1)" ] && find "$(1)" \( -type f -o -type l \) -name '*.t' | grep -q .; then \
		perl ../scripts/check_cppgm_function_census.pl $(CPPGM_DRIVER_APP) "$(1)"; \
	else \
		if [ "$(REPORT_EMPTY_OPTIONAL_TEST_ROOTS)" = "1" ]; then echo "$(TARGET) $(1): no tests"; fi; \
	fi
endef

define run_structural_control_bucket
	@if [ -d "$(1)" ] && find "$(1)" -type f -name '*.t' | grep -q .; then \
		status=0; \
		perl ../scripts/check_lowir_native_structural_controls.pl \
			$(LOWIR2NATIVE_APP) $(CPPGM_DRIVER_APP) "$(1)" || status=1; \
		perl ../scripts/check_lowir_native_parameter_addresses.pl \
			$(LOWIROPT_APP) $(LOWIR2NATIVE_APP) $(CPPGM_DRIVER_APP) "$(1)" || status=1; \
		perl ../scripts/check_lowir_native_tail_jump.pl \
			$(LOWIR2NATIVE_APP) $(CPPGM_DRIVER_APP) "$(1)" || status=1; \
		perl ../scripts/check_lowir_native_scalar_query.pl \
			$(LOWIR2NATIVE_APP) $(CPPGM_DRIVER_APP) "$(1)" || status=1; \
		test $$status -eq 0; \
	else \
		if [ "$(REPORT_EMPTY_OPTIONAL_TEST_ROOTS)" = "1" ]; then echo "$(TARGET) $(1): no tests"; fi; \
	fi
endef

define run_survivor_property_bucket
	@if [ -d "$(1)" ] && find "$(1)" -type f -name '*.t' | grep -q .; then \
		perl ../scripts/check_lowir_native_survivor_properties.pl \
			$(LOWIR2NATIVE_APP) "$(1)"; \
	else \
		if [ "$(REPORT_EMPTY_OPTIONAL_TEST_ROOTS)" = "1" ]; then echo "$(TARGET) $(1): no tests"; fi; \
	fi
endef

test: test-regression

test-regression: $(TEST_DEPS)
	$(call run_driver_census_bucket,$(DRIVER_CENSUS_TEST_ROOT))
	$(call run_native_bucket,$(REGRESSION_O1_TEST_ROOT),-O1)
	$(call run_native_bucket,$(REGRESSION_O2_TEST_ROOT),-O2)
	$(call run_native_bucket,$(REGRESSION_O3_TEST_ROOT),-O3)
	$(call run_native_bucket,$(REGRESSION_BEHAVIOR_O1_TEST_ROOT),-O1)
	$(call run_native_bucket,$(REGRESSION_BEHAVIOR_O2_TEST_ROOT),-O2)
	$(call run_native_bucket,$(REGRESSION_BEHAVIOR_O3_TEST_ROOT),-O3)
	$(call run_structural_control_bucket,$(STRUCTURAL_CONTROL_TEST_ROOT))
	@if [ -d "$(NATIVE_CONTROL_TEST_ROOT)" ]; then \
		perl ../scripts/check_pa24_native_contracts.pl $(LOWIR2NATIVE_APP) "$(NATIVE_CONTROL_TEST_ROOT)"; \
	fi
	$(call run_survivor_property_bucket,$(REGRESSION_O1_TEST_ROOT))
	$(call run_survivor_property_bucket,$(REGRESSION_O2_TEST_ROOT))
	$(call run_survivor_property_bucket,$(REGRESSION_BEHAVIOR_O1_TEST_ROOT))

ifeq ($(strip $(TEST)),)
ref-test: ref-test-regression
endif

ref-test-regression:
	$(call run_native_bucket_ref,$(REGRESSION_O1_TEST_ROOT),-O1)
	$(call run_native_bucket_ref,$(REGRESSION_O2_TEST_ROOT),-O2)
	$(call run_native_bucket_ref,$(REGRESSION_O3_TEST_ROOT),-O3)
	$(call run_native_bucket_ref,$(REGRESSION_BEHAVIOR_O1_TEST_ROOT),-O1)
	$(call run_native_bucket_ref,$(REGRESSION_BEHAVIOR_O2_TEST_ROOT),-O2)
	$(call run_native_bucket_ref,$(REGRESSION_BEHAVIOR_O3_TEST_ROOT),-O3)

CHECK_RUN += \
	'$(NATIVE_CONTROL_TEST_ROOT)/*::perl ../scripts/check_pa24_native_contracts.pl $(LOWIR2NATIVE_APP) {tests}' \
	'$(STRUCTURAL_CONTROL_TEST_ROOT)/*host-scalar-tail-query*.t::perl ../scripts/check_lowir_native_scalar_query.pl $(LOWIR2NATIVE_APP) $(CPPGM_DRIVER_APP) {tests}' \
	'$(STRUCTURAL_CONTROL_TEST_ROOT)/*host-tail-jump-transfer*.t::perl ../scripts/check_lowir_native_tail_jump.pl $(LOWIR2NATIVE_APP) $(CPPGM_DRIVER_APP) {tests}' \
	'$(DRIVER_CENSUS_TEST_ROOT)/*::perl ../scripts/check_cppgm_function_census.pl $(CPPGM_DRIVER_APP) {tests}' \
	'$(STRUCTURAL_CONTROL_TEST_ROOT)/*parameter-address-folding*.t::perl ../scripts/check_lowir_native_parameter_addresses.pl $(LOWIROPT_APP) $(LOWIR2NATIVE_APP) $(CPPGM_DRIVER_APP) {tests}' \
	'$(STRUCTURAL_CONTROL_TEST_ROOT)/*::perl ../scripts/check_lowir_native_structural_controls.pl $(LOWIR2NATIVE_APP) $(CPPGM_DRIVER_APP) {tests}'

CHECK_COMPARE += \
	'$(NATIVE_CONTROL_TEST_ROOT)/*::true' \
	'$(DRIVER_CENSUS_TEST_ROOT)/*::true' \
	'$(STRUCTURAL_CONTROL_TEST_ROOT)/*::true'

# The performance envelope: each behaviour program, course and regression,
# runs under Cachegrind and its instruction count must stay within
# PERF_TOLERANCE_PERCENT of its tracked `x.ref.ir` sidecar; a program without
# one fails.  `make test-perf-refs` builds the same programs and writes every
# sidecar from them.  Skipped when valgrind is not installed.
PERF_TEST_ROOTS = $(BEHAVIOR_O1_TEST_ROOT) $(BEHAVIOR_O2_TEST_ROOT) $(BEHAVIOR_O3_TEST_ROOT) \
	$(REGRESSION_BEHAVIOR_O1_TEST_ROOT) $(REGRESSION_BEHAVIOR_O2_TEST_ROOT) $(REGRESSION_BEHAVIOR_O3_TEST_ROOT)

define build_perf_programs
	$(call run_native_bucket,$(BEHAVIOR_O1_TEST_ROOT),-O1)
	$(call run_native_bucket,$(BEHAVIOR_O2_TEST_ROOT),-O2)
	$(call run_native_bucket,$(BEHAVIOR_O3_TEST_ROOT),-O3)
	$(call run_native_bucket,$(REGRESSION_BEHAVIOR_O1_TEST_ROOT),-O1)
	$(call run_native_bucket,$(REGRESSION_BEHAVIOR_O2_TEST_ROOT),-O2)
	$(call run_native_bucket,$(REGRESSION_BEHAVIOR_O3_TEST_ROOT),-O3)
endef

test-perf: $(TEST_DEPS)
	$(build_perf_programs)
	@perl ../scripts/check_ir_envelope.pl --require-sidecars $(PERF_TOLERANCE_PERCENT) $(PERF_TEST_ROOTS)

test-perf-refs: $(TEST_DEPS)
	$(build_perf_programs)
	@perl ../scripts/check_ir_envelope.pl --write $(PERF_TOLERANCE_PERCENT) $(PERF_TEST_ROOTS)
