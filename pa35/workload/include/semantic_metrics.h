#pragma once

#include <array>
#include <chrono>
#include <cstddef>
#include <iosfwd>
#include <map>
#include <string>
#include <vector>

namespace semantic_metrics {

enum CacheKind
{
  CK_IDENTIFIER_TOKENS = 0,
  CK_TEMPLATE_PLACEHOLDER_MENTIONS,
  CK_NON_NAMESPACE_BINDING_MENTIONS,
  CK_DEPENDENT_NON_NAMESPACE_BINDING_MENTIONS,
  CK_QUALIFIED_TYPE_LOOKUP,
  CK_DEPENDENT_TYPE_RESOLUTION,
  CK_COUNT
};

enum ClassDemandKind
{
  CDK_UNSPECIFIED = 0,
  CDK_COLLECT_DECLARATIONS,
  CDK_OUTPUT_SEED,
  CDK_FIXPOINT_REQUIRED_DEFINITION_REFRESH,
  CDK_FIXPOINT_INSTANTIATED_TEMPLATE_OUTPUT,
  CDK_FIXPOINT_SYNTHETIC_FUNCTION_OUTPUT,
  CDK_FIXPOINT_LATE_REQUIRED_FUNCTION_OUTPUT,
  CDK_FIXPOINT_LATE_REQUIRED_SYNTHESIZED_OUTPUT,
  CDK_FIXPOINT_CALLEE_CLOSURE,
  CDK_VALIDATE_REQUIRED_DEFINITIONS,
  CDK_WITNESS_UNEMITTED_BODIES,
  CDK_FIELD_MEMBER_OBJECT,
  CDK_BASE_CLASS_COLLECTION,
  CDK_NESTED_CLASS_COLLECTION,
  CDK_IMPLICIT_SPECIAL_MEMBERS,
  CDK_CLASS_VIRTUALS,
  CDK_CLASS_LAYOUT,
  CDK_OUTPUT_LAYOUT_COMPLETION,
  CDK_COUNT
};

struct CacheCounter
{
  std::size_t hits = 0;
  std::size_t misses = 0;
};

struct ClassWorkCounter
{
  std::size_t count = 0;
  std::size_t ast_children = 0;
};

struct ReferenceBeforeFullCounter
{
  std::size_t count = 0;
  std::size_t reference_ast_children = 0;
  std::size_t full_ast_children = 0;
};

struct MemberObjectCompletionCounter
{
  std::size_t count = 0;
  std::size_t complete_type_calls = 0;
  std::size_t layout_syncs = 0;
  std::size_t ast_children = 0;
};

struct AnalyzerCounters
{
  std::size_t template_instantiation_requests = 0;
  std::size_t template_definition_upgrades = 0;
  std::size_t required_definition_requests = 0;
  std::size_t required_definition_upgrades = 0;
  std::size_t fixpoint_iterations = 0;
  std::size_t output_seed_nodes = 0;
  std::size_t output_seed_output_appends = 0;
  std::size_t output_seed_state_changes = 0;
  std::size_t required_definition_refresh_scans = 0;
  std::size_t required_definition_refresh_updates = 0;
  std::size_t instantiated_class_output_scans = 0;
  std::size_t instantiated_class_output_emits = 0;
  std::size_t instantiated_function_output_scans = 0;
  std::size_t instantiated_function_output_emits = 0;
  std::size_t synthetic_function_output_scans = 0;
  std::size_t synthetic_function_output_emits = 0;
  std::size_t late_required_function_output_scans = 0;
  std::size_t late_required_function_output_emits = 0;
  std::size_t late_synthesized_class_scans = 0;
  std::size_t late_synthesized_method_scans = 0;
  std::size_t late_synthesized_method_emits = 0;
  std::size_t late_synthesized_static_function_scans = 0;
  std::size_t late_synthesized_static_function_emits = 0;
  std::size_t late_synthesized_rtti_scans = 0;
  std::size_t late_synthesized_rtti_emits = 0;
  std::size_t emitted_output_callee_top_level_scans = 0;
  std::size_t required_definition_validation_scans = 0;
  std::size_t rescanned_emitted_nodes = 0;
  std::size_t overload_candidate_sets = 0;
  std::size_t overload_candidate_attempts = 0;
  std::size_t overload_viable_candidates = 0;
  std::size_t overload_candidate_refresh_attempts = 0;
  std::size_t overload_candidate_refresh_successes = 0;
  std::size_t conversion_attempts = 0;
  std::size_t adl_associated_collections = 0;
  std::size_t adl_associated_type_visits = 0;
  std::size_t adl_associated_scope_outputs = 0;
  std::size_t adl_associated_function_outputs = 0;
  std::size_t adl_associated_template_outputs = 0;
  std::size_t adl_associated_scope_cache_hits = 0;
  std::size_t adl_associated_scope_cache_misses = 0;
  std::size_t adl_associated_scope_cache_entries = 0;
  std::size_t adl_associated_scope_cache_uncacheable = 0;
  std::size_t adl_associated_scope_cache_clears = 0;
  std::size_t candidate_identity_builds = 0;
  std::size_t candidate_identity_chars = 0;
  std::size_t candidate_partial_order_comparisons = 0;
  std::size_t candidate_partial_order_acceptance_checks = 0;
  std::size_t scope_cache_key_calls = 0;
  std::size_t semantic_only_type_text_resolutions = 0;
  std::size_t fragment_fallback_type_text_resolutions = 0;
  std::size_t function_symbol_upgrade_requests = 0;
  std::size_t function_symbol_upgrade_redeclaration_skips = 0;
  std::size_t function_template_deduction_cache_allowed_checks = 0;
  std::size_t function_template_deduction_cache_uncacheable_arg_rejects = 0;
  std::size_t function_template_deduction_cache_key_builds = 0;
  std::size_t function_template_deduction_cache_key_args = 0;
  std::size_t function_template_deduction_cache_use_scope_sensitive_keys = 0;
  std::size_t function_template_deduction_cache_hits = 0;
  std::size_t function_template_deduction_cache_misses = 0;
  std::size_t function_template_deduction_cache_entries = 0;
  std::size_t function_template_deduction_cache_clears = 0;
  std::size_t function_template_deduction_cacheable_type_checks = 0;
  std::size_t function_template_deduction_cacheable_type_hits = 0;
  std::size_t function_template_deduction_cacheable_type_scans = 0;
  std::size_t function_template_deduction_cacheable_type_entries = 0;
  std::size_t function_template_deduction_cacheable_type_clears = 0;
  std::size_t resolve_template_argument_calls = 0;
  std::size_t resolve_template_argument_single_bound_type_fast_hits = 0;
  std::size_t resolve_template_argument_simple_fast_successes = 0;
  std::size_t resolve_template_argument_simple_fast_failures = 0;
  std::size_t resolve_template_argument_simple_fast_unsupported = 0;
  std::size_t resolve_template_argument_simple_fast_expensive_successes = 0;
  std::size_t resolve_template_argument_simple_fast_expensive_failures = 0;
  std::size_t resolve_template_argument_simple_fast_expensive_unsupported = 0;
  std::size_t resolve_template_argument_fast_entry_key_builds = 0;
  std::size_t resolve_template_argument_pre_expansion_simple_type_successes = 0;
  std::size_t resolve_template_argument_pre_expansion_bound_member_failures = 0;
  std::size_t resolve_template_argument_pre_expansion_trait_bound_member_failures = 0;
  std::size_t resolve_template_argument_bound_member_failures = 0;
  std::size_t bound_member_failure_cache_hits = 0;
  std::size_t bound_member_failure_cache_entries = 0;
  std::size_t standard_enable_if_member_type_successes = 0;
  std::size_t standard_enable_if_member_type_failures = 0;
  std::size_t standard_conditional_member_type_successes = 0;
  std::size_t resolve_template_argument_fast_cache_hits = 0;
  std::size_t resolve_template_argument_cache_hits = 0;
  std::size_t resolve_template_argument_cache_misses = 0;
  std::size_t resolve_template_argument_success_entries = 0;
  std::size_t resolve_template_argument_failure_entries = 0;
  std::size_t resolve_template_argument_key_builds = 0;
  std::size_t resolve_template_argument_key_text_count = 0;
  std::size_t resolve_template_argument_key_text_chars = 0;
  std::size_t resolve_template_argument_expanded_texts = 0;
  std::size_t resolve_template_argument_source_location_calls = 0;
  std::size_t class_template_reference_requests = 0;
  std::size_t class_template_fast_existing_attempts = 0;
  std::size_t class_template_fast_existing_hits = 0;
  std::size_t class_template_fast_existing_misses = 0;
  std::size_t class_template_key_builds = 0;
  std::size_t class_template_specialization_name_builds = 0;
  std::size_t class_template_hits = 0;
  std::size_t class_template_creates = 0;
  std::size_t class_template_resets = 0;
  std::size_t class_template_canonical_arg_text_builds = 0;
  std::size_t dependent_class_template_type_shortcuts = 0;
  std::size_t class_info_for_type_definitely_not_class_skips = 0;
  std::size_t class_info_for_type_calls = 0;
  std::size_t class_info_for_type_pointer_cache_hits = 0;
  std::size_t class_info_for_type_named_key_cache_hits = 0;
  std::size_t class_info_for_type_map_lookups = 0;
  std::size_t class_info_for_type_map_hits = 0;
  std::size_t class_info_for_type_map_misses = 0;
  std::size_t complete_class_type_definitely_not_class_skips = 0;
  std::size_t complete_class_type_calls = 0;
  std::size_t complete_class_type_no_class = 0;
  std::size_t complete_class_type_already_complete = 0;
  std::size_t complete_class_type_layout_self_sync_skips = 0;
  std::size_t complete_class_type_in_progress = 0;
  std::size_t complete_class_type_materializations = 0;
  std::size_t member_object_completion_calls = 0;
  std::size_t member_object_completion_already_layout = 0;
  std::size_t member_object_completion_dependent = 0;
  std::size_t member_object_completion_no_class = 0;
  std::size_t member_object_completion_complete_type_calls = 0;
  std::size_t member_object_completion_direct_populates = 0;
  std::size_t member_object_completion_layout_syncs = 0;
  std::size_t reference_member_scope_prepares = 0;
  std::size_t reference_member_scope_ensures = 0;
  std::size_t reference_member_collections = 0;
  std::size_t instantiated_class_output_readiness_calls = 0;
  std::array<std::size_t, CDK_COUNT> class_info_for_type_by_demand;
  std::array<std::size_t, CDK_COUNT> complete_class_type_by_demand;
  std::array<std::size_t, CDK_COUNT> complete_class_type_materializations_by_demand;
  std::array<std::size_t, CDK_COUNT> class_populate_by_demand;
  std::array<std::size_t, CDK_COUNT> statement_analysis_by_demand;
  std::array<std::size_t, CDK_COUNT> expression_analysis_by_demand;
  std::array<std::size_t, CDK_COUNT> function_body_emit_by_demand;
  std::array<std::size_t, CDK_COUNT> function_body_statement_by_demand;
  std::array<std::size_t, CDK_COUNT> member_object_completion_by_parent_demand;
  std::array<std::size_t, CDK_COUNT> member_object_complete_type_by_parent_demand;
  std::array<std::size_t, CDK_COUNT> member_object_direct_populate_by_parent_demand;
  std::array<std::size_t, CDK_COUNT> reference_member_scope_prepare_by_demand;
  std::array<std::size_t, CDK_COUNT> reference_member_scope_ensure_by_demand;
  std::array<std::size_t, CDK_COUNT> reference_member_collection_by_demand;
  std::array<CacheCounter, CK_COUNT> caches;
  std::map<std::string, ClassWorkCounter> output_seed_class_materializations;
  std::map<std::string, ClassWorkCounter> reference_member_collections_by_class;
  std::map<std::string, ReferenceBeforeFullCounter> reference_before_full_by_class;
  std::map<std::string, MemberObjectCompletionCounter>
      member_object_completions_by_class;

