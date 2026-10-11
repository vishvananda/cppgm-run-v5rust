TARGET = lowir2native
CXX = g++
CPPGM_STDLIB_FLAGS ?=
CPPGM_TEST_APP ?= ../dev/$(TARGET)
REF_TEST_APP ?= $(CPPGM_TEST_APP)-ref
TEST_DEPS ?= dev-shared-target
CC_FLAGS = -std=gnu++11 -Wall -O3 $(CPPGM_STDLIB_FLAGS)
SRC = ../dev/src
GENERATED = ../obj/generated
INC = -I$(SRC) -I$(GENERATED)
OBJ = ../obj
VPATH = $(SRC)
RDEPDIR = $(OBJ)/release/.d
DDEPDIR = $(OBJ)/debug/.d
CONTROL_TEST_ROOT ?= tests/controls
# Our own shape references live in the regression lane, outside the course
# contract (tests/regression; see TESTING_AND_REFERENCES.md).
REGRESSION_ORACLE_TEST_ROOTS = $(wildcard tests/regression/strict tests/regression/structural tests/regression/behavior)
PERF_TOLERANCE_PERCENT ?= 10

$(shell mkdir -p $(RDEPDIR) $(DDEPDIR) >/dev/null)

include ../dev/frontend_source_sets.mk

common_obj_basenames = $(FRONTEND_OBJ_BASENAMES_$(TARGET))
common_objs = $(addprefix $(OBJ)/$(1)/,$(addsuffix .o,$(common_obj_basenames)))

all: dev-shared-target

$(OBJ)/release $(OBJ)/debug: %:
	@- mkdir -p $@

$(OBJ)/release/%.o: %.cpp
	@mkdir -p $(@D) $(dir $(RDEPDIR)/$*)
	$(CXX) $(CC_FLAGS) $(INC) -c -MT $@ -MMD -MP -MF $(RDEPDIR)/$*.Td -o $@ $<
	mv -f $(RDEPDIR)/$*.Td $(RDEPDIR)/$*.d

$(OBJ)/debug/%.o: %.cpp
	@mkdir -p $(@D) $(dir $(DDEPDIR)/$*)
	$(CXX) $(CC_FLAGS) $(INC) -c -MT $@ -MMD -MP -MF $(DDEPDIR)/$*.Td -o $@ $<
	mv -f $(DDEPDIR)/$*.Td $(DDEPDIR)/$*.d

# The course suite (test-course) judges by behaviour, contract shape where
# the shape is the contract, and envelope; the regression suite
# (test-regression) pins the course solution's dumps.
test: test-course test-regression

test-course: $(TEST_DEPS)
ifneq ($(CPPGM_BATCH_TESTS),)
	@perl ../scripts/run_lowir_native_tests_worker.pl $(CPPGM_TEST_APP) my tests
else
	@scripts/run_all_tests.pl $(CPPGM_TEST_APP) my tests
endif
	@scripts/compare_results.pl ref my tests
	@set -e; if [ -d "$(CONTROL_TEST_ROOT)" ]; then \
		perl ../scripts/check_pa24_native_contracts.pl \
			$(CPPGM_TEST_APP) "$(CONTROL_TEST_ROOT)"; \
	fi

# The course solution's selection policies (its copy and zero thresholds,
# frame-pointer and epilogue choice, switch compare chain, carried reloads and
# parameter carriers) are checked here, outside the course contract.
REGRESSION_CONTROL_TEST_ROOT = tests/regression/controls
REGRESSION_CONTROL_BEHAVIOR_TESTS = \
	tests/behavior/900-scratch-carried-frame-reloads.t \
	tests/behavior/900-deferred-address-parameter-carrier-reuse.t \
	tests/behavior/800-cost-directed-small-zeroinit.t

test-regression: $(TEST_DEPS)
	@set -e; if [ -d "$(REGRESSION_CONTROL_TEST_ROOT)" ]; then \
		perl ../scripts/check_pa24_native_contracts.pl \
			$(CPPGM_TEST_APP) "$(REGRESSION_CONTROL_TEST_ROOT)"; \
	fi; \
	for test in $(REGRESSION_CONTROL_BEHAVIOR_TESTS); do \
		perl ../scripts/check_pa24_native_contracts.pl $(CPPGM_TEST_APP) "$$test"; \
	done
ifneq ($(CPPGM_BATCH_TESTS),)
	@set -e; for root in $(REGRESSION_ORACLE_TEST_ROOTS); do \
		perl ../scripts/run_lowir_native_tests_worker.pl \
			$(CPPGM_TEST_APP) my "$$root"; \
		case "$$root" in \
			*/behavior) mode=mir_behavior_t ;; \
			*/structural) mode=mir_structural_t ;; \
			*) mode=mir_t ;; \
		esac; \
		perl ../scripts/compare_results_common.pl \
			"$$mode" ref my "$$root"; \
	done
