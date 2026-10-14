TARGET = cppgm++
TARGET_ARGS = --emit-lowir -O0
export CPPGM_APP_ARGS := $(TARGET_ARGS)
CXX = g++
CPPGM_STDLIB_FLAGS ?=
CPPGM_TEST_RUNNER ?= 1
CPPGM_TEST_APP ?= ../dev/$(TARGET)
REF_TEST_APP ?= $(CPPGM_TEST_APP)-ref
NATIVE_REFERENCE_APP ?= ../dev/lowir2native-ref
TEST_DEPS ?= dev-shared-target
TEST_RUNNER_SHARED_FLAGS = $(if $(filter 1,$(CPPGM_TEST_RUNNER)),-DTEST_RUNNER_ENABLE,)
TEST_RUNNER_ENTRY_FLAGS = $(TEST_RUNNER_SHARED_FLAGS) $(if $(filter 1,$(CPPGM_TEST_RUNNER)),-Dmain=test_runner_real_main,)
CC_FLAGS = -std=gnu++11 -Wall -O3 $(CPPGM_STDLIB_FLAGS)
ENTRY_CC_FLAGS = $(CC_FLAGS) $(TEST_RUNNER_ENTRY_FLAGS)
SRC = ../dev/src
GENERATED = ../obj/generated
INC = -I$(SRC) -I$(GENERATED)
OBJ = ../obj
VPATH = $(SRC)
RDEPDIR = $(OBJ)/release/.d
DDEPDIR = $(OBJ)/debug/.d
CONTROL_TEST_ROOT ?= tests/controls

$(shell mkdir -p $(RDEPDIR) $(DDEPDIR) >/dev/null)

include ../dev/frontend_source_sets.mk

common_obj_basenames = $(FRONTEND_OBJ_BASENAMES_$(TARGET))
common_objs = $(addprefix $(OBJ)/$(1)/,$(addsuffix .o,$(common_obj_basenames)))
runner_obj = $(if $(filter 1,$(CPPGM_TEST_RUNNER)),$(OBJ)/$(1)/test_runner_enabled.o)
runner_state_stamp = $(OBJ)/$(1)/.test_runner_mode
link_objs = $(call common_objs,$(1)) $(call runner_obj,$(1))
link_prereqs = $(call link_objs,$(1)) $(call runner_state_stamp,$(1))

all: dev-shared-target

$(OBJ)/release $(OBJ)/debug: %:
	@- mkdir -p $@

FORCE:

$(OBJ)/release/.test_runner_mode $(OBJ)/debug/.test_runner_mode: FORCE | $(OBJ)/release $(OBJ)/debug
	@current=$$(cat $@ 2>/dev/null || true); \
	if [ "$$current" != "$(CPPGM_TEST_RUNNER)" ]; then \
		printf '%s\n' '$(CPPGM_TEST_RUNNER)' > $@; \
	fi

$(OBJ)/release/%.o: %.cpp
	@mkdir -p $(@D) $(dir $(RDEPDIR)/$*)
	$(CXX) $(CC_FLAGS) $(INC) -c -MT $@ -MMD -MP -MF $(RDEPDIR)/$*.Td -o $@ $<
	mv -f $(RDEPDIR)/$*.Td $(RDEPDIR)/$*.d

$(OBJ)/debug/%.o: %.cpp
	@mkdir -p $(@D) $(dir $(DDEPDIR)/$*)
	$(CXX) $(CC_FLAGS) $(INC) -c -MT $@ -MMD -MP -MF $(DDEPDIR)/$*.Td -o $@ $<
	mv -f $(DDEPDIR)/$*.Td $(DDEPDIR)/$*.d

$(OBJ)/release/test_runner_enabled.o: $(SRC)/$(FRONTEND_TEST_RUNNER_SOURCE_ID).cpp
	$(CXX) $(CC_FLAGS) $(TEST_RUNNER_SHARED_FLAGS) $(INC) -c -MT $@ -MMD -MP -MF $(RDEPDIR)/test_runner_enabled.Td -o $@ $<
	mv -f $(RDEPDIR)/test_runner_enabled.Td $(RDEPDIR)/test_runner_enabled.d