  AnalyzerCounters();
  void note_cache_hit(CacheKind kind);
  void note_cache_miss(CacheKind kind);
  void note_output_seed_class_materialization(const std::string & class_name,
                                              std::size_t ast_children);
  void note_reference_member_collection(const std::string & class_name,
                                        std::size_t ast_children);
  void note_reference_before_full_collection(const std::string & class_name,
                                             std::size_t full_ast_children);
  void note_member_object_completion_class(const std::string & class_name,
                                           std::size_t ast_children);
  void note_member_object_completion_complete_type_call(
      const std::string & class_name);
  void note_member_object_completion_layout_sync(const std::string & class_name);
  void dump(std::ostream & out) const;
};

bool enabled();
bool phase_stats_enabled();
const char * cache_kind_name(CacheKind kind);
const char * class_demand_kind_name(ClassDemandKind kind);
ClassDemandKind current_class_demand();

class ScopedClassDemand
{
public:
  explicit ScopedClassDemand(ClassDemandKind kind);
  ~ScopedClassDemand();

  ScopedClassDemand(const ScopedClassDemand &) = delete;
  ScopedClassDemand & operator=(const ScopedClassDemand &) = delete;

private:
  ClassDemandKind previous_;
};

class ScopedPhaseTimer
{
public:
  ScopedPhaseTimer(const char * name, const std::string & detail = std::string());
  ~ScopedPhaseTimer();

private:
  using Clock = std::chrono::steady_clock;

  const char * name_;
  std::string detail_;
  bool enabled_;
  Clock::time_point start_;
};

}  // namespace semantic_metrics