else
	@set -e; for root in $(REGRESSION_ORACLE_TEST_ROOTS); do \
		scripts/run_all_tests.pl $(CPPGM_TEST_APP) my "$$root"; \
		case "$$root" in \
			*/behavior) mode=mir_behavior_t ;; \
			*/structural) mode=mir_structural_t ;; \
			*) mode=mir_t ;; \
		esac; \
		perl ../scripts/compare_results_common.pl \
			"$$mode" ref my "$$root"; \
	done
endif

# Without TEST, `make ref-test` regenerates the regression lane's references
# as well; the course runner prunes `regression/` from `tests`.  Structural
# references carry a canonical `.ref.cmir` beside the dump, so every
# structural root is recanonicalized, whichever fixtures were regenerated.
STRUCTURAL_TEST_ROOTS = $(wildcard tests/structural tests/regression/structural)

ifeq ($(strip $(TEST)),)
ref-test: ref-test-regression
endif

ref-test:
	@$(REF_TEST_RUN)
	@set -e; for root in $(STRUCTURAL_TEST_ROOTS); do \
		perl ../scripts/canonicalize_lowir_native_refs.pl "$$root"; \
	done

ref-test-regression:
	@set -e; for root in $(REGRESSION_ORACLE_TEST_ROOTS); do \
		scripts/run_all_tests.pl $(REF_TEST_APP) ref "$$root"; \
	done
	@if [ -d tests/regression/structural ]; then \
		perl ../scripts/canonicalize_lowir_native_refs.pl tests/regression/structural; \
	fi

REF_TEST_RUN = case "$(TEST)" in \
	$(CONTROL_TEST_ROOT)/*|$(REGRESSION_CONTROL_TEST_ROOT)/*) echo "$(TEST): focused native predicate has no reference sidecar" ;; \
	*) scripts/run_all_tests.pl $(REF_TEST_APP) ref "$(if $(strip $(TEST)),$(TEST),tests)" ;; \
esac

CHECK_RUN = case "$(TEST)" in \
	$(CONTROL_TEST_ROOT)/*|$(REGRESSION_CONTROL_TEST_ROOT)/*) perl ../scripts/check_pa24_native_contracts.pl $(CPPGM_TEST_APP) "$(TEST)" ;; \
	*) scripts/run_all_tests.pl $(CPPGM_TEST_APP) check "$(TEST)" ;; \
esac
CHECK_COMPARE = case "$(TEST)" in \
	$(CONTROL_TEST_ROOT)/*|$(REGRESSION_CONTROL_TEST_ROOT)/*) true ;; \
	*) scripts/compare_results.pl ref check "$(TEST)" ;; \
esac

include ../scripts/pa_run_check_targets.mk

.PHONY: dev-shared-target ref-test ref-test-regression
$(CPPGM_TEST_APP):
	$(MAKE) -C ../dev $(TARGET) CXX=$(CXX) OBJ=$(OBJ)

dev-shared-target:
	@if [ "$(CPPGM_SKIP_DEV_REBUILD)" = "1" ]; then true; else $(MAKE) -C ../dev $(TARGET) CXX=$(CXX) OBJ=$(OBJ); fi

clean:
	-rm -f $(TARGET) $(TARGET)-debug $(OBJ)/release/*.o $(OBJ)/debug/*.o

$(RDEPDIR)/%.d: ;
.PRECIOUS: $(RDEPDIR)/%.d

$(DDEPDIR)/%.d: ;
.PRECIOUS: $(DDEPDIR)/%.d

-include $(addprefix $(RDEPDIR)/,$(addsuffix .d,$(common_obj_basenames)))
-include $(addprefix $(DDEPDIR)/,$(addsuffix .d,$(common_obj_basenames)))

# The performance envelope: each behaviour program runs under Cachegrind and
# its instruction count must stay within PERF_TOLERANCE_PERCENT of the
# `x.ref.ir` sidecar.  `make test-perf-refs` writes the sidecars from the
# current programs.  Skipped when valgrind is not installed.
test-perf: $(TEST_DEPS)
	@perl ../scripts/check_ir_envelope.pl $(PERF_TOLERANCE_PERCENT) tests/behavior

test-perf-refs: $(TEST_DEPS)
	@perl ../scripts/check_ir_envelope.pl --write $(PERF_TOLERANCE_PERCENT) tests/behavior