$(OBJ)/debug/test_runner_enabled.o: $(SRC)/$(FRONTEND_TEST_RUNNER_SOURCE_ID).cpp
	$(CXX) $(CC_FLAGS) $(TEST_RUNNER_SHARED_FLAGS) $(INC) -c -MT $@ -MMD -MP -MF $(DDEPDIR)/test_runner_enabled.Td -o $@ $<
	mv -f $(DDEPDIR)/test_runner_enabled.Td $(DDEPDIR)/test_runner_enabled.d

# tests/controls: a control that states its expected outcome is a self-checking
# program graded by ../scripts/check_controls.pl (see TESTING_AND_REFERENCES.md).
CONTROLS_RUN = if [ -d "$(CONTROL_TEST_ROOT)" ]; then \
		CPPGM_CONTROL_NATIVE_APP="$(NATIVE_REFERENCE_APP)" \
		perl ../scripts/run_all_tests_common.pl control_t $(CPPGM_TEST_APP) my "$(CONTROL_TEST_ROOT)" && \
		perl ../scripts/compare_results_common.pl control_t ref my "$(CONTROL_TEST_ROOT)"; \
	fi

test: $(TEST_DEPS)
	@scripts/run_all_tests.pl $(CPPGM_TEST_APP) my tests
	@scripts/compare_results.pl ref my tests
	@$(CONTROLS_RUN)

test-controls: $(TEST_DEPS)
	@$(CONTROLS_RUN)

ref-test:
	@$(REF_TEST_RUN)

REF_TEST_RUN = case "$(TEST)" in \
	$(CONTROL_TEST_ROOT)/*) echo "$(TEST): focused LowIR predicate has no reference sidecar" ;; \
	*) scripts/run_all_tests.pl $(REF_TEST_APP) ref "$(if $(strip $(TEST)),$(TEST),tests)" ;; \
esac

CHECK_RUN = case "$(TEST)" in \
	$(CONTROL_TEST_ROOT)/*) CPPGM_CONTROL_NATIVE_APP="$(NATIVE_REFERENCE_APP)" perl ../scripts/run_all_tests_common.pl control_t $(CPPGM_TEST_APP) check "$(TEST)" ;; \
	*) scripts/run_all_tests.pl $(CPPGM_TEST_APP) check "$(TEST)" ;; \
esac
CHECK_COMPARE = case "$(TEST)" in \
	$(CONTROL_TEST_ROOT)/*) perl ../scripts/compare_results_common.pl control_t ref check "$(TEST)" ;; \
	*) scripts/compare_results.pl ref check "$(TEST)" ;; \
esac

include ../scripts/pa_run_check_targets.mk

.PHONY: dev-shared-target ref-test test-controls
../dev/$(TARGET):
	$(MAKE) -C ../dev $(TARGET) CXX=$(CXX)

dev-shared-target:
	@if [ "$(CPPGM_SKIP_DEV_REBUILD)" = "1" ]; then true; else $(MAKE) -C ../dev $(TARGET) CXX=$(CXX); fi

clean:
	-rm -f $(TARGET) $(TARGET)-debug $(OBJ)/release/*.o $(OBJ)/debug/*.o

$(RDEPDIR)/%.d: ;
.PRECIOUS: $(RDEPDIR)/%.d

$(DDEPDIR)/%.d: ;
.PRECIOUS: $(DDEPDIR)/%.d

-include $(addprefix $(RDEPDIR)/,$(addsuffix .d,$(common_obj_basenames)))
-include $(addprefix $(DDEPDIR)/,$(addsuffix .d,$(common_obj_basenames)))
-include $(RDEPDIR)/test_runner_enabled.d
-include $(DDEPDIR)/test_runner_enabled.d
