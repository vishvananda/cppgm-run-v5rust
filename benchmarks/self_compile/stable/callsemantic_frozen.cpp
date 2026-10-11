#include "callsemantic.h"
#include "callsemantic/constant_call_evaluation.h"
#include "callsemantic/constant_value_lookup.h"
#include "callsemantic/class_template_reference.h"
#include "callsemantic/function_registry.h"
#include "callsemantic/memory_census.h"
#include "callsemantic/nothrow_analysis.h"
#include "callsemantic/source_location_utils.h"
#include "callsemantic/template_body_checks.h"
#include "callsemantic/template_declaration_collector.h"
#include "callsemantic/template_source_utils.h"
#include "callsemantic/type_registry.h"
#include "callsemantic/type_trait_analysis.h"
#include "callsemantic_internal.h"
#include "callsemantic_phase_bridge.h"
#include "rtti_names.h"

#include <algorithm>
#include <cctype>
#include <functional>
#include <iostream>
#include <limits>
#include <map>
#include <memory>
#include <set>
#include <sstream>
#include <stdexcept>
#include <string>
#include <ctime>
#include <unordered_map>
#include <unordered_set>
#include <vector>

using namespace std;

#include "cpp_decl_bridge.h"
#include "symbol_linkage.h"
#include "cpp_decl_ast.h"
#include "cpp_decl_model.h"
#include "cpp_scope_lookup.h"
#include "cppast_ast.h"
#include "cppast_dump.h"
#include "cppast_parser.h"
#include "constexpr_eval.h"
#include "constructor_lifecycle_service.h"
#include "parser_trace.h"
#include "preprocessor.h"
#include "semantic_metrics.h"
#include "semantic_model.h"
#include "semantic_conversion.h"
#include "semantic_builtins.h"
#include "semantic_cache.h"
#include "semantic_class_model.h"
#include "semantic_declaration.h"
#include "semantic_errors.h"
#include "semantic_expression.h"
#include "semantic_fallback_audit.h"
#include "semantic_hotspot.h"
#include "semantic_lifetime.h"
#include "semantic_lookup.h"
#include "semantic_overload.h"
#include "output_requirement_engine.h"
#include "semantic_output.h"
#include "semantic_parameter_recovery.h"
#include "semantic_consteval.h"
#include "semantic_statement.h"
#include "semantic_trace.h"
#include "semantic_utils.h"
#include "template_angle_parser.h"
#include "template_angle_lookup.h"
#include "template_model.h"
#include "template_api.h"
#include "template_audit.h"
#include "template_argument_semantics.h"
#include "template_services.h"
#include "template_scope.h"
#include "witness_api.h"
#include "types.h"

namespace {

using namespace cpp_decl;
using namespace semantic_model;
using namespace semantic_conversion;
using namespace semantic_lookup;
using DumpNode = CallSemNode;
using template_model::TemplateArgument;
using template_model::TemplateParameterInfo;
using template_model::template_arguments_are_dependent;
using template_model::find_template_parameter_by_name;
using semantic_utils::has_top_level_comma;
using semantic_utils::is_wrapped_in_balanced_parens;
using semantic_utils::split_qualified_name_text;
using semantic_utils::strip_elaborated_type_prefix;
using semantic_utils::strip_trailing_top_level_template_arguments;
using semantic_utils::trim_space;
using semantic_utils::unqualified_member_name;

using namespace callsemantic_internal;
using callsemantic::ParsedSourceLocation;
using callsemantic::parse_physical_source_location;
using callsemantic::parse_source_location;
using callsemantic::prefer_earlier_source_location;
using callsemantic::prefer_later_source_location;
using callsemantic::source_location_is_later;
using callsemantic::NestedSourceTemplateIdOccurrence;
using callsemantic::QualifiedUseOccurrence;
using callsemantic::SourceLocationTokenView;
using callsemantic::class_member_body_has_invalid_nondependent_lookup;
using callsemantic::class_member_redeclares_template_parameter;
using callsemantic::declarator_declared_identifier;
using callsemantic::split_unqualified_template_head_text;
using callsemantic::subtree_alias_redeclares_template_parameter;
using callsemantic::template_parameter_names;


using callsemantic::source_selection_kind_for_match_kind;
using callsemantic::find_function_body_node;
using callsemantic::find_descendant_kind;
using callsemantic::first_template_id_syntax_in_subtree;
using callsemantic::qualified_template_id_syntax_in_subtree;
using callsemantic::qualified_name_syntax_text;
using callsemantic::repair_compacted_template_argument_expression_spacing;
using callsemantic::template_argument_syntax_text_preserving_spacing;
using callsemantic::template_argument_syntax_witness_source_text;
using callsemantic::template_id_syntax_text_preserving_spacing;
using callsemantic::template_id_argument_texts_preserving_spacing;
using callsemantic::template_id_argument_witness_source_texts;
using callsemantic::parameter_declarations_from_clause;
using callsemantic::normalized_template_argument_syntaxes;
using callsemantic::should_prefer_named_key_for_instantiation_identity;
using callsemantic::ScopedTemplateUseLocation;
using callsemantic::ExactTemplateTypeLookupAnchor;
using callsemantic::QualifiedOwnerClassResolution;
using callsemantic::ScopedExactTemplateTypeLookupAnchor;
using callsemantic::ScopedSuppressedTemplateUseLocation;
using callsemantic::compact_lookup_text;
using callsemantic::template_lookup_fragment_text;
using callsemantic::template_lookup_fragment_identifier;
using callsemantic::node_has_template_id_qualifier_syntax;
using callsemantic::remaining_dependent_member_template_ids;
using callsemantic::current_exact_template_type_lookup_anchor;
using callsemantic::exact_template_type_lookup_anchor_matches;
using callsemantic::exact_template_type_lookup_anchor_matches_syntax;
using callsemantic::exact_template_type_lookup_anchor_matches_identifier;
using callsemantic::exact_template_type_lookup_anchor_arg_texts;
using callsemantic::exact_template_type_lookup_anchor_arg_texts_are_full_match;
using callsemantic::exact_template_type_lookup_anchor_arg_syntaxes;
using callsemantic::source_template_id_args_are_arity_compatible;
using callsemantic::function_template_decl_primary_location;
using callsemantic::function_template_decl_location_details;
using callsemantic::function_template_trace_has_identity;
using callsemantic::function_binding_template_trace_has_source;
using callsemantic::append_function_binding_template_trace_fields;
using callsemantic::append_function_template_decl_trace_fields;
using callsemantic::function_binding_source_template_primary_location;
using callsemantic::function_binding_source_template_location_details;
using callsemantic::template_id_syntax_for_anchor;
using callsemantic::template_id_syntax_for_anchor_at_or_after_location;
using callsemantic::template_id_value_for_anchor;
using callsemantic::template_argument_texts_mention_source_bindings;
using callsemantic::template_argument_texts_mention_instantiated_class_local_type_aliases;
using callsemantic::template_argument_texts_mention_enclosing_source_template_parameters;
using callsemantic::template_argument_texts_mention_template_bound_scope_names;
using callsemantic::template_argument_texts_mention_current_specialization_names;
using callsemantic::current_specialization_source_argument_semantic_text;
using callsemantic::rewrite_current_specialization_alias_binding_texts;
using callsemantic::template_public_use_location_or;
using callsemantic::class_template_origin_decl;
using callsemantic::class_has_template_origin;
using callsemantic::class_template_origin_arguments;
using callsemantic::class_has_explicit_template_origin;
using callsemantic::class_template_origin_class_node;
using callsemantic::find_member_class_template_declaration_node;
using callsemantic::source_dependent_class_template_use_drops_;
using callsemantic::env_flag_enabled;

class Analyzer : private SemanticContext,
                 private callsemantic::TemplateDeclarationParsingServices,
                 private callsemantic::TemplateDeclarationSourceServices,
                 private callsemantic::OutOfClassMemberResolutionServices,
                 private callsemantic::FunctionTemplateDeclarationPolicy
{
public:
  Analyzer(const CppAstNode & ast,
           bool expand_output_closure = false,
           bool emit_all_source_function_definitions = false,
           witness::TemplateWitnessSession * witness_session = nullptr)
    : ast(ast),
      template_witness_session_(witness_session),
      emit_all_source_function_definitions_(emit_all_source_function_definitions)
  {
    analysis_policy_.expand_output_closure = expand_output_closure;
  }

  Analyzer(const CppAstNode & ast,
           const vector<RecogToken> & tokens,
           bool expand_output_closure = false,
           bool emit_all_source_function_definitions = false,
           witness::TemplateWitnessSession * witness_session = nullptr)
    : ast(ast),
      raw_tokens(&tokens),
      template_witness_session_(witness_session),
      emit_all_source_function_definitions_(emit_all_source_function_definitions)
  {
    analysis_policy_.expand_output_closure = expand_output_closure;
  }

  Analyzer(const CppAstNode & ast,
           IRecogTokenSequence & tokens,
           bool expand_output_closure = false,
           bool emit_all_source_function_definitions = false,
           witness::TemplateWitnessSession * witness_session = nullptr)
    : ast(ast),
      token_sequence(&tokens),
      source_locations(tokens.source_locations()),
      template_witness_session_(witness_session),
      emit_all_source_function_definitions_(emit_all_source_function_definitions)
  {
    analysis_policy_.expand_output_closure = expand_output_closure;
  }

  ~Analyzer()
  {
    if(env_flag_enabled("CPPGM_LAZY_BODY_STATS") &&
       (lazy_body_materialize_attempts != 0 ||
        lazy_body_materialize_successes != 0 ||
        lazy_body_materialize_failures != 0)) {
      cerr << "cppgm lazy-body materialize stats:"
           << " attempts=" << lazy_body_materialize_attempts
           << " successes=" << lazy_body_materialize_successes
           << " failures=" << lazy_body_materialize_failures
           << " tokens=" << lazy_body_materialize_tokens
           << '\n';
    }
  }

  DumpNode analyze()
  {
    if(ast.kind != CppAstKind::translation_unit) {
      throw logic_error("expected translation-unit");
    }

    struct MetricsDumpGuard
    {
      explicit MetricsDumpGuard(Analyzer & analyzer_in)
        : analyzer(analyzer_in)
      {}

      ~MetricsDumpGuard()
      {
        if(semantic_metrics::enabled()) {
          analyzer.metrics_.dump(std::cerr);
        }
        if(semantic_cache::stats_enabled()) {
          analyzer.cache_state_.dump(std::cerr);
        }
      }

      Analyzer & analyzer;
    } metrics_dump_guard(*this);

    witness::ScopedTemplateWitnessSession scoped_witness_session(
        template_witness_session_);
    ScopedCallSemDumpSourceLocationContext dump_source_locations(this);
    root.reset(new Scope(nullptr, "<global>", true));
    register_builtin_types(*root);
    register_builtin_functions(*root);
    {
      semantic_metrics::ScopedPhaseTimer phase(
          "semantic.record_template_source_contexts_for_witness");
      record_template_source_contexts_for_witness();
    }
    {
      semantic_metrics::ScopedPhaseTimer phase("semantic.collect_declarations");
      collect_top_level_declarations();
    }
    {
      semantic_metrics::ScopedPhaseTimer phase("semantic.determine_virtual_abi_mode");
      determine_virtual_abi_mode();
    }
    DumpNode out = make_dump_node(CallSemKind::translation_unit);
    semantic_output::OutputState state = make_output_state();

    const auto output_state_size =
        [&state]() -> size_t
        {
          return state.required_function_definitions.size() +
                 state.late_required_class_methods.size() +
                 state.late_required_class_static_functions.size() +
                 state.pending_late_required_class_methods.size() +
                 state.pending_late_required_class_static_functions.size() +
                 state.instantiated_classes.size() +
                 state.instantiated_functions.size() +
                 state.synthetic_functions.size() +
                 state.deferred_constexpr_functions.size() +
                 state.emitted_rtti_symbols.size() +
                 state.emitted_rtti_types.size() +
                 state.emitted_rtti_output.size() +
                 state.emitted_vtable_output.size();
        };

    {
      semantic_metrics::ScopedPhaseTimer phase("semantic.output_seed");
      for(size_t i = 0; i < ast.children.size(); ++i) {
        const CppAstNode & child = ast.children[i];
        if(child.kind == CppAstKind::empty_declaration) {
          continue;
        }
        const size_t previous_output_count = out.children.size();
        const size_t previous_state_size = output_state_size();
        if(semantic_metrics::AnalyzerCounters * counters = performance_counters()) {
          ++counters->output_seed_nodes;
        }
        semantic_output::analyze_declaration_output(*this, state, *root, child, out, false);
        if(semantic_metrics::AnalyzerCounters * counters = performance_counters()) {
          if(out.children.size() > previous_output_count) {
            counters->output_seed_output_appends +=
                out.children.size() - previous_output_count;
          }
          if(output_state_size() != previous_state_size) {
            ++counters->output_seed_state_changes;
          }
        }
      }
    }
    bool pending_output_changed = false;
    do {
      if(semantic_metrics::AnalyzerCounters * counters = performance_counters()) {
        ++counters->fixpoint_iterations;
      }
      const size_t previous_output_count = out.children.size();
      const size_t previous_instantiated_class_count = state.instantiated_classes.size();
      const size_t previous_instantiated_function_count = state.instantiated_functions.size();
      const size_t previous_synthetic_count = state.synthetic_functions.size();
      const size_t previous_deferred_constexpr_count =
          state.deferred_constexpr_functions.size();
      const size_t previous_required_definition_count =
          state.required_function_definitions.size();
      const size_t previous_late_class_method_count =
          state.late_required_class_methods.size();
      const size_t previous_late_class_static_function_count =
          state.late_required_class_static_functions.size();
      const size_t previous_pending_late_class_method_count =
          state.pending_late_required_class_methods.size();
      const size_t previous_pending_late_class_static_function_count =
          state.pending_late_required_class_static_functions.size();
      const std::string iteration_detail =
          std::string("iteration=") +
          std::to_string(performance_counters() ?
                             performance_counters()->fixpoint_iterations :
                             0);

      {
        semantic_metrics::ScopedPhaseTimer phase(
            "semantic.fixpoint.required_definition_refresh", iteration_detail);
        semantic_output::expand_required_function_definition_closure(*this, state);
      }
      {
        semantic_metrics::ScopedPhaseTimer phase(
            "semantic.fixpoint.instantiated_template_output", iteration_detail);
        semantic_output::analyze_instantiated_template_output(*this, state, out);
      }
      {
        semantic_metrics::ScopedPhaseTimer phase(
            "semantic.fixpoint.synthetic_function_output", iteration_detail);
        semantic_output::analyze_synthetic_function_output(*this, state, out);
      }
      {
        semantic_metrics::ScopedPhaseTimer phase(
            "semantic.fixpoint.late_required_function_output", iteration_detail);
        semantic_output::analyze_late_required_function_output(*this, state, out);
      }
      {
        semantic_metrics::ScopedPhaseTimer phase(
            "semantic.fixpoint.late_required_synthesized_output", iteration_detail);
        semantic_output::analyze_late_required_synthesized_output(*this, state, out);
      }
      {
        semantic_metrics::ScopedPhaseTimer phase(
            "semantic.fixpoint.callee_closure", iteration_detail);
        semantic_output::expand_emitted_output_callee_closure(*this, state, out);
      }

      pending_output_changed =
          out.children.size() != previous_output_count ||
          state.required_function_definitions.size() != previous_required_definition_count ||
          state.instantiated_classes.size() != previous_instantiated_class_count ||
          state.instantiated_functions.size() != previous_instantiated_function_count ||
          state.synthetic_functions.size() != previous_synthetic_count ||
          state.deferred_constexpr_functions.size() != previous_deferred_constexpr_count;
      pending_output_changed =
          pending_output_changed ||
          state.late_required_class_methods.size() != previous_late_class_method_count ||
          state.late_required_class_static_functions.size() !=
              previous_late_class_static_function_count ||
          state.pending_late_required_class_methods.size() !=
              previous_pending_late_class_method_count ||
          state.pending_late_required_class_static_functions.size() !=
              previous_pending_late_class_static_function_count;
    } while(pending_output_changed);
    {
      semantic_metrics::ScopedPhaseTimer phase("semantic.validate_required_definitions");
      semantic_output::validate_required_function_definition_closure(*this, state);
    }
    {
      semantic_metrics::ScopedPhaseTimer phase("semantic.witness_unemitted_bodies");
      analyze_unemitted_member_bodies_for_witness_semantics();
    }
    if(memory_census_enabled()) {
      dump_memory_census(std::cerr, out);
    }
    return out;
  }

private:
  using ParsedTypeTextCacheEntry = semantic_cache::ParsedTypeTextCacheEntry;
  using TextMentionCacheState = semantic_cache::TextMentionCacheState;
  using DependentTypeResolutionCacheEntry =
      semantic_cache::DependentTypeResolutionCacheEntry;

  void analyze_unemitted_member_bodies_for_witness_semantics()
  {
    if(!witness::source_capture_enabled(template_witness_context())) {
      return;
    }
    for(size_t i = 0; i < functions.size(); ++i) {
      FunctionBinding * binding = functions[i].get();
      if(!binding ||
         !binding->body ||
         !binding->owner_class ||
         !binding->owner_class->member_scope ||
         binding->output_emitted ||
         binding->definition_output_emitted ||
         binding->definition_output_in_progress ||
         template_api::function_binding_has_template_identity(binding) ||
         template_api::class_has_template_identity(binding->owner_class)) {
        continue;
      }
      semantic_output::analyze_function_body_for_witness_semantics(
          *this,
          *binding->owner_class->member_scope,
          *binding);
    }
  }

  static bool debug_bound_type_trace_enabled()
  {
    static const bool enabled = []()
    {
      const char * value = std::getenv("CPPGM_DEBUG_BOUND_TYPE_TRACE");
      return value && *value && std::string(value) != "0";
    }();
    return enabled;
  }

  static size_t debug_bound_type_trace_limit()
  {
    static const size_t limit = []() -> size_t
    {
      const char * value = std::getenv("CPPGM_DEBUG_BOUND_TYPE_TRACE_LIMIT");
      if(value == nullptr || *value == '\0') {
        return 20;
      }
      const long parsed = std::strtol(value, nullptr, 10);
      return parsed > 0 ? static_cast<size_t>(parsed) : 20;
    }();
    return limit;
  }

  static void debug_bound_type_trace(const char * kind,
                                     const string & text)
  {
    if(!debug_bound_type_trace_enabled()) {
      return;
    }
    static size_t emitted = 0;
    if(emitted >= debug_bound_type_trace_limit()) {
      return;
    }
    ++emitted;
    cerr << "BOUND_TYPE_TRACE kind=" << (kind ? kind : "<none>")
         << " text=" << text
         << '\n'
         << DiagnosticContext::format_stack()
         << '\n';
  }

  static bool debug_partial_order_escape_trap_enabled()
  {
    static const bool enabled =
        env_flag_enabled("CPPGM_DEBUG_THROW_ON_PARTIAL_ORDER_ESCAPE");
    return enabled;
  }

  bool type_contains_partial_order_artifact(const TypePtr & type) const
  {
    if(!type) {
      return false;
    }

    switch(type->kind) {
    case Type::TK_FUNDAMENTAL:
      return false;

    case Type::TK_NAMED:
      return type->named_key.find("partial-order ") == 0 ||
             (type->named_key.find("builtin ") == 0 &&
              type->named_key.find("partial-order ") != string::npos);

    case Type::TK_ATOMIC:
    case Type::TK_CV:
    case Type::TK_POINTER:
    case Type::TK_BLOCK_POINTER:
    case Type::TK_ARRAY:
    case Type::TK_RVALUE_REFERENCE:
    case Type::TK_LVALUE_REFERENCE:
      return type_contains_partial_order_artifact(type->inner);

    case Type::TK_MEMBER_POINTER:
      return type_contains_partial_order_artifact(type->inner) ||
             type_contains_partial_order_artifact(type->owner);

    case Type::TK_FUNCTION:
      if(type_contains_partial_order_artifact(type->inner)) {
        return true;
      }
      for(size_t i = 0; i < type->params.size(); ++i) {
        if(type_contains_partial_order_artifact(type->params[i])) {
          return true;
        }
      }
      return false;
    }

    return false;
  }

  std::string first_partial_order_artifact_key(const TypePtr & type) const
  {
    if(!type) {
      return std::string();
    }

    switch(type->kind) {
    case Type::TK_FUNDAMENTAL:
      return std::string();

    case Type::TK_NAMED:
      return type->named_key.find("partial-order ") == 0 ||
                     (type->named_key.find("builtin ") == 0 &&
                      type->named_key.find("partial-order ") != string::npos) ?
                 type->named_key :
                 std::string();

    case Type::TK_ATOMIC:
    case Type::TK_CV:
    case Type::TK_POINTER:
    case Type::TK_BLOCK_POINTER:
    case Type::TK_ARRAY:
    case Type::TK_RVALUE_REFERENCE:
    case Type::TK_LVALUE_REFERENCE:
      return first_partial_order_artifact_key(type->inner);

    case Type::TK_MEMBER_POINTER:
    {
      const std::string inner = first_partial_order_artifact_key(type->inner);
      return !inner.empty() ? inner : first_partial_order_artifact_key(type->owner);
    }

    case Type::TK_FUNCTION:
    {
      const std::string result = first_partial_order_artifact_key(type->inner);
      if(!result.empty()) {
        return result;
      }
      for(size_t i = 0; i < type->params.size(); ++i) {
        const std::string param = first_partial_order_artifact_key(type->params[i]);
        if(!param.empty()) {
          return param;
        }
      }
      return std::string();
    }
    }

    return std::string();
  }

  void debug_throw_on_partial_order_escape(const TypePtr & type,
                                           const char * where) const
  {
    if(!debug_partial_order_escape_trap_enabled() ||
       !type_contains_partial_order_artifact(type)) {
      return;
    }

    std::ostringstream msg;
    msg << "partial-order placeholder escaped into " << where
        << ": " << describe_type(type);
    const std::string key = first_partial_order_artifact_key(type);
    if(!key.empty()) {
      msg << " [artifact-key " << key << "]";
    }
    throw logic_error(msg.str());
  }

  static bool memory_census_enabled()
  {
    static const bool enabled = env_flag_enabled("CPPGM_MEMORY_CENSUS");
    return enabled;
  }

  static bool parsed_type_text_cache_enabled()
  {
    static const bool enabled = !env_flag_enabled("CPPGM_DISABLE_PARSED_TYPE_TEXT_CACHE");
    return enabled;
  }

  const CppAstNode & ast;
  const vector<RecogToken> * raw_tokens = nullptr;
  IRecogTokenSequence * token_sequence = nullptr;
  const SourceLocationTable * source_locations = nullptr;
  mutable template_api::template_witness_detail::SourceTokenIndex
      source_token_index_;
  witness::TemplateWitnessSession * template_witness_session_ = nullptr;
  bool emit_all_source_function_definitions_ = false;
  mutable bool qualified_use_occurrences_built_ = false;
  mutable std::vector<QualifiedUseOccurrence> qualified_use_occurrences_;
  mutable std::unordered_map<std::string, std::string>
      earliest_qualified_use_location_by_prefix_;
  unique_ptr<Scope> root;
  vector<unique_ptr<FunctionBinding> > functions;
  unordered_map<string, vector<FunctionBinding *> > functions_by_internal_symbol;
  vector<unique_ptr<ClassInfo> > classes;
  map<string, ClassInfo *> classes_by_key;
  size_t classes_by_key_version = 0;
  unordered_map<string, size_t> classes_by_key_epochs;
  map<string, Scope *> type_scopes_by_key;
  vector<unique_ptr<FunctionTemplateDecl> > function_templates;
  map<string, vector<FunctionTemplateDecl *> > pending_friend_function_templates;
  vector<unique_ptr<ClassTemplateDecl> > class_templates;
  vector<unique_ptr<AliasTemplateDecl> > alias_templates;
  vector<unique_ptr<VariableTemplateDecl> > variable_templates;
  vector<unique_ptr<Scope> > template_scopes;
  vector<unique_ptr<Scope> > captured_local_scopes;
  vector<unique_ptr<Scope> > durable_type_scopes;
  vector<unique_ptr<CppAstNode> > synthetic_ast_nodes;
  map<pair<size_t, size_t>, const CppAstNode *> materialized_lazy_function_bodies;
  size_t lazy_body_materialize_attempts = 0;
  size_t lazy_body_materialize_successes = 0;
  size_t lazy_body_materialize_failures = 0;
  size_t lazy_body_materialize_tokens = 0;
  vector<FunctionBinding *> instantiated_functions;
  unordered_set<FunctionBinding *> instantiated_function_set;
  vector<ClassInfo *> instantiated_classes;
  vector<FunctionBinding *> required_function_definitions;
  unordered_set<FunctionBinding *> required_function_definition_set;
  vector<FunctionBinding *> late_required_class_methods;
  unordered_set<FunctionBinding *> late_required_class_method_set;
  vector<FunctionBinding *> late_required_class_static_functions;
  unordered_set<FunctionBinding *> late_required_class_static_function_set;
  vector<FunctionBinding *> synthetic_functions;
  vector<FunctionBinding *> deferred_constexpr_functions;
  struct SyntheticLambdaKey
  {
    const CppAstNode * node = nullptr;
    size_t scope_key = 0;
    const FunctionBinding * function = nullptr;
    const ClassInfo * class_info = nullptr;
    string function_instantiation_key;
    string class_instantiation_key;

    bool operator<(const SyntheticLambdaKey & other) const
    {
      if(node != other.node) {
        return std::less<const CppAstNode *>()(node, other.node);
      }
      if(scope_key != other.scope_key) {
        return scope_key < other.scope_key;
      }
      if(function != other.function) {
        return std::less<const FunctionBinding *>()(function, other.function);
      }
      if(class_info != other.class_info) {
        return std::less<const ClassInfo *>()(class_info, other.class_info);
      }
      if(function_instantiation_key != other.function_instantiation_key) {
        return function_instantiation_key < other.function_instantiation_key;
      }
      return class_instantiation_key < other.class_instantiation_key;
    }
  };

  map<SyntheticLambdaKey, FunctionBinding *> synthetic_lambda_bindings;
  map<SyntheticLambdaKey, ClassInfo *> synthetic_lambda_closures;
  map<string, size_t> lambda_itanium_signature_counts;
  size_t lambda_counter = 0;
  size_t synthetic_local_counter = 0;
  size_t anonymous_enum_counter = 0;
  set<string> emitted_rtti_symbols;
  map<string, TypePtr> emitted_rtti_types;
  set<string> emitted_rtti_output;
  set<string> emitted_vtable_output;
  set<string> used_internal_symbols;
  map<string, string> exported_symbol_owners;
  map<string, string> exported_symbol_internals;
  AnalysisPolicy analysis_policy_;
  const bool collect_metrics_ = semantic_metrics::enabled();
  mutable semantic_cache::SemanticCache cache_state_;
  mutable semantic_metrics::AnalyzerCounters metrics_;
  struct ClassInfoForTypeNamedKeyCacheEntry
  {
    ClassInfo * info = nullptr;
    std::size_t epoch = 0;
    bool key_present = false;
  };
  mutable std::map<const Type *, ClassInfo *> class_info_for_type_cache_;
  mutable std::unordered_map<std::string, ClassInfoForTypeNamedKeyCacheEntry>
      class_info_for_type_named_key_cache_;
  mutable std::size_t class_info_for_type_cache_classes_by_key_version_ = 0;
  mutable long long next_live_metrics_dump_us_ = 0;
  mutable std::map<std::pair<const Scope *, std::string>, witness::ClassUseSourceDecision>
      nested_class_use_source_decision_cache_;
  std::set<const CppAstNode *> class_template_base_source_use_active_;
  std::set<const CppAstNode *> class_template_base_source_use_recorded_;

  void dump_memory_census(ostream & out,
                          const DumpNode & translation_unit) const
  {
    const callsemantic::MemoryCensusInput input = {
        root.get(),
        functions,
        classes,
        function_templates,
        class_templates,
        alias_templates,
        variable_templates,
        template_scopes,
        captured_local_scopes,
        durable_type_scopes,
        synthetic_ast_nodes,
        instantiated_functions,
        instantiated_classes,
        required_function_definitions,
        late_required_class_methods,
        late_required_class_static_functions,
        synthetic_functions,
        deferred_constexpr_functions,
        cache_state_,
        translation_unit};
    callsemantic::dump_memory_census(out, input);
  }

  size_t scope_cache_key(const Scope & scope) const
  {
    if(collect_metrics_) {
      ++metrics_.scope_cache_key_calls;
    }
    return template_scope::scope_binding_fingerprint(scope);
  }

  static void hash_combine(size_t & seed, size_t value)
  {
    seed ^= value + 0x9e3779b97f4a7c15ULL + (seed << 6) + (seed >> 2);
  }

  static void hash_combine(size_t & seed, const string & value)
  {
    hash_combine(seed, std::hash<string>()(value));
  }

  template<class Assoc>
  static void hash_assoc_keys(size_t & seed, const Assoc & assoc)
  {
    hash_combine(seed, assoc.size());
    for(typename Assoc::const_iterator it = assoc.begin(); it != assoc.end(); ++it) {
      hash_combine(seed, it->first);
    }
  }

  template<class SetLike>
  static void hash_name_set(size_t & seed, const SetLike & set_like)
  {
    hash_combine(seed, set_like.size());
    for(typename SetLike::const_iterator it = set_like.begin();
        it != set_like.end();
        ++it) {
      hash_combine(seed, *it);
    }
  }

  size_t scope_fragment_parse_fingerprint(const Scope & scope) const
  {
    return template_scope::scope_instance_fingerprint(scope);
  }

  semantic_cache::InternedTextPtr intern_query_text(const string & text) const
  {
    return cache_state_.intern_text(text);
  }

  semantic_cache::ScopeTextKey scope_text_cache_key(const Scope & scope,
                                                    const string & text) const
  {
    semantic_cache::ScopeTextKey key;
    key.scope_key = scope_cache_key(scope);
    key.text = intern_query_text(text);
    return key;
  }

  semantic_cache::ParsedTypeTextCacheKey parsed_type_text_cache_key(
      const Scope & parse_scope,
      const Scope & semantic_scope,
      const string & text,
      bool reference_class_templates_only) const
  {
    semantic_cache::ParsedTypeTextCacheKey key;
    key.parse_scope_fingerprint = scope_fragment_parse_fingerprint(parse_scope);
    key.semantic_scope_key = scope_cache_key(semantic_scope);
    key.text = intern_query_text(text);
    key.reference_class_templates_only = reference_class_templates_only;
    return key;
  }

  semantic_cache::QualifiedTypeLookupKey qualified_type_lookup_cache_key(
      size_t scope_key,
      const string & normalized_name,
      bool reference_class_templates_only,
      bool allow_dependent_class_qualifiers) const
  {
    semantic_cache::QualifiedTypeLookupKey key;
    key.scope_key = scope_key;
    key.normalized_name = intern_query_text(normalized_name);
    key.reference_class_templates_only = reference_class_templates_only;
    key.allow_dependent_class_qualifiers = allow_dependent_class_qualifiers;
    return key;
  }

  semantic_output::OutputState make_output_state()
  {
    semantic_output::OutputState state = {
        classes,
        required_function_definitions,
        late_required_class_methods,
        late_required_class_static_functions,
        instantiated_classes,
        instantiated_functions,
        synthetic_functions,
        deferred_constexpr_functions,
        emitted_rtti_symbols,
        emitted_rtti_types,
        emitted_rtti_output,
        emitted_vtable_output};
    return state;
  }

  output_requirement_engine::State make_output_requirement_state()
  {
    output_requirement_engine::State state = {
        required_function_definitions,
        required_function_definition_set,
        late_required_class_methods,
        late_required_class_method_set,
        late_required_class_static_functions,
        late_required_class_static_function_set,
        instantiated_functions,
        instantiated_function_set,
        instantiated_classes};
    return state;
  }

  output_requirement_engine::Hooks make_output_requirement_hooks()
  {
    output_requirement_engine::Hooks hooks;
    hooks.should_materialize_direct_call_output =
        [this](const FunctionBinding & binding) -> bool
        {
          return should_materialize_direct_call_output(binding);
        };
    hooks.resolve_output_function_binding =
        [this](FunctionBinding * binding) -> FunctionBinding *
        {
          return semantic_output::resolve_output_function_binding(*this, binding);
        };
    hooks.ensure_function_template_definition =
        [this](FunctionBinding * binding,
               Scope & use_scope) -> FunctionBinding *
        {
          const template_api::ScopedTemplateWitnessEntryContext entry_context =
              template_api::maybe_enter_function_body_materialization_context(*this,
                                                                              binding);
          return ensure_function_template_definition(binding, use_scope);
        };
    hooks.require_function_parameter_abi_output =
        [this](FunctionBinding * binding)
        {
          require_function_parameter_abi_output(binding);
        };
    hooks.track_instantiated_class =
        [this](ClassInfo * info)
        {
          const bool already_logged =
              template_api::class_instantiation_log_emitted(info);
          const template_api::ScopedTemplateWitnessEntryContext entry_context =
              template_api::maybe_enter_class_tracking_context(*this, info);
          track_instantiated_class(info);
          template_api::note_output_tracked_class_instantiation_if_needed(
              *this,
              info,
              already_logged);
        };
    return hooks;
  }

  SourceLocationTokenView source_location_tokens() const
  {
    return SourceLocationTokenView(source_locations, token_sequence);
  }

  const CppAstNode * materialize_lazy_function_body(const CppAstNode & body) override
  {
    if(body.kind != CppAstKind::lazy_function_body) {
      return &body;
    }

    ++lazy_body_materialize_attempts;
    const pair<size_t, size_t> key(body.token_start, body.token_end);
    map<pair<size_t, size_t>, const CppAstNode *>::const_iterator found =
        materialized_lazy_function_bodies.find(key);
    if(found != materialized_lazy_function_bodies.end()) {
      return found->second;
    }

    vector<RecogToken> body_tokens;
    if(token_sequence) {
      body_tokens = token_sequence->slice(body.token_start, body.token_end);
    } else if(raw_tokens && body.token_end <= raw_tokens->size()) {
      body_tokens.assign(raw_tokens->begin() + body.token_start,
                         raw_tokens->begin() + body.token_end);
    } else {
      ++lazy_body_materialize_failures;
      throw logic_error("cannot materialize lazy function body without token source");
    }
    body_tokens.push_back(
        RecogToken(RT_EOF, string(), static_cast<ETokenType>(0), body.source_location_id));

    CppAstParser parser(body_tokens);
    if(body.name_lookup_snapshot) {
      parser.restore_name_lookup_state_from(*body.name_lookup_snapshot);
    }
    CppAstNode parsed;
    if(!parser.parse_compound_statement_fragment(parsed)) {
      ++lazy_body_materialize_failures;
      throw logic_error(string("failed to materialize lazy function body: ") + parser.error());
    }

    lazy_body_materialize_tokens += body.token_end - body.token_start;
    synthetic_ast_nodes.push_back(unique_ptr<CppAstNode>(new CppAstNode(std::move(parsed))));
    const CppAstNode * materialized = synthetic_ast_nodes.back().get();
    materialized_lazy_function_bodies[key] = materialized;
    ++lazy_body_materialize_successes;
    return materialized;
  }

  virtual string source_location_for_node(const CppAstNode & node) const
  {
    return source_location_tokens().source_location_for_node(node);
  }

  virtual string source_location_for_name_in_node(const CppAstNode & node,
                                                  const string & name,
                                                  bool prefer_last = false) const
  {
    return source_location_tokens().source_location_for_name_in_node(
        node, name, prefer_last);
  }

  string source_location_for_name_in_subtree(const CppAstNode & node,
                                             const string & name,
                                             bool prefer_last = false) const override
  {
    return source_location_tokens().source_location_for_name_in_subtree(
        node, name, prefer_last);
  }

  enum WitnessUseLocationSearchKind
  {
    WITNESS_USE_LOCATION_NAME_IN_NODE,
    WITNESS_USE_LOCATION_NAME_IN_SUBTREE,
    WITNESS_USE_LOCATION_NODE_START
  };

  struct WitnessUseLocationSearch
  {
    WitnessUseLocationSearch(WitnessUseLocationSearchKind kind,
                             const CppAstNode * node)
      : kind(kind), node(node) {}

    WitnessUseLocationSearchKind kind;
    const CppAstNode * node;
  };

  string select_witness_use_location(
      const std::vector<WitnessUseLocationSearch> & searches,
      const string & identifier,
      bool prefer_last = false) const
  {
    if(identifier.empty()) {
      return string();
    }
    for(size_t i = 0; i < searches.size(); ++i) {
      if(searches[i].node == nullptr) {
        continue;
      }
      string location;
      switch(searches[i].kind) {
      case WITNESS_USE_LOCATION_NAME_IN_NODE:
        location = source_location_for_name_in_node(*searches[i].node,
                                                    identifier,
                                                    prefer_last);
        break;
      case WITNESS_USE_LOCATION_NAME_IN_SUBTREE:
        location = source_location_for_name_in_subtree(*searches[i].node,
                                                       identifier,
                                                       prefer_last);
        break;
      case WITNESS_USE_LOCATION_NODE_START:
        location = source_location_for_node(*searches[i].node);
        break;
      }
      location =
          template_api::normalize_template_witness_source_location(location);
      if(source_location_points_at_identifier(location, identifier)) {
        return location;
      }
    }
    return string();
  }

  virtual template_api::TemplateWitnessContext template_witness_context() const
  {
    template_api::TemplateWitnessContext out;
    out.session = template_witness_session_;
    out.source_use_table =
        template_witness_session_ ? &template_witness_session_->source_use_table :
                                    nullptr;
    out.public_use_location = parser_trace::current_use_location();
    out.token_sequence = token_sequence;
    out.source_locations = source_locations;
    out.source_token_index = &source_token_index_;
    return out;
  }

  const RecogToken * witness_token_at(size_t index) const
  {
    if(token_sequence != nullptr) {
      if(index >= token_sequence->size()) {
        return nullptr;
      }
      return &token_sequence->peek(index);
    }
    if(raw_tokens != nullptr && index < raw_tokens->size()) {
      return &(*raw_tokens)[index];
    }
    return nullptr;
  }

  bool witness_token_is_simple(size_t index, ETokenType type) const
  {
    const RecogToken * token = witness_token_at(index);
    return token != nullptr && token->is_simple(type);
  }

  string raw_source_location_for_witness_token(size_t index) const
  {
    if(source_locations == nullptr) {
      return string();
    }
    const RecogToken * token = witness_token_at(index);
    if(token == nullptr) {
      return string();
    }
    return template_api::template_witness_detail::
        source_location_for_location_id_raw(*source_locations,
                                            token->location_id);
  }

  bool template_declaration_is_explicit_specialization_class_body(
      const CppAstNode & node,
      size_t open_brace_index) const
  {
    bool saw_empty_template_parameters = false;
    for(size_t i = node.token_start; i < open_brace_index; ++i) {
      if(witness_token_is_simple(i, KW_TEMPLATE) &&
         i + 2 < open_brace_index &&
         witness_token_is_simple(i + 1, OP_LT) &&
         witness_token_is_simple(i + 2, OP_GT)) {
        saw_empty_template_parameters = true;
        continue;
      }
      if(saw_empty_template_parameters &&
         (witness_token_is_simple(i, KW_CLASS) ||
          witness_token_is_simple(i, KW_STRUCT) ||
          witness_token_is_simple(i, KW_UNION))) {
        return true;
      }
    }
    return false;
  }

  bool collect_first_declarator_identifier_for_witness(
      const CppAstNode & node,
      string & out) const
  {
    if(node.kind == CppAstKind::identifier) {
      out = unqualified_member_name(node.value);
      return !out.empty();
    }
    if(node.kind == CppAstKind::parameter_clause) {
      return false;
    }
    for(size_t i = 0; i < node.children.size(); ++i) {
      if(collect_first_declarator_identifier_for_witness(node.children[i],
                                                         out)) {
        return true;
      }
    }
    return false;
  }

  void collect_template_parameter_names_for_witness_clause(
      const CppAstNode & clause,
      vector<string> & out) const
  {
    const CppAstNode * list =
        find_child_kind(clause, CppAstKind::template_parameter_list);
    if(list == nullptr) {
      return;
    }
    set<string> names;
    for(size_t i = 0; i < list->children.size(); ++i) {
      const CppAstNode & parameter = list->children[i];
      if(parameter.kind == CppAstKind::type_parameter) {
        for(size_t j = 0; j < parameter.children.size(); ++j) {
          if(parameter.children[j].kind == CppAstKind::identifier &&
             !parameter.children[j].value.empty()) {
            names.insert(parameter.children[j].value);
          }
        }
        continue;
      }
      if(parameter.kind == CppAstKind::non_type_template_parameter) {
        for(size_t j = 0; j < parameter.children.size(); ++j) {
          if(parameter.children[j].kind != CppAstKind::declarator &&
             parameter.children[j].kind != CppAstKind::abstract_declarator) {
            continue;
          }
          string name;
          if(collect_first_declarator_identifier_for_witness(
                 parameter.children[j],
                 name)) {
            names.insert(name);
          }
          break;
        }
      }
    }
    out.assign(names.begin(), names.end());
  }

  void record_template_header_context_for_witness_node(
      const CppAstNode & node) const
  {
    if(node.kind != CppAstKind::template_declaration ||
       template_witness_session_ == nullptr ||
       source_locations == nullptr ||
       node.token_end <= node.token_start ||
       node.children.empty()) {
      return;
    }
    const CppAstNode * clause =
        find_child_kind(node, CppAstKind::template_parameter_clause);
    if(clause == nullptr) {
      return;
    }
    vector<string> parameter_names;
    collect_template_parameter_names_for_witness_clause(*clause,
                                                        parameter_names);
    if(parameter_names.empty()) {
      return;
    }

    size_t terminator_index = node.token_end;
    for(size_t i = node.token_start; i < node.token_end; ++i) {
      if(witness_token_is_simple(i, OP_LBRACE) ||
         witness_token_is_simple(i, OP_SEMICOLON)) {
        terminator_index = i;
        break;
      }
    }
    if(terminator_index == node.token_end) {
      return;
    }

    using template_api::template_witness_detail::ParsedSourceLocation;
    using template_api::template_witness_detail::parse_source_location;
    const ParsedSourceLocation begin =
        parse_source_location(
            raw_source_location_for_witness_token(node.token_start));
    const ParsedSourceLocation end =
        parse_source_location(
            raw_source_location_for_witness_token(terminator_index));
    if(!begin.valid || !end.valid || begin.file != end.file) {
      return;
    }
    template_api::record_template_witness_template_header_context(
        begin.file,
        begin.line,
        1,
        end.line,
        end.column,
        parameter_names);
  }

  void record_template_body_range_for_witness_node(
      const CppAstNode & node) const
  {
    if(node.kind != CppAstKind::template_declaration ||
       template_witness_session_ == nullptr ||
       source_locations == nullptr ||
       node.token_end <= node.token_start) {
      return;
    }

    size_t open_brace_index = node.token_end;
    for(size_t i = node.token_start; i < node.token_end; ++i) {
      if(witness_token_is_simple(i, OP_SEMICOLON)) {
        return;
      }
      if(witness_token_is_simple(i, OP_LBRACE)) {
        open_brace_index = i;
        break;
      }
    }
    if(open_brace_index == node.token_end ||
       template_declaration_is_explicit_specialization_class_body(
           node,
           open_brace_index)) {
      return;
    }

    int depth = 0;
    size_t close_brace_index = node.token_end;
    for(size_t i = open_brace_index; i < node.token_end; ++i) {
      if(witness_token_is_simple(i, OP_LBRACE)) {
        ++depth;
      } else if(witness_token_is_simple(i, OP_RBRACE)) {
        --depth;
        if(depth == 0) {
          close_brace_index = i;
          break;
        }
      }
    }
    if(close_brace_index == node.token_end) {
      return;
    }

    using template_api::template_witness_detail::ParsedSourceLocation;
    using template_api::template_witness_detail::parse_source_location;
    const ParsedSourceLocation open =
        parse_source_location(
            raw_source_location_for_witness_token(open_brace_index));
    const ParsedSourceLocation close =
        parse_source_location(
            raw_source_location_for_witness_token(close_brace_index));
    if(!open.valid || !close.valid || open.file != close.file) {
      return;
    }

    if(open.line < close.line) {
      template_api::record_template_witness_template_body_range(
          open.file,
          open.line + 1,
          close.line - 1,
          1);
    } else {
      template_api::record_template_witness_template_body_range(
          open.file,
          open.line,
          open.line,
          open.column + 1);
    }
  }

  void record_template_source_contexts_for_witness_node(
      const CppAstNode & node) const
  {
    record_template_header_context_for_witness_node(node);
    record_template_body_range_for_witness_node(node);
    for(size_t i = 0; i < node.children.size(); ++i) {
      record_template_source_contexts_for_witness_node(node.children[i]);
    }
  }

  void record_template_source_contexts_for_witness() const
  {
    if(template_witness_session_ == nullptr || source_locations == nullptr) {
      return;
    }
    record_template_source_contexts_for_witness_node(ast);
  }

  bool source_location_points_at_identifier(const std::string & location,
                                            const std::string & identifier) const
  {
    return template_api::template_witness_detail::
        source_location_points_at_identifier_token(template_witness_context(),
                                                   location,
                                                   identifier);
  }

  bool source_location_identifier_followed_by(const std::string & location,
                                              const std::string & identifier,
                                              char ch) const
  {
    return template_api::template_witness_detail::
        source_location_identifier_token_followed_by(template_witness_context(),
                                                     location,
                                                     identifier,
                                                     std::string(1, ch));
  }

  bool source_location_identifier_followed_by_indirection_type_suffix(
      const std::string & location,
      const std::string & identifier) const
  {
    return source_location_tokens()
        .source_location_identifier_followed_by_indirection_type_suffix(
            location, identifier);
  }

  std::string source_location_for_qualified_member_start_on_line(
      const std::string & location,
      const std::string & member_name) const
  {
    return source_location_tokens().source_location_for_qualified_member_start_on_line(
        location, member_name);
  }

  std::string source_location_for_last_qualified_member_start_before(
      const std::string & location,
      const std::string & before_location,
      std::string & member_name) const
  {
    return source_location_tokens()
        .source_location_for_last_qualified_member_start_before(
            location, before_location, member_name);
  }

  bool source_location_mentions_qualified_member(
      const std::string & location,
      const std::string & member_name) const
  {
    return template_api::template_witness_detail::
        source_location_line_mentions_qualified_member_token(
            template_witness_context(),
            location,
            member_name);
  }

  string source_location_for_token_index(std::size_t index) const
  {
    return source_location_tokens().source_location_for_token_index(index);
  }

  bool parsed_source_location_for_token_index(std::size_t index,
                                              ParsedSourceLocation & out) const
  {
    return source_location_tokens().parsed_source_location_for_token_index(index, out);
  }

  bool token_index_for_source_location(const std::string & location,
                                       const std::string & token_source,
                                       std::size_t & out_index) const
  {
    return source_location_tokens().token_index_for_source_location(
        location, token_source, out_index);
  }

  bool token_index_at_source_location(const std::string & location,
                                      std::size_t & out_index) const
  {
    return source_location_tokens().token_index_at_source_location(
        location, out_index);
  }

  bool find_next_token_source_on_same_line(std::size_t start_index,
                                           const std::string & token_source,
                                           std::size_t & out_index) const
  {
    return source_location_tokens().find_next_token_source_on_same_line(
        start_index, token_source, out_index);
  }

  bool tokens_are_adjacent(std::size_t left_index,
                           std::size_t right_index) const
  {
    return source_location_tokens().tokens_are_adjacent(left_index, right_index);
  }

  std::string compact_token_range_text(std::size_t begin,
                                       std::size_t end) const
  {
    return source_location_tokens().compact_token_range_text(begin, end);
  }

  std::string compact_template_id_token_text(std::size_t begin,
                                             std::size_t close_index,
                                             int close_chars) const
  {
    return source_location_tokens().compact_template_id_token_text(
        begin, close_index, close_chars);
  }

  bool template_name_before_open(std::size_t open_index,
                                 std::size_t & name_begin,
                                 std::size_t & anchor_index) const
  {
    return source_location_tokens().template_name_before_open(
        open_index, name_begin, anchor_index);
  }

  bool matching_template_close_token(std::size_t open_index,
                                     std::size_t & close_index,
                                     int & close_chars,
                                     bool same_line_only = true) const
  {
    return source_location_tokens().matching_template_close_token(
        open_index, close_index, close_chars, same_line_only);
  }

  std::vector<NestedSourceTemplateIdOccurrence>
  nested_template_id_source_occurrences_at_location(
      const std::string & location,
      bool same_line_only = true) const
  {
    return source_location_tokens().nested_template_id_source_occurrences_at_location(
        location, same_line_only);
  }

  bool template_argument_token_ranges_from_open(
      std::size_t open_index,
      std::vector<std::pair<std::size_t, std::size_t> > & ranges) const
  {
    return source_location_tokens().template_argument_token_ranges_from_open(
        open_index, ranges);
  }

  string preferred_source_location_for_node(const CppAstNode & node) const
  {
    std::string best = source_location_for_node(node);
    for(std::size_t i = 0; i < node.children.size(); ++i) {
      best = prefer_later_source_location(best,
                                          preferred_source_location_for_node(
                                              node.children[i]));
    }
    return best;
  }

  string earliest_child_source_location_for_node(const CppAstNode & node) const
  {
    std::string best;
    for(std::size_t i = 0; i < node.children.size(); ++i) {
      best = prefer_earlier_source_location(
          best,
          source_location_for_node(node.children[i]));
      best = prefer_earlier_source_location(
          best,
          earliest_child_source_location_for_node(node.children[i]));
    }
    return best;
  }

  void ensure_qualified_use_occurrences_built() const
  {
    if(qualified_use_occurrences_built_) {
      return;
    }
    qualified_use_occurrences_built_ = true;
    qualified_use_occurrences_.clear();
    earliest_qualified_use_location_by_prefix_.clear();
    for(std::size_t i = 0; i < ast.children.size(); ++i) {
      const CppAstNode & child = ast.children[i];
      callsemantic::collect_qualified_use_occurrences(
          child,
          source_location_tokens(),
          template_witness_context(),
          qualified_use_occurrences_);
    }
    for(std::size_t i = 0; i < qualified_use_occurrences_.size(); ++i) {
      const QualifiedUseOccurrence & occurrence = qualified_use_occurrences_[i];
      if(occurrence.value.empty() || occurrence.location.empty()) {
        continue;
      }
      earliest_qualified_use_location_by_prefix_[occurrence.value] =
          prefer_earlier_source_location(
              earliest_qualified_use_location_by_prefix_[occurrence.value],
              occurrence.location);
      std::size_t pos = 0;
      while((pos = occurrence.value.find("::", pos)) != std::string::npos) {
        if(pos != 0) {
          const std::string prefix = occurrence.value.substr(0, pos);
          earliest_qualified_use_location_by_prefix_[prefix] =
              prefer_earlier_source_location(
                  earliest_qualified_use_location_by_prefix_[prefix],
                  occurrence.location);
        }
        pos += 2;
      }
    }
  }

  std::string earliest_qualified_use_location_for_prefix(
      const std::string & prefix) const override
  {
    ensure_qualified_use_occurrences_built();
    std::unordered_map<std::string, std::string>::const_iterator cached =
        earliest_qualified_use_location_by_prefix_.find(prefix);
    if(cached != earliest_qualified_use_location_by_prefix_.end()) {
      return cached->second;
    }

    std::string earliest =
        callsemantic::earliest_qualified_use_location_for_prefix(
            qualified_use_occurrences_,
            prefix);

    earliest_qualified_use_location_by_prefix_[prefix] = earliest;
    return earliest;
  }

  std::string earliest_qualified_use_location_for_value(const std::string & value) const
  {
    ensure_qualified_use_occurrences_built();
    return callsemantic::earliest_qualified_use_location_for_value(
        qualified_use_occurrences_,
        value);
  }

  string source_file_for_node(const CppAstNode * node) const
  {
    if(node == nullptr || source_locations == nullptr || token_sequence == nullptr) {
      return string();
    }

    const RecogToken & token = token_sequence->peek(node->token_start);
    if(token.location_id == 0 ||
       token.location_id >= source_locations->locations.size()) {
      return string();
    }

    const SourceLocation & location =
        source_locations->locations[token.location_id];
    if(location.file_index >= source_locations->files.size()) {
      return string();
    }
    return source_locations->files[location.file_index];
  }

  static bool path_has_directory_prefix(const string & path, const string & prefix)
  {
    if(prefix.empty() || path.size() < prefix.size() ||
       path.compare(0, prefix.size(), prefix) != 0) {
      return false;
    }
    if(path.size() == prefix.size()) {
      return true;
    }
    if(prefix[prefix.size() - 1] == '/' || prefix[prefix.size() - 1] == '\\') {
      return true;
    }
    return path[prefix.size()] == '/' || path[prefix.size()] == '\\';
  }

  bool node_comes_from_standard_include_path(const CppAstNode * node) const override
  {
    const string source_file = source_file_for_node(node);
    if(source_file.empty()) {
      return false;
    }
    const vector<string> & stdinc = standard_include_paths();
    for(size_t i = 0; i < stdinc.size(); ++i) {
      if(path_has_directory_prefix(source_file, stdinc[i])) {
        return true;
      }
    }
    return false;
  }

  bool definition_comes_from_standard_include_path(const CppAstNode * declaration_node,
                                                   const CppAstNode * body,
                                                   bool is_defaulted) const override
  {
    const bool declaration_is_definition_syntax =
        declaration_node &&
        (declaration_node->kind == CppAstKind::function_definition ||
         declaration_node->kind == CppAstKind::special_member_definition);
    if(declaration_is_definition_syntax) {
      return node_comes_from_standard_include_path(declaration_node);
    }
    if(body != nullptr) {
      return node_comes_from_standard_include_path(body);
    }
    if(is_defaulted) {
      return node_comes_from_standard_include_path(declaration_node);
    }
    return false;
  }

  bool definition_prefers_local_object_binding(const CppAstNode * declaration_node,
                                               const CppAstNode * body,
                                               bool is_defaulted) const
  {
    return definition_comes_from_standard_include_path(declaration_node,
                                                       body,
                                                       is_defaulted);
  }

  bool owner_class_comes_from_standard_include_path(const ClassInfo * owner_class) const
  {
    return owner_class &&
           (node_comes_from_standard_include_path(owner_class->class_node) ||
            node_comes_from_standard_include_path(owner_class->template_output_node));
  }

  bool function_prefers_local_object_binding(const CppAstNode * declaration_node,
                                             const CppAstNode * body,
                                             bool is_defaulted,
                                             const ClassInfo * owner_class,
                                             bool is_constructor,
                                             bool is_destructor,
                                             bool is_copy_assignment,
                                             bool is_move_assignment) const
  {
    if(definition_prefers_local_object_binding(declaration_node, body, is_defaulted)) {
      return true;
    }
    return !declaration_node &&
           !body &&
           (is_constructor ||
            is_destructor ||
            is_copy_assignment ||
            is_move_assignment) &&
           owner_class_comes_from_standard_include_path(owner_class);
  }

  bool uses_extended_virtual_abi() const
  {
    return analysis_policy_.use_extended_virtual_abi;
  }

  semantic_builtins::RegistrationHooks make_builtin_registration_hooks()
  {
    semantic_builtins::RegistrationHooks hooks;
    hooks.register_function =
        [this](Scope & scope,
               const string & name,
               const TypePtr & result_type,
               const vector<TypePtr> & params,
               bool explicit_nothrow)
        {
          register_builtin_function(scope, name, result_type, params, explicit_nothrow);
        };
    return hooks;
  }

  string normalize_type_lookup_name(const string & text) const
  {
    return callsemantic_internal::normalize_type_lookup_name(text);
  }

  const IdentifierTokenSet & identifier_tokens_for(const string & text) const
  {
    const semantic_cache::InternedTextPtr text_key = intern_query_text(text);
    unordered_map<semantic_cache::InternedTextPtr, IdentifierTokenSet>::iterator found =
        cache_state_.identifier_token_cache.find(text_key);
    if(found != cache_state_.identifier_token_cache.end()) {
      metrics_.note_cache_hit(semantic_metrics::CK_IDENTIFIER_TOKENS);
      return found->second;
    }
    metrics_.note_cache_miss(semantic_metrics::CK_IDENTIFIER_TOKENS);
    return cache_state_.identifier_token_cache.insert(
        make_pair(text_key, collect_identifier_tokens(*text_key))).first->second;
  }

  bool is_conversion_function_name(const string & name) const
  {
    return ::is_conversion_function_name(name);
  }

  void set_expr_info_metadata(ExprInfo & expr,
                              const TypePtr & type,
                              ValueCategory category) const
  {
    expr.type = type;
    expr.category = category;
    ::set_expr_metadata(expr.node, expr.type, expr.category);
  }

  struct ScopedAnalysisPolicyOverride
  {
    Analyzer & analyzer;
    AnalysisPolicy saved_policy;

    ScopedAnalysisPolicyOverride(Analyzer & analyzer_,
                                 const AnalysisPolicy & override_policy)
      : analyzer(analyzer_),
        saved_policy(analyzer_.analysis_policy_)
    {
      analyzer.analysis_policy_ = override_policy;
    }
    ~ScopedAnalysisPolicyOverride()
    {
      analyzer.analysis_policy_ = saved_policy;
    }
  };

  bool instantiate_function_bodies_enabled() const
  {
    return analysis_policy_.instantiate_function_bodies;
  }

  bool materialize_user_defined_output_enabled() const
  {
    return analysis_policy_.materialize_user_defined_output;
  }

  string spaced_node_text(const CppAstNode & node) const override
  {
    if(raw_tokens && has_valid_node_span(*raw_tokens, node)) {
      return spaced_token_span_text(*raw_tokens, node.token_start, node.token_end);
    }
    if(token_sequence && has_valid_node_span(*token_sequence, node)) {
      return spaced_token_span_text(*token_sequence, node.token_start, node.token_end);
    }
    return node.value;
  }

  void determine_virtual_abi_mode()
  {
    analysis_policy_.use_extended_virtual_abi = false;
    for(size_t i = 0; i < classes.size(); ++i) {
      ClassInfo & info = *classes[i];
      if(semantic_class_model::class_has_virtual_bases(info) ||
         info.vtables.size() > 1) {
        analysis_policy_.use_extended_virtual_abi = true;
        return;
      }
    }
  }

  void register_builtin_types(Scope & scope)
  {
    semantic_builtins::register_builtin_types(scope);
  }

  void register_builtin_functions(Scope & scope)
  {
    semantic_builtins::register_builtin_functions(scope,
                                                  make_builtin_registration_hooks());
  }

  TypePtr rtti_handle_type() const
  {
    return make_pointer(make_cv(make_fundamental(FT_VOID), true, false));
  }

  TypePtr make_enum_type(Scope & scope,
                         const string & enum_prefix,
                         const string & name,
                         bool complete,
                         size_t alignment = 4,
                         size_t size = 4)
  {
    string display = enum_prefix + " " + name;
    string key = enum_prefix + " " + scope_qualified_name(scope, name);
    return make_named(display, key, complete, true, alignment, size);
  }

  void assign_anonymous_enum_itanium_metadata(Scope & scope,
                                             const TypePtr & enum_type)
  {
    if(!scope.class_info || !scope.class_info->type || !enum_type) {
      return;
    }

    string owner_encoding;
    vector<string> owner_substitution_keys;
    if(!symbol_linkage::mangle_itanium_type_encoding_with_substitutions(
           scope.class_info->type,
           owner_encoding,
           &owner_substitution_keys) ||
       owner_encoding.empty()) {
      return;
    }

    string encoding;
    if(owner_encoding.size() >= 2 &&
       owner_encoding[0] == 'N' &&
       owner_encoding[owner_encoding.size() - 1] == 'E') {
      encoding = owner_encoding.substr(0, owner_encoding.size() - 1);
    } else {
      encoding = 'N' + owner_encoding;
    }
    encoding += "Ut_E";

    TypePtr base = strip_top_level_cv(enum_type);
    if(!base || base->kind != Type::TK_NAMED) {
      return;
    }
    base->named_itanium_abi_encoding = encoding;
    base->named_itanium_abi_substitution_keys = owner_substitution_keys;
  }

  Scope & append_namespace_scope(Scope & parent, const string & name)
  {
    parent.namespace_children.emplace_back(new Scope(&parent, name, true));
    return *parent.namespace_children.back();
  }

  Scope * find_named_namespace_child(Scope & scope, const string & name)
  {
    for(size_t i = 0; i < scope.namespace_children.size(); ++i) {
      Scope * child = scope.namespace_children[i].get();
      if(child->name == name) {
        return child;
      }
    }
    return nullptr;
  }

  bool scope_has_template_placeholders(Scope & scope) const
  {
    const bool concrete_instantiated_class_scope =
        template_api::class_has_non_dependent_source_template_identity(scope.class_info);
    unordered_set<string> seen_type_names;
    unordered_set<string> seen_type_pack_names;
    unordered_set<string> seen_value_names;
    seen_type_names.reserve(16);
    seen_type_pack_names.reserve(8);
    seen_value_names.reserve(16);
    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope || current->parent == nullptr) {
        break;
      }
      if(concrete_instantiated_class_scope &&
         current != &scope &&
         current->class_info == nullptr) {
        break;
      }
      if(current != &scope &&
         template_api::class_has_non_dependent_source_template_identity(
             current->class_info)) {
        break;
      }
      for(const auto & named : current->named_types) {
        if(seen_type_names.count(named.first) == 0 &&
           type_depends_on_template_parameter(named.second)) {
          return true;
        }
      }
      for(const auto & pack : current->named_type_packs) {
        if(seen_type_pack_names.count(pack.first) != 0) {
          continue;
        }
        for(size_t i = 0; i < pack.second.size(); ++i) {
          if(type_depends_on_template_parameter(pack.second[i])) {
            return true;
          }
        }
      }
      for(const auto & value : current->values) {
        if(seen_value_names.count(value.first) != 0) {
          continue;
        }
        if(value_binding_depends_on_template_parameters(value.second)) {
          return true;
        }
      }
      for(const auto & named : current->named_types) {
        seen_type_names.insert(named.first);
      }
      for(const auto & pack : current->named_type_packs) {
        seen_type_pack_names.insert(pack.first);
      }
      for(const auto & value : current->values) {
        seen_value_names.insert(value.first);
      }
    }
    return false;
  }

  bool scope_is_inside_source_template_context(Scope & scope) const
  {
    return callsemantic::scope_is_inside_source_template_context(scope);
  }

  void require_function_definition(FunctionBinding * binding,
                                   OutputReason reason,
                                   bool enabled = true)
  {
    CallableEmissionDecision decision =
        decide_callable_emission(binding, reason, enabled);
    if(!decision.should_seed_definition) {
      return;
    }
    ++metrics_.required_definition_requests;
    FunctionBinding * original_binding = binding;
    output_requirement_engine::State state = make_output_requirement_state();
    output_requirement_engine::Hooks hooks = make_output_requirement_hooks();
    binding = output_requirement_engine::require_definition(state, hooks, binding, reason, true);
    if(original_binding && binding && binding != original_binding) {
      ++metrics_.required_definition_upgrades;
    }
    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "require-function-definition function=" << binding->name
            << " reason=" << static_cast<int>(reason)
            << " owner="
            << (binding->owner_class ? binding->owner_class->qualified_name :
                                       std::string("<none>"))
            << " key=" << template_api::function_binding_template_trace_key(binding)
            << " internal-symbol=" << binding->symbol.internal_symbol
            << " object-symbol=" << binding->symbol.object_symbol
            << " has-definition=" << (binding->has_definition ? "yes" : "no")
            << " source-template="
            << (template_api::function_binding_has_source_template_identity(binding) ?
                    "yes" :
                    "no")
            << " excluded-explicit="
            << (template_api::function_binding_excluded_from_explicit_instantiation(*binding) ?
                    "yes" :
                    "no")
            << " output-requirements=" << binding->output_requirements;
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
  }

  CallableEmissionDecision decide_callable_emission(FunctionBinding * binding,
                                                    OutputReason reason,
                                                    bool request_definition = true) const
  {
    CallableEmissionDecision result;
    if(!request_definition || !binding || binding->is_deleted) {
      return result;
    }

    const bool direct_callable_reason =
        reason == OutputReason::ConstructorUse ||
        reason == OutputReason::DirectCall ||
        reason == OutputReason::FunctionIdUse ||
        reason == OutputReason::NewExpression;
    if(direct_callable_reason &&
       !should_materialize_direct_call_output(*binding)) {
      return result;
    }
    if((reason == OutputReason::ConstructorUse ||
        reason == OutputReason::SyntheticDependency) &&
       is_trivial_special_member_binding(*binding)) {
      return result;
    }

    const bool immediate_class_static_direct_call =
        direct_callable_reason &&
        analysis_policy_.expand_output_closure &&
        binding->owner_class &&
        !binding->is_method &&
        !binding->is_constructor &&
        !binding->is_destructor &&
        binding->has_definition;

    result.should_seed_definition = true;
    result.mark_output_required_now =
        !direct_callable_reason ||
        !analysis_policy_.expand_output_closure ||
        immediate_class_static_direct_call;
    return result;
  }

  FunctionBinding * refresh_required_function_definition(FunctionBinding * binding,
                                                         bool insert_if_missing = true)
  {
    output_requirement_engine::State state = make_output_requirement_state();
    output_requirement_engine::Hooks hooks = make_output_requirement_hooks();
    binding = output_requirement_engine::refresh_required_definition_binding(state,
                                                                            hooks,
                                                                            binding,
                                                                            insert_if_missing);
    return binding;
  }

  void upgrade_function_symbol_linkage(FunctionBinding * binding,
                                       symbol_linkage::SymbolLinkage linkage)
  {
    if(!binding) {
      return;
    }

    upgrade_function_symbol_linkage(*binding,
                                    function_binding_qualified_name_for_symbol(*binding),
                                    function_binding_display_name_for_symbol(*binding),
                                    binding->type,
                                    linkage);
  }

  void note_instantiated_function_output(
      FunctionBinding * binding,
      InstantiatedFunctionOutputMode mode = InstantiatedFunctionOutputMode::TrackOnly)
  {
    output_requirement_engine::State state = make_output_requirement_state();
    output_requirement_engine::Hooks hooks = make_output_requirement_hooks();
    output_requirement_engine::note_instantiation(state, hooks, binding, mode);
    if(!binding) {
      return;
    }

    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "note-instantiated-function-output function=" << binding->name
            << " mode=track-only"
            << " owner="
            << (binding->owner_class ? binding->owner_class->qualified_name :
                                       std::string("<none>"))
            << " key=" << template_api::function_binding_template_trace_key(binding)
            << " internal-symbol=" << binding->symbol.internal_symbol
            << " object-symbol=" << binding->symbol.object_symbol;
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
  }

  bool should_materialize_direct_call_output(const FunctionBinding & binding) const
  {
    const auto has_direct_callable_definition =
        [](const FunctionBinding & candidate) -> bool
        {
          return candidate.has_definition &&
                 (symbol_linkage::has_weak_linkage(candidate.symbol) ||
                  candidate.symbol.linkage == symbol_linkage::SL_INTERNAL);
        };

    if(!analysis_policy_.materialize_direct_call_output) {
      return false;
    }
    if(binding.is_deleted) {
      return false;
    }
    if(binding.suppress_implicit_instantiation_definition &&
       template_api::function_binding_has_source_template_identity(&binding) &&
       !binding.has_definition) {
      return false;
    }
    if(analysis_policy_.expand_output_closure) {
      return true;
    }

    if(template_api::class_has_template_identity(binding.owner_class)) {
      const bool class_template_member_definition =
          binding.has_definition;
      const bool direct_template_member_definition =
          template_api::function_binding_has_source_template_identity(&binding) &&
          (binding.is_method || binding.is_constructor || binding.is_destructor);
      const bool class_template_member_use =
          binding.is_method ||
          binding.is_constructor ||
          binding.is_destructor ||
          binding.declaration_scope;
      return binding.synthesized ||
             binding.is_defaulted ||
             class_template_member_definition ||
             direct_template_member_definition ||
             class_template_member_use;
    }

    if(binding.owner_class) {
      return binding.synthesized ||
             binding.is_defaulted ||
             has_direct_callable_definition(binding);
    }

    return template_api::function_binding_has_source_template_identity(&binding) ||
           binding.synthesized ||
           binding.is_defaulted ||
           has_direct_callable_definition(binding);
  }

  bool expand_output_closure_enabled() const
  {
    return analysis_policy_.expand_output_closure;
  }

  const AnalysisPolicy & current_analysis_policy() const
  {
    return analysis_policy_;
  }

  bool emit_all_source_function_definitions() const
  {
    return emit_all_source_function_definitions_;
  }

  semantic_metrics::AnalyzerCounters * performance_counters()
  {
    return collect_metrics_ ? &metrics_ : nullptr;
  }

  bool function_binding_is_live(const FunctionBinding * binding) const
  {
    if(!binding) {
      return false;
    }
    for(size_t i = 0; i < functions.size(); ++i) {
      if(functions[i].get() == binding) {
        return true;
      }
    }
    return false;
  }

  static bool live_metrics_enabled()
  {
    static const bool enabled = env_flag_enabled("CPPGM_SEMANTIC_STATS_LIVE");
    return enabled;
  }

  static long long live_metrics_step_us()
  {
    static const long long step = []() -> long long
    {
      const char * value = std::getenv("CPPGM_SEMANTIC_STATS_LIVE_STEP_MS");
      if(value && *value) {
        const long parsed = std::strtol(value, nullptr, 10);
        if(parsed > 0) {
          return static_cast<long long>(parsed) * 1000LL;
        }
      }
      return 5000000LL;
    }();
    return step;
  }

  static long long monotonic_time_us()
  {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return static_cast<long long>(ts.tv_sec) * 1000000LL +
           static_cast<long long>(ts.tv_nsec / 1000);
  }

  void maybe_dump_live_metrics() const
  {
    if(!collect_metrics_ || !live_metrics_enabled()) {
      return;
    }
    const long long now = monotonic_time_us();
    if(next_live_metrics_dump_us_ == 0) {
      next_live_metrics_dump_us_ = now + live_metrics_step_us();
      return;
    }
    if(now < next_live_metrics_dump_us_) {
      return;
    }
    metrics_.dump(std::cerr);
    next_live_metrics_dump_us_ = now + live_metrics_step_us();
  }

  void note_performance_counter(
      std::size_t semantic_metrics::AnalyzerCounters::* field,
      std::size_t amount = 1)
  {
    if(collect_metrics_) {
      (metrics_.*field) += amount;
      maybe_dump_live_metrics();
    }
  }

  void note_late_required_class_method(FunctionBinding * binding)
  {
    output_requirement_engine::State state = make_output_requirement_state();
    output_requirement_engine::note_late_required_class_method(state, binding);
  }

  callsemantic::TypeTraitCallbacks type_trait_callbacks() const
  {
    callsemantic::TypeTraitCallbacks callbacks;
    callbacks.class_info_for_type =
        [this](const TypePtr & type) -> ClassInfo *
        {
          return class_info_for_type(type);
        };
    return callbacks;
  }

  callsemantic::NothrowCallbacks nothrow_callbacks()
  {
    callsemantic::NothrowCallbacks callbacks;
    callbacks.make_type_trait_callbacks =
        [this]() -> callsemantic::TypeTraitCallbacks
        {
          return type_trait_callbacks();
        };
    callbacks.complete_class_type =
        [this](const TypePtr & type) -> ClassInfo *
        {
          return complete_class_type(type);
        };
    callbacks.analyze_expression_for_target =
        [this](Scope & scope,
               const CppAstNode & node,
               const TypePtr & target) -> ExprInfo
        {
          return analyze_expression_for_target(scope, node, target);
        };
    callbacks.select_default_constructor =
        [this](Scope & scope, ClassInfo & info) -> FunctionBinding *
        {
          vector<ExprInfo> args_out;
          vector<ExprInfo> source_args;
          const ConstructorSelectionOptions ctor_options =
              constructor_lifecycle_service::selection_options_for(
                  constructor_lifecycle_service::no_user_defined_constructor_profile(
                      "nothrow default initialization"));
          return select_constructor_from_exprs(scope,
                                               info,
                                               source_args,
                                               args_out,
                                               nullptr,
                                               ctor_options);
        };
    callbacks.evaluate_explicit_function_nothrow_semantically =
        [this](FunctionBinding & binding, bool & out) -> bool
        {
          return evaluate_explicit_function_nothrow_semantically(binding, out);
        };
    callbacks.lookup_type =
        [this](Scope & scope, const string & name) -> TypePtr
        {
          return lookup_type(scope, name);
        };
    callbacks.destructor_for =
        [this](ClassInfo & info) -> FunctionBinding *
        {
          return destructor_for(info);
        };
    callbacks.copy_assignment_for =
        [this](ClassInfo & info) -> FunctionBinding *
        {
          return copy_assignment_for(info);
        };
    callbacks.move_assignment_for =
        [this](ClassInfo & info) -> FunctionBinding *
        {
          return move_assignment_for(info);
        };
    callbacks.resolve_dump_callee_binding =
        [this](const DumpNode & callee, FunctionBinding *& binding) -> bool
        {
          return resolve_dump_callee_binding(callee, binding);
        };
    callbacks.is_declval_dump_callee =
        [this](Scope & scope, const DumpNode & callee) -> bool
        {
          return is_declval_dump_callee(scope, callee);
        };
    return callbacks;
  }

  callsemantic::FunctionRegistryState function_registry_state()
  {
    callsemantic::FunctionRegistryState state = {
        functions,
        functions_by_internal_symbol,
        used_internal_symbols,
        instantiated_functions,
        instantiated_function_set,
        required_function_definitions,
        required_function_definition_set,
        late_required_class_methods,
        late_required_class_method_set,
        late_required_class_static_functions,
        late_required_class_static_function_set,
        synthetic_functions,
        deferred_constexpr_functions};
    return state;
  }

  callsemantic::FunctionRegistryCallbacks function_registry_callbacks() const
  {
    callsemantic::FunctionRegistryCallbacks callbacks;
    callbacks.types_equivalent_for_member_binding =
        [this](const TypePtr & lhs, const TypePtr & rhs) -> bool
        {
          return types_equivalent_for_member_binding(lhs, rhs);
        };
    return callbacks;
  }

  callsemantic::TypeRegistryState type_registry_state()
  {
    callsemantic::TypeRegistryState state = {
        classes,
        classes_by_key,
        classes_by_key_version,
        classes_by_key_epochs,
        type_scopes_by_key,
        instantiated_classes};
    return state;
  }

  callsemantic::TypeRegistryCallbacks type_registry_callbacks()
  {
    callsemantic::TypeRegistryCallbacks callbacks;
    callbacks.direct_named_type =
        [this](Scope & scope, const string & name) -> TypePtr
        {
          return direct_named_type(scope, name);
        };
    callbacks.persistent_enclosing_scope =
        [this](Scope & scope) -> Scope &
        {
          return persistent_enclosing_scope(scope);
        };
    callbacks.current_location_note =
        [this](const CppAstNode * node) -> string
        {
          return semantic_trace::current_location_note(*this, node);
        };
    callbacks.previous_class_location_note =
        [this](const char * label, const ClassInfo * info) -> string
        {
          return semantic_trace::previous_class_location_note(
              *this, label, info);
        };
    return callbacks;
  }

  bool text_mentions_template_placeholders(Scope & scope,
                                           const string & text) const
  {
    const semantic_cache::ScopeTextKey cache_key =
        scope_text_cache_key(scope, text);
    std::unordered_map<semantic_cache::ScopeTextKey,
                       semantic_cache::TextMentionCacheState,
                       semantic_cache::ScopeTextKeyHash>::iterator found =
        cache_state_.template_placeholder_mentions_cache.find(cache_key);
    if(found != cache_state_.template_placeholder_mentions_cache.end()) {
      metrics_.note_cache_hit(semantic_metrics::CK_TEMPLATE_PLACEHOLDER_MENTIONS);
      if(found->second == semantic_cache::TMCS_IN_PROGRESS) {
        return true;
      }
      return found->second == semantic_cache::TMCS_TRUE;
    }
    metrics_.note_cache_miss(semantic_metrics::CK_TEMPLATE_PLACEHOLDER_MENTIONS);

    semantic_cache::TextMentionCacheState & cache_entry =
        cache_state_.template_placeholder_mentions_cache[cache_key];
    cache_entry = semantic_cache::TMCS_IN_PROGRESS;
    const bool result =
        template_api::type::text_mentions_template_placeholders(
            *const_cast<Analyzer *>(this), scope, text);
    cache_entry =
        result ? semantic_cache::TMCS_TRUE : semantic_cache::TMCS_FALSE;
    return result;
  }

  bool qualifier_identifier_uses_template_binding(Scope & scope,
                                                  const string & identifier) const
  {
    return template_api::type::qualifier_identifier_uses_template_binding(
        *const_cast<Analyzer *>(this), scope, identifier);
  }

  bool template_id_syntax_mentions_template_binding(
      Scope & scope,
      const TemplateIdSyntax & syntax) const
  {
    std::function<bool(const CppAstNode &)> ast_mentions;
    std::function<bool(const TemplateArgumentSyntax &)> argument_mentions;
    std::function<bool(const TemplateIdSyntax &)> template_id_mentions;

    const auto identifier_mentions = [&](const std::string & text) -> bool
    {
      return qualifier_identifier_uses_template_binding(
          scope,
          unqualified_member_name(
              strip_trailing_top_level_template_arguments(text)));
    };

    template_id_mentions = [&](const TemplateIdSyntax & current) -> bool
    {
      if(identifier_mentions(current.name.name)) {
        return true;
      }
      for(size_t i = 0; i < current.name.qualifiers.size(); ++i) {
        if(identifier_mentions(current.name.qualifiers[i])) {
          return true;
        }
      }
      for(size_t i = 0; i < current.argument_syntaxes.size(); ++i) {
        if(argument_mentions(current.argument_syntaxes[i])) {
          return true;
        }
      }
      return false;
    };

    ast_mentions = [&](const CppAstNode & node) -> bool
    {
      if(node.kind == CppAstKind::id_expression) {
        if(const QualifiedName * qualified = cppast_qualified_name_syntax(node)) {
          for(size_t i = 0; i < qualified->qualifiers.size(); ++i) {
            if(identifier_mentions(qualified->qualifiers[i])) {
              return true;
            }
          }
          if(identifier_mentions(qualified->name)) {
            return true;
          }
        } else if(identifier_mentions(node.value)) {
          return true;
        }
      } else if(is_identifier_text(node.value) && identifier_mentions(node.value)) {
        return true;
      }

      if(const TemplateIdSyntax * node_template_id =
             cppast_template_id_syntax(node)) {
        if(template_id_mentions(*node_template_id)) {
          return true;
        }
      }
      for(size_t i = 0; i < node.qualifier_template_id_syntaxes.size(); ++i) {
        if(template_id_mentions(node.qualifier_template_id_syntaxes[i])) {
          return true;
        }
      }
      for(size_t i = 0; i < node.children.size(); ++i) {
        if(ast_mentions(node.children[i])) {
          return true;
        }
      }
      return false;
    };

    argument_mentions = [&](const TemplateArgumentSyntax & argument) -> bool
    {
      if(argument.dependent) {
        return true;
      }
      if(argument.template_id && template_id_mentions(*argument.template_id)) {
        return true;
      }
      if(argument.type_id && ast_mentions(*argument.type_id)) {
        return true;
      }
      if(argument.expression && ast_mentions(*argument.expression)) {
        return true;
      }
      return is_identifier_text(argument.text) && identifier_mentions(argument.text);
    };

    return template_id_mentions(syntax);
  }

  bool ast_node_has_leading_typename_qualified_member(
      const CppAstNode & node) const
  {
    if(node.has_leading_typename) {
      if(const QualifiedName * qualified = cppast_qualified_name_syntax(node)) {
        if(!qualified->qualifiers.empty()) {
          return true;
        }
      }
    }
    for(size_t i = 0; i < node.children.size(); ++i) {
      if(ast_node_has_leading_typename_qualified_member(node.children[i])) {
        return true;
      }
    }
    return false;
  }

  bool template_argument_syntax_preserves_qualified_member(
      const TemplateArgumentSyntax & syntax) const
  {
    return (syntax.type_id &&
            ast_node_has_leading_typename_qualified_member(*syntax.type_id)) ||
           (syntax.expression &&
            ast_node_has_leading_typename_qualified_member(*syntax.expression));
  }

  bool template_argument_preserves_qualified_member(
      const TemplateArgument & argument) const
  {
    if(argument.kind != TemplateArgument::TA_TYPE || !argument.type) {
      return false;
    }
    TypePtr owner;
    vector<string> members;
    bool leading_typename = false;
    return named_type_dependent_qualified_member(argument.type,
                                                owner,
                                                members,
                                                leading_typename,
                                                nullptr) &&
           !members.empty();
  }

  void mark_qualified_member_source_bindings(
      std::vector<template_api::TemplateWitnessSourceBinding> & bindings,
      const std::vector<TemplateArgument> & arguments,
      const std::vector<TemplateArgumentSyntax> * source_arg_syntaxes) const
  {
    const size_t argument_limit = std::min(bindings.size(), arguments.size());
    for(size_t i = 0; i < argument_limit; ++i) {
      if(template_argument_preserves_qualified_member(arguments[i])) {
        bindings[i].preserve_qualified_member = true;
      }
    }
    if(!source_arg_syntaxes) {
      return;
    }
    const size_t syntax_limit = std::min(bindings.size(),
                                         source_arg_syntaxes->size());
    for(size_t i = 0; i < syntax_limit; ++i) {
      if(template_argument_syntax_preserves_qualified_member(
             (*source_arg_syntaxes)[i])) {
        bindings[i].preserve_qualified_member = true;
      }
    }
  }

  bool compute_text_mentions_non_namespace_binding_names(
      Scope & scope,
      const string & text) const
  {
    return template_api::type::text_mentions_non_namespace_binding_names(
        *const_cast<Analyzer *>(this), scope, text);
  }

  bool text_mentions_non_namespace_binding_names(Scope & scope,
                                                 const string & text) const
  {
    const semantic_cache::ScopeTextKey cache_key =
        scope_text_cache_key(scope, text);
    std::unordered_map<semantic_cache::ScopeTextKey,
                       bool,
                       semantic_cache::ScopeTextKeyHash>::iterator found =
        cache_state_.non_namespace_binding_mentions_cache.find(cache_key);
    if(found != cache_state_.non_namespace_binding_mentions_cache.end()) {
      metrics_.note_cache_hit(
          semantic_metrics::CK_NON_NAMESPACE_BINDING_MENTIONS);
      return found->second;
    }
    metrics_.note_cache_miss(semantic_metrics::CK_NON_NAMESPACE_BINDING_MENTIONS);

    const bool result =
        template_api::type::text_mentions_non_namespace_binding_names(
            *const_cast<Analyzer *>(this), scope, text);
    cache_state_.non_namespace_binding_mentions_cache[cache_key] = result;
    return result;
  }

  bool text_mentions_current_specialization_names(Scope & scope,
                                                  const string & text) const
  {
    return template_api::type::text_mentions_current_specialization_names(
        *const_cast<Analyzer *>(this), scope, text);
  }

  bool text_mentions_dependent_non_namespace_binding_names(
      Scope & scope,
      const string & text) const
  {
    const semantic_cache::ScopeTextKey cache_key =
        scope_text_cache_key(scope, text);
    std::unordered_map<semantic_cache::ScopeTextKey,
                       bool,
                       semantic_cache::ScopeTextKeyHash>::iterator found =
        cache_state_.dependent_non_namespace_binding_mentions_cache.find(
            cache_key);
    if(found != cache_state_.dependent_non_namespace_binding_mentions_cache.end()) {
      metrics_.note_cache_hit(
          semantic_metrics::CK_DEPENDENT_NON_NAMESPACE_BINDING_MENTIONS);
      return found->second;
    }
    metrics_.note_cache_miss(
        semantic_metrics::CK_DEPENDENT_NON_NAMESPACE_BINDING_MENTIONS);

    const bool result =
        template_api::type::text_mentions_dependent_non_namespace_binding_names(
            *const_cast<Analyzer *>(this), scope, text);
    cache_state_.dependent_non_namespace_binding_mentions_cache[cache_key] =
        result;
    return result;
  }

  bool value_binding_depends_on_template_parameters(const ValueBinding & binding) const
  {
    if(binding.dependent_template_value ||
       type_depends_on_template_parameter(binding.type)) {
      return true;
    }
    if(binding.has_constant_value || binding.has_constexpr_value) {
      return false;
    }
    if(!binding.constant_initializer || !binding.constant_initializer_scope) {
      return false;
    }
    return text_mentions_template_placeholders(*binding.constant_initializer_scope,
                                               node_text(*binding.constant_initializer));
  }

  vector<string> expand_bound_expression_pack_texts(Scope & scope,
                                                    const string & text)
  {
    return template_api::type::expand_bound_expression_pack_texts(
        *this,
        scope,
        text);
  }

  vector<string> rewrite_decltype_expression_pack_texts(Scope & scope,
                                                        const string & text)
  {
    return template_api::type::rewrite_decltype_expression_pack_texts(
        *this, scope, text);
  }

  bool should_defer_unresolved_type_lookup(Scope & scope,
                                           const string & text) const
  {
    return text_mentions_template_placeholders(scope, text) ||
           text_mentions_dependent_non_namespace_binding_names(scope, text);
  }

  bool should_defer_member_collection_template_recursion(
      Scope & scope,
      const string & text,
      ClassTemplateDecl * class_template) const
  {
    ClassInfo * info = scope.class_info;
    if(!info ||
       info->member_scope.get() != &scope ||
       !template_api::class_source_template_identity_matches(info, class_template) ||
       !info->full_member_collection_in_progress ||
       !info->template_instantiation_in_progress ||
       info->complete) {
      return false;
    }

    return text_mentions_non_namespace_binding_names(scope, text);
  }

  ClassTemplateDecl * direct_class_template(Scope & scope, const string & name)
  {
    map<string, ClassTemplateDecl *>::iterator found = scope.class_templates.find(name);
    return found == scope.class_templates.end() ? nullptr : found->second;
  }

  string canonicalize_template_parameter_redeclaration_text(
      const vector<TemplateParameterInfo> & parameters,
      const string & text) const
  {
    return callsemantic::canonicalize_template_parameter_redeclaration_text(
        parameters, text);
  }

  bool template_parameter_redeclarations_compatible(
      const vector<TemplateParameterInfo> & existing_parameters,
      const vector<TemplateParameterInfo> & incoming_parameters,
      size_t index) const
  {
    return callsemantic::template_parameter_redeclarations_compatible(
        existing_parameters, incoming_parameters, index);
  }

  bool merge_template_parameter_redeclarations(vector<TemplateParameterInfo> & target,
                                               const vector<TemplateParameterInfo> & incoming)
  {
    return callsemantic::merge_template_parameter_redeclarations(target, incoming);
  }

  void prefer_incoming_template_parameter_spellings(vector<TemplateParameterInfo> & target,
                                                    const vector<TemplateParameterInfo> & incoming)
  {
    callsemantic::prefer_incoming_template_parameter_spellings(target, incoming);
  }

  vector<FunctionTemplateDecl *> direct_function_templates(Scope & scope, const string & name)
  {
    return semantic_lookup::lookup_direct_function_templates(scope, name);
  }

  VariableTemplateDecl * direct_variable_template(Scope & scope, const string & name)
  {
    map<string, VariableTemplateDecl *>::iterator found = scope.variable_templates.find(name);
    return found == scope.variable_templates.end() ? nullptr : found->second;
  }

  bool parse_decltype_specifier(Scope & scope,
                                const CppAstNode & node,
                                TypePtr & out)
  {
    const string & text = node.value;
    const string decltype_prefix = "decltype(";
    const string decltype_gnu_prefix = "__decltype(";
    const string decltype_gnu_alt_prefix = "__decltype__(";
    const string typeof_prefix = "__typeof(";
    const string typeof_alt_prefix = "__typeof__(";

    size_t prefix_size = 0;
    bool is_typeof = false;
    if(text.size() >= decltype_prefix.size() + 1 &&
       text.compare(0, decltype_prefix.size(), decltype_prefix) == 0 &&
       text[text.size() - 1] == ')') {
      prefix_size = decltype_prefix.size();
    } else if(text.size() >= decltype_gnu_prefix.size() + 1 &&
              text.compare(0, decltype_gnu_prefix.size(), decltype_gnu_prefix) == 0 &&
              text[text.size() - 1] == ')') {
      prefix_size = decltype_gnu_prefix.size();
    } else if(text.size() >= decltype_gnu_alt_prefix.size() + 1 &&
              text.compare(0, decltype_gnu_alt_prefix.size(), decltype_gnu_alt_prefix) == 0 &&
              text[text.size() - 1] == ')') {
      prefix_size = decltype_gnu_alt_prefix.size();
    } else if(text.size() >= typeof_prefix.size() + 1 &&
              text.compare(0, typeof_prefix.size(), typeof_prefix) == 0 &&
              text[text.size() - 1] == ')') {
      prefix_size = typeof_prefix.size();
      is_typeof = true;
    } else if(text.size() >= typeof_alt_prefix.size() + 1 &&
              text.compare(0, typeof_alt_prefix.size(), typeof_alt_prefix) == 0 &&
              text[text.size() - 1] == ')') {
      prefix_size = typeof_alt_prefix.size();
      is_typeof = true;
    } else {
      return false;
    }

    string inner = trim_space(text.substr(prefix_size, text.size() - prefix_size - 1));
    bool parenthesized = false;
    if(!is_typeof && is_wrapped_in_balanced_parens(inner)) {
      parenthesized = true;
      inner = trim_space(inner.substr(1, inner.size() - 2));
    }

    if(inner.empty()) {
      return false;
    }
    const string source_inner = inner;
    const CppAstNode * operand = node.children.empty() ? nullptr : &node.children[0];

    const bool mentions_bound_names =
        text_mentions_non_namespace_binding_names(scope, inner);
    const bool mentions_dependent_names =
        text_mentions_dependent_non_namespace_binding_names(scope, inner);
    const auto dependent_fallback =
        [&]() -> bool
    {
      if(scope_has_template_placeholders(scope) || mentions_bound_names) {
        if(operand) {
          out = make_dependent_type_expression_type(
              text,
              is_typeof ? Type::NSK_DEPENDENT_TYPEOF :
                          Type::NSK_DEPENDENT_DECLTYPE,
              text,
              node);
        } else {
          out = make_semantic_named(
              text,
              is_typeof ? Type::NSK_DEPENDENT_TYPEOF :
                          Type::NSK_DEPENDENT_DECLTYPE,
              text,
              true);
        }
        return true;
      }
      return false;
    };

    if(is_typeof) {
      if(operand && operand->kind == CppAstKind::type_id &&
         parse_type_id(scope, *operand, out, false)) {
        return true;
      }
      if(!operand && parse_type_text_scoped(scope, scope, inner, out, false)) {
        return true;
      }
    }

    const vector<string> rewritten_exprs = rewrite_decltype_expression_pack_texts(scope, inner);
    if(rewritten_exprs.size() != 1) {
      return dependent_fallback();
    }
    inner = rewritten_exprs[0];

    if(!operand || operand->kind == CppAstKind::type_id) {
      return dependent_fallback();
    }
    const CppAstNode & expr = *operand;

    const string decltype_expression_location =
        template_api::normalize_template_witness_source_location(
            source_location_for_node(expr));
    const ScopedTemplateUseLocation decltype_use_location_guard(
        !decltype_expression_location.empty() ?
            decltype_expression_location :
            template_public_use_location_or(string()));
    semantic_overload::CallAnalysisHints decltype_call_hints;
    decltype_call_hints.use_location =
        !decltype_expression_location.empty() ?
            decltype_expression_location :
            template_public_use_location_or(string());

    try {
      AnalysisPolicy suppressed_policy = analysis_policy_;
      suppressed_policy.instantiate_function_bodies = false;
      ScopedAnalysisPolicyOverride suppression(*this, suppressed_policy);
      if(!is_typeof && expr.kind == CppAstKind::delete_expression) {
        const bool is_array_delete =
            !expr.children.empty() && expr.children[0].kind == CppAstKind::array_delete;
        const CppAstNode * delete_operand =
            expr.children.empty() ? nullptr : &expr.children[is_array_delete ? 1 : 0];
        if(!delete_operand) {
          return dependent_fallback();
        }
        ExprInfo pointer = analyze_expression(scope, *delete_operand);
        TypePtr pointer_type = strip_top_level_cv(remove_reference_type(pointer.type));
        if(!pointer_type || pointer_type->kind != Type::TK_POINTER) {
          return dependent_fallback();
        }
        out = make_fundamental(FT_VOID);
        return true;
      }
      ExprInfo info =
          expr.kind == CppAstKind::call_expression ?
              analyze_call_expression(
                  scope,
                  expr,
                  semantic_policy::without_body_instantiation(&decltype_call_hints)) :
              analyze_expression(scope, expr);
      if(!info.type) {
        return dependent_fallback();
      }
      if(is_typeof) {
        out = info.type;
        return true;
      }
      if(!parenthesized &&
         (expr.kind == CppAstKind::id_expression ||
          expr.kind == CppAstKind::member_expression)) {
        if(expr.kind == CppAstKind::id_expression) {
          const QualifiedName * qualified = cppast_qualified_name_syntax(expr);
          const ValueBinding * binding =
              qualified && (qualified->rooted || !qualified->qualifiers.empty()) ?
                  semantic_lookup::lookup_qualified_value_binding(*this, scope, *qualified) :
                  lookup_value(scope, expr.value);
          if(binding && binding->type) {
            out = binding->type;
          } else {
            const TemplateIdSyntax * template_id =
                cppast_template_id_syntax(expr);
            vector<FunctionBinding *> functions =
                template_id ?
                    lookup_function_template_id(
                        scope,
                        *template_id,
                        semantic_policy::default_call_analysis()) :
                    lookup_functions(scope,
                                     expr.value,
                                     semantic_policy::default_call_analysis());
            out = functions.size() == 1 && functions[0] ? functions[0]->type : info.type;
          }
        } else {
          out = info.type;
        }
      } else if(info.category == VC_LVALUE) {
        out = collapse_lvalue_reference_type(info.type);
      } else if(info.category == VC_XVALUE) {
        out = collapse_rvalue_reference_type(info.type);
      } else {
        out = info.type;
      }
      return true;
    } catch(const logic_error & ex) {
      if(parser_trace::enabled("template.resolve")) {
        std::ostringstream trace;
        trace << "parse-" << (is_typeof ? "typeof" : "decltype")
              << "-fallback text=" << text
              << " inner=" << inner
              << " reason=" << ex.what()
              << " placeholders=" << (scope_has_template_placeholders(scope) ? "yes" : "no")
              << " mentions-bound=" << (mentions_bound_names ? "yes" : "no")
              << " mentions-dependent=" << (mentions_dependent_names ? "yes" : "no");
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      return dependent_fallback();
    }
  }

  TypePtr current_specialization_type_for_template_id(Scope & scope,
                                                      const QualifiedName & template_id,
                                                      const vector<string> & arg_texts,
                                                      const vector<TemplateArgumentSyntax> * arg_syntaxes)
  {
    for(Scope * current = &scope; current; current = current->parent) {
      template_api::ClassTemplateUseInfo use;
      if(!current->class_info ||
         !template_api::class_template_use_info_for_class(
             *this, *current, current->class_info, use, false) ||
         !use.has_stored_key ||
         !template_api::template_id_matches_class_template_origin(
             template_id, use)) {
        continue;
      }

      if(use.canonical_texts && *use.canonical_texts == arg_texts) {
        return use.instance->type;
      }

      vector<TemplateArgument> current_specialization_arguments;
      if(resolve_template_arguments(*current,
                                    *use.parameters,
                                    arg_texts,
                                    arg_syntaxes,
                                    current_specialization_arguments,
                                    use.origin->declaring_scope) &&
         template_args_identity_key(current_specialization_arguments) ==
             use.key) {
        return use.instance->type;
      }
      break;
    }
    return TypePtr();
  }

  string instantiation_arg_text_list(const vector<string> & arg_texts) const
  {
    string out;
    for(size_t i = 0; i < arg_texts.size(); ++i) {
      if(i != 0) {
        out += ",";
      }
      out += arg_texts[i];
    }
    return out;
  }

  TypePtr current_specialization_type_for_lookup_text(Scope & scope,
                                                      const string & normalized_name)
  {
    if(semantic_hotspot::enabled()) {
      semantic_hotspot::note_semantic_query("current_specialization_type_for_lookup_text",
                                            normalized_name);
    }
    debug_bound_type_trace("current_specialization_type_for_lookup_text", normalized_name);
    for(Scope * current = &scope; current; current = current->parent) {
      template_api::ClassTemplateUseInfo use;
      if(!current->class_info ||
         !template_api::class_template_use_info_for_class(
             *this, *current, current->class_info, use, false) ||
         !use.canonical_texts ||
         use.canonical_texts->empty()) {
        continue;
      }

      const auto match_current_class_name =
          [&](const string & candidate) -> TypePtr
          {
            if(candidate.empty()) {
              return TypePtr();
            }
            TypePtr matched =
                match_wrapped_type_text(normalized_name,
                                        candidate,
                                        use.instance->type);
            if(matched) {
              return matched;
            }
            return TypePtr();
          };
      if(TypePtr matched = match_current_class_name(current->class_info->name)) {
        return matched;
      }
      if(TypePtr matched = match_current_class_name(use.instance->qualified_name)) {
        return matched;
      }
      if(TypePtr matched = match_current_class_name(use.instance->display_qualified_name)) {
        return matched;
      }

      const string arg_list =
          instantiation_arg_text_list(*use.canonical_texts);
      const string injected_unqualified = use.origin->name;
      const string unqualified =
          injected_unqualified + "<" + arg_list + ">";
      TypePtr matched = match_wrapped_type_text(normalized_name,
                                                injected_unqualified,
                                                use.instance->type);
      if(matched) {
        return matched;
      }
      matched = match_wrapped_type_text(normalized_name,
                                        unqualified,
                                        use.instance->type);
      if(matched) {
        return matched;
      }
      if(use.origin->declaring_scope) {
        const string injected_qualified =
            scope_qualified_name(*use.origin->declaring_scope, use.origin->name);
        const string qualified =
            injected_qualified + "<" + arg_list + ">";
        matched = match_wrapped_type_text(normalized_name,
                                          injected_qualified,
                                          use.instance->type);
        if(matched) {
          return matched;
        }
        matched = match_wrapped_type_text(normalized_name,
                                          qualified,
                                          use.instance->type);
        if(matched) {
          return matched;
        }
      }
    }
    return TypePtr();
  }

  bool type_depends_on_template_parameter(const TypePtr & type) const
  {
    if(!type) {
      return false;
    }

    switch(type->kind) {
    case Type::TK_FUNDAMENTAL:
      return false;

    case Type::TK_NAMED:
      if(named_type_has_dependent_semantic(type) ||
         named_type_key_contains_dependent_semantic(type)) {
        return true;
      }
      if(ClassInfo * info = class_info_for_type(type)) {
        if(template_api::class_template_instantiation_depends_on_template_parameter(
               *const_cast<Analyzer *>(this), *info)) {
          return true;
        }
      }
      return false;

    case Type::TK_ATOMIC:
      return type_depends_on_template_parameter(type->inner);

    case Type::TK_CV:
    case Type::TK_POINTER:
    case Type::TK_MEMBER_POINTER:
    case Type::TK_BLOCK_POINTER:
    case Type::TK_LVALUE_REFERENCE:
    case Type::TK_RVALUE_REFERENCE:
    case Type::TK_ARRAY:
      return type_depends_on_template_parameter(type->inner) ||
             (type->kind == Type::TK_MEMBER_POINTER &&
              type_depends_on_template_parameter(type->owner));

    case Type::TK_FUNCTION:
      if(type_depends_on_template_parameter(type->inner)) {
        return true;
      }
      for(size_t i = 0; i < type->params.size(); ++i) {
        if(type_depends_on_template_parameter(type->params[i])) {
          return true;
        }
      }
      return false;
    }

    return false;
  }

  bool class_layout_depends_on_template_parameters(const ClassInfo & info) const
  {
    const bool class_scope_dependent =
        info.member_scope && scope_has_template_placeholders(*info.member_scope);
    for(size_t i = 0; i < info.bases.size(); ++i) {
      if(info.bases[i].type &&
         (!info.bases[i].type->type->named_has_layout ||
          type_depends_on_template_parameter(info.bases[i].type->type))) {
        return true;
      }
    }

    for(size_t i = 0; i < info.fields.size(); ++i) {
      if(type_depends_on_template_parameter(info.fields[i].type)) {
        return true;
      }
      TypePtr field_type = strip_top_level_cv(remove_reference_type(info.fields[i].type));
      if(class_scope_dependent &&
         field_type && field_type->kind == Type::TK_NAMED &&
         !field_type->named_has_layout) {
        return true;
      }
      ClassInfo * field_class = class_info_for_type(info.fields[i].type);
      if(template_api::class_has_source_template_identity(field_class) &&
         !field_class->type->named_has_layout) {
        return true;
      }
    }

    return false;
  }

  TypePtr collapse_lvalue_reference_type(const TypePtr & inner) const
  {
    if(!inner) {
      return inner;
    }
    TypePtr base = strip_top_level_cv(inner);
    if(base && (base->kind == Type::TK_LVALUE_REFERENCE ||
                base->kind == Type::TK_RVALUE_REFERENCE)) {
      return make_lvalue_reference_raw(base->inner);
    }
    return make_lvalue_reference_raw(inner);
  }

  TypePtr collapse_rvalue_reference_type(const TypePtr & inner) const
  {
    if(!inner) {
      return inner;
    }
    TypePtr base = strip_top_level_cv(inner);
    if(base && base->kind == Type::TK_LVALUE_REFERENCE) {
      return make_lvalue_reference_raw(base->inner);
    }
    if(base && base->kind == Type::TK_RVALUE_REFERENCE) {
      return make_rvalue_reference_raw(base->inner);
    }
    return make_rvalue_reference_raw(inner);
  }

  bool sizeof_depends_on_template_parameters(const TypePtr & type) const
  {
    if(!type) {
      return false;
    }

    if(type_depends_on_template_parameter(type)) {
      return true;
    }

    TypePtr base = strip_top_level_cv(remove_reference_type(type));
    if(!base) {
      return false;
    }

    if(base->kind == Type::TK_ARRAY) {
      return sizeof_depends_on_template_parameters(base->inner);
    }

    if(base->kind != Type::TK_NAMED) {
      return false;
    }

    ClassInfo * info = class_info_for_type(base);
    return info && info->complete && !base->named_has_layout &&
           class_layout_depends_on_template_parameters(*info);
  }

  void finalize_dependent_class_shape(ClassInfo & info)
  {
    info.type->named_complete = true;
    info.type->named_has_layout = false;
    info.complete = true;
  }

  bool template_arguments_are_dependent(const vector<TemplateArgument> & arguments) const
  {
    return template_api::template_arguments_are_dependent(
        *const_cast<Analyzer *>(this), arguments);
  }

  bool current_class_member_scope_has_unqualified_member_name(
      Scope & scope,
      const std::string & name,
      const ClassTemplateDecl * required_source_template = nullptr,
      bool type_members_only = false) const
  {
    if(name.empty()) {
      return false;
    }
    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope) {
        break;
      }
      if(!(current->class_info &&
           template_api::class_has_source_template_identity(current->class_info) &&
           current->class_info->member_scope)) {
        continue;
      }
      if(required_source_template &&
         !template_api::class_source_template_identity_matches(
             current->class_info,
             required_source_template)) {
        continue;
      }
      if(current->template_bound_type_names.count(name) != 0 ||
         current->template_bound_type_pack_names.count(name) != 0 ||
         current->template_bound_value_names.count(name) != 0 ||
         current->template_bound_template_names.count(name) != 0) {
        continue;
      }
      const Scope & members = *current->class_info->member_scope;
      if(members.named_types.find(name) != members.named_types.end() ||
         (!type_members_only &&
          members.values.find(name) != members.values.end())) {
        return true;
      }
    }
    return false;
  }

  bool qualified_name_is_unqualified_current_class_member(
      Scope & scope,
      const QualifiedName & qualified,
      const ClassTemplateDecl * required_source_template = nullptr,
      bool type_members_only = false) const
  {
    return !qualified.rooted &&
           qualified.qualifiers.empty() &&
           current_class_member_scope_has_unqualified_member_name(scope,
                                                                 qualified.name,
                                                                 required_source_template,
                                                                 type_members_only);
  }

  bool template_id_syntax_mentions_unqualified_current_class_member(
      Scope & scope,
      const TemplateIdSyntax & syntax,
      const ClassTemplateDecl * required_source_template = nullptr,
      bool type_members_only = false) const
  {
    if(qualified_name_is_unqualified_current_class_member(
           scope,
           syntax.name,
           required_source_template,
           type_members_only)) {
      return true;
    }
    for(size_t i = 0; i < syntax.argument_syntaxes.size(); ++i) {
      if(template_argument_syntax_mentions_unqualified_current_class_member(
             scope,
             syntax.argument_syntaxes[i],
             required_source_template,
             type_members_only)) {
        return true;
      }
    }
    return false;
  }

  bool ast_node_mentions_unqualified_current_class_member(
      Scope & scope,
      const CppAstNode & node,
      const ClassTemplateDecl * required_source_template = nullptr,
      bool type_members_only = false) const
  {
    if(node.qualified_name_syntax &&
       qualified_name_is_unqualified_current_class_member(
           scope,
           *node.qualified_name_syntax,
           required_source_template,
           type_members_only)) {
      return true;
    }
    if((node.kind == CppAstKind::identifier ||
        node.kind == CppAstKind::id_expression ||
        node.kind == CppAstKind::type_name ||
       node.kind == CppAstKind::decl_specifier ||
       node.kind == CppAstKind::type_specifier) &&
       current_class_member_scope_has_unqualified_member_name(scope,
                                                             node.value,
                                                             required_source_template,
                                                             type_members_only)) {
      return true;
    }
    if(node.template_id_syntax &&
       template_id_syntax_mentions_unqualified_current_class_member(
           scope,
           *node.template_id_syntax,
           required_source_template,
           type_members_only)) {
      return true;
    }
    for(size_t i = 0; i < node.qualifier_template_id_syntaxes.size(); ++i) {
      if(template_id_syntax_mentions_unqualified_current_class_member(
             scope,
             node.qualifier_template_id_syntaxes[i],
             required_source_template,
             type_members_only)) {
        return true;
      }
    }
    for(size_t i = 0; i < node.qualifier_type_syntaxes.size(); ++i) {
      if(ast_node_mentions_unqualified_current_class_member(
             scope,
             node.qualifier_type_syntaxes[i],
             required_source_template,
             type_members_only)) {
        return true;
      }
    }
    for(size_t i = 0; i < node.children.size(); ++i) {
      if(ast_node_mentions_unqualified_current_class_member(scope,
                                                           node.children[i],
                                                           required_source_template,
                                                           type_members_only)) {
        return true;
      }
    }
    return false;
  }

  bool template_argument_syntax_mentions_unqualified_current_class_member(
      Scope & scope,
      const TemplateArgumentSyntax & syntax,
      const ClassTemplateDecl * required_source_template = nullptr,
      bool type_members_only = false) const
  {
    if(syntax.template_id &&
       template_id_syntax_mentions_unqualified_current_class_member(
           scope,
           *syntax.template_id,
           required_source_template,
           type_members_only)) {
      return true;
    }
    if(syntax.type_id &&
       ast_node_mentions_unqualified_current_class_member(scope,
                                                         *syntax.type_id,
                                                         required_source_template,
                                                         type_members_only)) {
      return true;
    }
    if(syntax.expression &&
       ast_node_mentions_unqualified_current_class_member(scope,
                                                         *syntax.expression,
                                                         required_source_template,
                                                         type_members_only)) {
      return true;
    }
    return false;
  }

  bool template_argument_syntaxes_mention_unqualified_current_class_member(
      Scope & scope,
      const vector<TemplateArgumentSyntax> & syntaxes,
      const ClassTemplateDecl * required_source_template = nullptr,
      bool type_members_only = false) const
  {
    for(size_t i = 0; i < syntaxes.size(); ++i) {
      if(template_argument_syntax_mentions_unqualified_current_class_member(
             scope,
             syntaxes[i],
             required_source_template,
             type_members_only)) {
        return true;
      }
    }
    return false;
  }

  bool try_builtin_type_transform(Scope & scope,
                                  const string & text,
                                  TypePtr & out)
  {
    return semantic_builtins::try_builtin_type_transform(*this, scope, text, out);
  }

  bool is_named_enum_type(const TypePtr & type) const
  {
    TypePtr base = strip_top_level_cv(type);
    if(base && base->kind == Type::TK_NAMED) {
      if(base->named_key.compare(0, 5, "enum ") == 0) {
        return true;
      }
    }
    ClassInfo * info = class_info_for_type(type);
    return info && info->class_kind == "enum";
  }

  bool is_named_union_type(const TypePtr & type)
  {
    ClassInfo * info = class_info_for_type(type);
    return info && info->class_kind == "union";
  }

  bool is_named_class_type(const TypePtr & type)
  {
    ClassInfo * info = class_info_for_type(type);
    return info && info->class_kind != "union" && info->class_kind != "enum";
  }

  bool is_operator_overload_operand_type(const TypePtr & type)
  {
    TypePtr base = strip_top_level_cv(remove_reference_type(type));
    return base && (is_named_class_type(base) || is_named_enum_type(base));
  }

  bool requires_class_or_enum_operand_for_nonmember_operator(const string & name) const
  {
    if(!is_builtin_operator_function_name(name)) {
      return false;
    }
    const string unqualified = unqualified_member_name(name);
    return unqualified != "operator new" &&
           unqualified != "operator new[]" &&
           unqualified != "operator delete" &&
           unqualified != "operator delete[]" &&
           unqualified != "operatornew" &&
           unqualified != "operatornew[]" &&
           unqualified != "operatordelete" &&
           unqualified != "operatordelete[]";
  }

  bool is_scalar_or_member_pointer_type(const TypePtr & type) const
  {
    TypePtr base = strip_top_level_cv(type);
    return base &&
           (base->kind == Type::TK_MEMBER_POINTER ||
            is_integral_type(base) ||
            is_floating_type(base) ||
            is_pointer_type(base) ||
            is_named_enum_type(base) ||
            (base->kind == Type::TK_FUNDAMENTAL &&
             base->fundamental == FT_NULLPTR_T));
  }

  bool lookup_pack_size(Scope & scope, const string & name, size_t & out)
  {
    for(Scope * current = &scope; current != nullptr; current = current->parent) {
      map<string, size_t>::const_iterator found = current->named_pack_sizes.find(name);
      if(found != current->named_pack_sizes.end()) {
        out = found->second;
        return true;
      }
    }
    return false;
  }

  bool parse_integer_literal_count_text(const string & text, long long & out)
  {
    const string trimmed = trim_space(text);
    if(trimmed.empty() ||
       !std::isdigit(static_cast<unsigned char>(trimmed[0]))) {
      return false;
    }
    unsigned long long value = 0;
    string ud_suffix;
    try {
      const EFundamentalType type = classify_int(trimmed, value, ud_suffix);
      if(type == FT_VOID || !ud_suffix.empty() ||
         value > static_cast<unsigned long long>(
                     std::numeric_limits<long long>::max())) {
        return false;
      }
    } catch(const logic_error &) {
      return false;
    }
    out = static_cast<long long>(value);
    return true;
  }

  bool parse_sizeof_pack_count_text(Scope & scope, const string & text, long long & out)
  {
    const string trimmed = trim_space(text);
    const string prefix = "sizeof...(";
    if(trimmed.size() <= prefix.size() ||
       trimmed.compare(0, prefix.size(), prefix) != 0 ||
       trimmed[trimmed.size() - 1] != ')') {
      return false;
    }
    const string pack_name =
        trim_space(trimmed.substr(prefix.size(), trimmed.size() - prefix.size() - 1));
    size_t pack_size = 0;
    if(pack_name.empty() || !lookup_pack_size(scope, pack_name, pack_size)) {
      return false;
    }
    out = static_cast<long long>(pack_size);
    return true;
  }

  bool lookup_integral_constant_count_text(Scope & scope,
                                           const string & text,
                                           long long & out)
  {
    const string name = trim_space(text);
    if(!is_identifier_text(name)) {
      return false;
    }
    for(Scope * current = &scope; current != nullptr; current = current->parent) {
      map<string, ValueBinding>::const_iterator found = current->values.find(name);
      if(found == current->values.end()) {
        continue;
      }
      const ValueBinding & binding = found->second;
      if(binding.dependent_template_value) {
        return false;
      }
      if(binding.has_constant_value) {
        out = binding.constant_value;
        return true;
      }
      if(binding.has_constexpr_value &&
         constant_eval::constexpr_value_to_integral(binding.constexpr_value, out)) {
        return true;
      }
      return false;
    }
    return false;
  }

  bool evaluate_integer_pack_count_text(Scope & scope,
                                        const string & text,
                                        long long & out)
  {
    return parse_integer_literal_count_text(text, out) ||
           parse_sizeof_pack_count_text(scope, text, out) ||
           lookup_integral_constant_count_text(scope, text, out);
  }

  bool has_user_declared_destructor(const ClassInfo & info) const
  {
    return callsemantic::has_user_declared_destructor(info);
  }

  bool has_nontrivial_copy_or_move_assignment(const ClassInfo & info) const
  {
    return callsemantic::has_nontrivial_copy_or_move_assignment(info);
  }

  bool is_destructible_type(const TypePtr & type)
  {
    return callsemantic::is_destructible_type(type, type_trait_callbacks());
  }

  bool is_trivially_destructible_type(const TypePtr & type) const
  {
    return callsemantic::is_trivially_destructible_type(
        type,
        type_trait_callbacks());
  }

  bool is_trivially_copy_constructible_type(const TypePtr & type) const
  {
    return callsemantic::is_trivially_copy_constructible_type(
        type,
        type_trait_callbacks());
  }

  bool is_trivially_copy_assignable_type(const TypePtr & type) const
  {
    return callsemantic::is_trivially_copy_assignable_type(
        type,
        type_trait_callbacks());
  }

  bool is_function_local_class_info(const ClassInfo & info) const
  {
    return callsemantic::is_function_local_class_info(info);
  }

  bool type_contains_function_local_class(const TypePtr & type) const
  {
    return callsemantic::type_contains_function_local_class(
        type,
        type_trait_callbacks());
  }

  bool is_trivial_special_member_binding(const FunctionBinding & binding) const
  {
    return callsemantic::is_trivial_special_member_binding(
        binding,
        type_trait_callbacks());
  }

  string lookup_text_for_type_argument(const TypePtr & type) const
  {
    debug_throw_on_partial_order_escape(type, "lookup_text_for_type_argument");
    if(!type) {
      return string();
    }
    return type_contains_function_local_class(type) ?
        reparseable_type_argument_text(type) :
        template_argument_type_text(type);
  }

  TypeSpellingParts spell_instantiation_identity_type_argument(const TypePtr & type) const
  {
    switch(type->kind) {
    case Type::TK_FUNDAMENTAL:
      return TypeSpellingParts{type_to_string(type->fundamental) + " ", ""};

    case Type::TK_NAMED:
    {
      string text = type->named_display.empty() ? type->named_key : type->named_display;
      if(should_prefer_named_key_for_instantiation_identity(type)) {
        text = type->named_key;
      } else if(text.compare(0, 19, "template-parameter ") == 0 ||
                text.compare(0, 10, "dependent ") == 0) {
        text = type->named_display;
      }
      text = collapse_reparseable_scope_operators(strip_elaborated_type_prefix(text));
      return TypeSpellingParts{text + " ", ""};
    }

    case Type::TK_CV:
    {
      TypeSpellingParts inner = spell_instantiation_identity_type_argument(type->inner);
      string qualifier;
      if(type->cv_const) {
        qualifier += " const";
      }
      if(type->cv_volatile) {
        qualifier += " volatile";
      }
      inner.before = trim_space(inner.before) + qualifier + " ";
      return inner;
    }

    case Type::TK_ATOMIC:
    {
      TypeSpellingParts inner = spell_instantiation_identity_type_argument(type->inner);
      return TypeSpellingParts{"_Atomic(" + trim_space(inner.before) + inner.after + ") ", ""};
    }

    case Type::TK_POINTER:
    {
      TypeSpellingParts inner = spell_instantiation_identity_type_argument(type->inner);
      if(!inner.after.empty()) {
        return TypeSpellingParts{
            append_parenthesized_type_spelling_prefix(inner.before, "(*"),
            ")" + inner.after};
      }
      return TypeSpellingParts{trim_space(inner.before) + " * ", ""};
    }

    case Type::TK_MEMBER_POINTER:
    {
      TypeSpellingParts inner = spell_instantiation_identity_type_argument(type->inner);
      const string owner_text =
          trim_space(spell_instantiation_identity_type_argument(type->owner).before);
      if(!inner.after.empty()) {
        return TypeSpellingParts{
            append_parenthesized_type_spelling_prefix(inner.before,
                                                      "(" + owner_text + "::*)"),
            inner.after};
      }
      return TypeSpellingParts{trim_space(inner.before) + " " + owner_text + "::* ", ""};
    }

    case Type::TK_BLOCK_POINTER:
    {
      TypeSpellingParts inner = spell_instantiation_identity_type_argument(type->inner);
      if(!inner.after.empty()) {
        return TypeSpellingParts{
            append_parenthesized_type_spelling_prefix(inner.before, "(^"),
            ")" + inner.after};
      }
      return TypeSpellingParts{trim_space(inner.before) + " ^ ", ""};
    }

    case Type::TK_LVALUE_REFERENCE:
    {
      TypeSpellingParts inner = spell_instantiation_identity_type_argument(type->inner);
      if(!inner.after.empty()) {
        return TypeSpellingParts{
            append_parenthesized_type_spelling_prefix(inner.before, "(&"),
            ")" + inner.after};
      }
      return TypeSpellingParts{trim_space(inner.before) + " & ", ""};
    }

    case Type::TK_RVALUE_REFERENCE:
    {
      TypeSpellingParts inner = spell_instantiation_identity_type_argument(type->inner);
      if(!inner.after.empty()) {
        return TypeSpellingParts{
            append_parenthesized_type_spelling_prefix(inner.before, "(&&"),
            ")" + inner.after};
      }
      return TypeSpellingParts{trim_space(inner.before) + " && ", ""};
    }

    case Type::TK_ARRAY:
    {
      TypeSpellingParts inner = spell_instantiation_identity_type_argument(type->inner);
      if(type->has_bound) {
        ostringstream out;
        out << "[" << type->bound << "]";
        inner.after += out.str();
      } else if(!type->bound_text.empty()) {
        inner.after += "[" + type->bound_text + "]";
      } else {
        inner.after += "[]";
      }
      return inner;
    }

    case Type::TK_FUNCTION:
    {
      TypeSpellingParts inner = spell_instantiation_identity_type_argument(type->inner);
      inner.before = trim_space(inner.before);
      ostringstream out;
      out << "(";
      for(size_t i = 0; i < type->params.size(); ++i) {
        if(i != 0) {
          out << ", ";
        }
        TypeSpellingParts param = spell_instantiation_identity_type_argument(type->params[i]);
        out << trim_space(param.before + param.after);
      }
      if(type->variadic) {
        if(!type->params.empty()) {
          out << ", ";
        }
        out << "...";
      } else if(type->prototype_relaxed) {
        if(!type->params.empty()) {
          out << ", ";
        }
        out << "<prototype_relaxed>";
      }
      out << ")";
      if(type->function_const) {
        out << " const";
      }
      if(type->function_volatile) {
        out << " volatile";
      }
      inner.after += out.str();
      return inner;
    }
    }

    throw logic_error("unknown type kind in spell_instantiation_identity_type_argument");
  }

  string instantiation_identity_text_for_type_argument(const TypePtr & type) const
  {
    debug_throw_on_partial_order_escape(type, "instantiation_identity_text_for_type_argument");
    if(!type) {
      return string();
    }
    const TypeSpellingParts parts = spell_instantiation_identity_type_argument(type);
    return trim_space(parts.before + parts.after);
  }

  string lookup_text_for_non_type_template_argument(const TypePtr & type,
                                                    long long value) const
  {
    TemplateArgument arg;
    arg.kind = TemplateArgument::TA_VALUE;
    arg.type = type;
    arg.value = value;
    return template_model::template_argument_text(
        arg,
        [this](const TypePtr & current_type)
        {
          return lookup_text_for_type_argument(current_type);
        });
  }

  string lookup_reparseable_text_for_type_argument(Scope & scope, const TypePtr & type) const
  {
    (void)scope;
    return reparseable_type_argument_text(type);
  }

  void canonicalize_simple_dependent_argument_texts(vector<TemplateArgument> & arguments) const
  {
    template_api::canonicalize_simple_dependent_argument_texts(
        *const_cast<Analyzer *>(this), arguments);
  }

  TypePtr canonicalize_member_typedef_type(Scope & scope, const TypePtr & type) const
  {
    if(!type) {
      return type;
    }

    const auto requires_exact_local_reparse =
        [&](const TypePtr & candidate) -> bool
        {
          TypePtr base = strip_top_level_cv(remove_reference_type(candidate));
          if(!base) {
            return false;
          }
          const string reparseable =
              trim_space(reparseable_type_argument_text(candidate));
          ClassInfo * info = class_info_for_type(base);
          return (info &&
                  (info->is_lambda_closure ||
                   (info->enclosing_scope && info->enclosing_scope->function != nullptr))) ||
                 reparseable.find("__local_") != string::npos;
        };

    const bool needs_local_reparse = requires_exact_local_reparse(type);
    if(!type_depends_on_template_parameter(type) && !needs_local_reparse) {
      return type;
    }

    TypePtr resolved;
    if(const_cast<Analyzer *>(this)->resolve_instantiated_dependent_type(scope, type, resolved) &&
       resolved &&
       !type_depends_on_template_parameter(resolved) &&
       !requires_exact_local_reparse(resolved)) {
      return resolved;
    }

    (void)scope;
    (void)needs_local_reparse;
    return type;
  }

  bool is_empty_class_info(const ClassInfo * info) const
  {
    return callsemantic::is_empty_class_info(info, type_trait_callbacks());
  }

  bool resolve_dump_callee_binding(const DumpNode & callee,
                                   FunctionBinding *& binding)
  {
    callsemantic::FunctionRegistryState state = function_registry_state();
    return callsemantic::resolve_dump_callee_binding(
        state,
        function_registry_callbacks(),
        callee,
        binding);
  }

  bool is_declval_dump_callee(Scope & scope,
                              const DumpNode & callee)
  {
    (void)scope;
    return callee.kind == CallSemKind::callee && callee.is_declval_callee;
  }

  bool evaluate_trivial_noexcept_expression(const CppAstNode & expr,
                                            bool & out) const
  {
    switch(expr.kind) {
    case CppAstKind::keyword_literal:
      if(expr.value == "true") {
        out = true;
        return true;
      }
      if(expr.value == "false") {
        out = false;
        return true;
      }
      return false;
    case CppAstKind::parenthesized_expression:
      return expr.children.size() == 1 &&
             evaluate_trivial_noexcept_expression(expr.children[0], out);
    case CppAstKind::unary_expression:
      if(expr.children.size() != 1 || !node_has_simple_type(expr, OP_LNOT)) {
        return false;
      }
      if(!evaluate_trivial_noexcept_expression(expr.children[0], out)) {
        return false;
      }
      out = !out;
      return true;
    case CppAstKind::binary_expression:
      if(expr.children.size() != 2) {
        return false;
      }
      if(node_has_simple_type(expr, OP_LAND)) {
        bool lhs = false;
        bool rhs = false;
        if(!evaluate_trivial_noexcept_expression(expr.children[0], lhs) ||
           !evaluate_trivial_noexcept_expression(expr.children[1], rhs)) {
          return false;
        }
        out = lhs && rhs;
        return true;
      }
      if(node_has_simple_type(expr, OP_LOR)) {
        bool lhs = false;
        bool rhs = false;
        if(!evaluate_trivial_noexcept_expression(expr.children[0], lhs) ||
           !evaluate_trivial_noexcept_expression(expr.children[1], rhs)) {
          return false;
        }
        out = lhs || rhs;
        return true;
      }
      return false;
    default:
      return false;
    }
  }

  void parse_explicit_function_nothrow_parse_state(
      const CppAstNode * qualifier,
      ExplicitFunctionNothrowKind & kind,
      string & expr_text)
  {
    kind = EFNK_NONE;
    expr_text.clear();

    if(!qualifier) {
      return;
    }

    const string text = trim_space(qualifier->value);
    if(text == "noexcept" || text == "throw()") {
      kind = EFNK_ALWAYS_TRUE;
      return;
    }
    if(text.compare(0, 6, "throw(") == 0 && !text.empty() && text.back() == ')') {
      kind = EFNK_ALWAYS_FALSE;
      return;
    }
    if(text.compare(0, 9, "noexcept(") == 0 && !text.empty() && text.back() == ')') {
      kind = EFNK_EXPR;
      expr_text = text.substr(9, text.size() - 10);
      return;
    }

    kind = EFNK_INVALID;
  }

  void refresh_explicit_function_nothrow_parse_state(FunctionBinding & binding)
  {
    if(binding.explicit_function_nothrow_cached_qualifier == binding.function_qualifier &&
       binding.explicit_function_nothrow_kind != EFNK_UNINITIALIZED) {
      return;
    }

    binding.explicit_function_nothrow_cached_qualifier = binding.function_qualifier;
    binding.explicit_function_nothrow_kind = EFNK_NONE;
    binding.explicit_function_nothrow_expr_text.clear();
    binding.explicit_function_nothrow_eval_cached = false;
    binding.explicit_function_nothrow_eval_value = false;
    parse_explicit_function_nothrow_parse_state(binding.function_qualifier,
                                                binding.explicit_function_nothrow_kind,
                                                binding.explicit_function_nothrow_expr_text);
  }

  bool evaluate_explicit_function_nothrow_parse_state(FunctionBinding & binding,
                                                      ExplicitFunctionNothrowKind kind,
                                                      const string & expr_text,
                                                      const CppAstNode * qualifier,
                                                      bool & out)
  {
    switch(kind) {
    case EFNK_ALWAYS_TRUE:
      out = true;
      return true;
    case EFNK_ALWAYS_FALSE:
      out = false;
      return true;
    case EFNK_EXPR:
      break;
    case EFNK_NONE:
    case EFNK_INVALID:
    case EFNK_UNINITIALIZED:
      return false;
    }

    Scope qualifier_scope = build_function_qualifier_scope(binding);
    const CppAstNode * expr =
        qualifier && !qualifier->children.empty() ?
            &qualifier->children[0] :
            nullptr;
    if(expr_text.empty() || expr == nullptr) {
      return false;
    }

    constant_eval::ConstexprValue value;
    return evaluate_constant_expression_value(qualifier_scope, *expr, value) &&
           constant_eval::constexpr_value_truthy(value, out);
  }

  bool explicit_function_nothrow_specifications_match(FunctionBinding & existing,
                                                      const CppAstNode * qualifier) override
  {
    refresh_explicit_function_nothrow_parse_state(existing);

    ExplicitFunctionNothrowKind candidate_kind = EFNK_UNINITIALIZED;
    string candidate_expr_text;
    parse_explicit_function_nothrow_parse_state(qualifier,
                                                candidate_kind,
                                                candidate_expr_text);

    const bool existing_explicit =
        existing.explicit_function_nothrow_kind != EFNK_NONE &&
        existing.explicit_function_nothrow_kind != EFNK_INVALID;
    const bool candidate_explicit =
        candidate_kind != EFNK_NONE &&
        candidate_kind != EFNK_INVALID;
    if(existing_explicit != candidate_explicit) {
      return false;
    }
    if(!existing_explicit) {
      return true;
    }

    bool existing_value = false;
    bool candidate_value = false;
    const bool existing_known =
        evaluate_explicit_function_nothrow_semantically(existing, existing_value);
    const bool candidate_known =
        evaluate_explicit_function_nothrow_parse_state(existing,
                                                       candidate_kind,
                                                       candidate_expr_text,
                                                       qualifier,
                                                       candidate_value);
    if(existing_known && candidate_known) {
      return existing_value == candidate_value;
    }

    if(existing.explicit_function_nothrow_kind != candidate_kind) {
      return false;
    }
    if(existing.explicit_function_nothrow_kind == EFNK_EXPR) {
      return existing.explicit_function_nothrow_expr_text == candidate_expr_text;
    }
    return true;
  }

  Scope build_function_qualifier_scope(FunctionBinding & binding)
  {
    Scope qualifier_scope(binding.declaration_scope, "<function-qualifier>");
    qualifier_scope.class_info = binding.owner_class;
    qualifier_scope.function = &binding;
    for(size_t i = 0; i < binding.params.size(); ++i) {
      const string binding_name = function_parameter_binding_name(binding, i);
      if(binding_name.empty()) {
        continue;
      }
      const string alias_name = function_parameter_alias_name(binding, i);
      const string output_name = !alias_name.empty() ? alias_name : binding_name;
      ValueBinding parameter(ValueBinding::VK_PARAMETER,
                             output_name,
                             binding.params[i].second);
      qualifier_scope.values[binding_name] = parameter;
      if(!alias_name.empty() && alias_name != binding_name) {
        qualifier_scope.values[alias_name] = parameter;
      }
    }
    return qualifier_scope;
  }

  bool expression_is_nothrow(Scope & scope,
                             const CppAstNode & expr,
                             bool & out)
  {
    return semantic_builtins::expression_is_nothrow(*this, scope, expr, out);
  }

  bool initializer_is_nothrow(const TypePtr & target,
                              Scope & scope,
                              const CppAstNode & initializer,
                              set<FunctionBinding *> & visiting)
  {
    return callsemantic::initializer_is_nothrow(
        target,
        scope,
        initializer,
        visiting,
        nothrow_callbacks());
  }

  bool default_initialization_is_nothrow(const TypePtr & type,
                                         Scope & scope,
                                         set<FunctionBinding *> & visiting)
  {
    return callsemantic::default_initialization_is_nothrow(
        type,
        scope,
        visiting,
        nothrow_callbacks());
  }

  bool function_binding_is_nothrow(FunctionBinding & binding,
                                   set<FunctionBinding *> & visiting)
  {
    return callsemantic::function_binding_is_nothrow(
        binding,
        visiting,
        nothrow_callbacks());
  }

  bool function_binding_is_nothrow(FunctionBinding & binding)
  {
    return callsemantic::function_binding_is_nothrow(binding, nothrow_callbacks());
  }

  bool callsem_node_can_throw(Scope & scope,
                              const DumpNode & node,
                              set<FunctionBinding *> & visiting)
  {
    return callsemantic::callsem_node_can_throw(
        scope,
        node,
        visiting,
        nothrow_callbacks());
  }

  bool evaluate_builtin_type_trait(Scope & scope,
                                   const string & name,
                                   const TypePtr & type,
                                   long long & out)
  {
    return semantic_builtins::evaluate_builtin_type_trait(*this, scope, name, type, out);
  }

  bool is_supported_builtin_type_trait_name(const string & name)
  {
    return semantic_builtins::is_supported_builtin_type_trait_name(name);
  }

  bool evaluate_builtin_binary_type_trait(Scope & scope,
                                          const string & name,
                                          const TypePtr & lhs,
                                          const TypePtr & rhs,
                                          long long & out)
  {
    return semantic_builtins::evaluate_builtin_binary_type_trait(
        *this, scope, name, lhs, rhs, out);
  }

  bool try_parse_builtin_type_trait_call_arg(Scope & scope,
                                             const CppAstNode & arg,
                                             TypePtr & type)
  {
    return semantic_builtins::try_parse_builtin_type_trait_call_arg(*this, scope, arg, type);
  }

  bool try_parse_builtin_type_trait_call(Scope & scope,
                                         const CppAstNode & node,
                                         string & trait_name,
                                         vector<TypePtr> & types)
  {
    return semantic_builtins::try_parse_builtin_type_trait_call(
        *this, scope, node, trait_name, types);
  }

  bool try_parse_builtin_type_trait_text(Scope & scope,
                                         const string & text,
                                         string & trait_name,
                                         vector<TypePtr> & types)
  {
    return semantic_builtins::try_parse_builtin_type_trait_text(
        *this, scope, text, trait_name, types);
  }

  TypePtr direct_named_type(Scope & scope, const string & name)
  {
    map<string, TypePtr>::iterator found = scope.named_types.find(name);
    if(found == scope.named_types.end()) {
      return TypePtr();
    }
    if(scope.class_info &&
       template_api::class_has_non_dependent_source_template_identity(scope.class_info) &&
       found->second &&
       type_depends_on_template_parameter(found->second)) {
      TypePtr resolved;
      if(resolve_instantiated_dependent_type(scope, found->second, resolved) &&
         resolved &&
         !type_depends_on_template_parameter(resolved)) {
        return resolved;
      }
    }
    return found->second;
  }

  TypePtr lookup_enclosing_type_before_reference_placeholder(
      const Scope & class_scope,
      const string & lookup_name) const
  {
    set<const Scope *> visited;
    function<TypePtr(const Scope &)> lookup_scope_and_inline_children =
        [&](const Scope & current_scope) -> TypePtr
        {
          if(!visited.insert(&current_scope).second) {
            return TypePtr();
          }
          map<string, TypePtr>::const_iterator direct =
              current_scope.named_types.find(lookup_name);
          if(direct != current_scope.named_types.end()) {
            return direct->second;
          }
          for(size_t i = 0; i < current_scope.namespace_children.size(); ++i) {
            const Scope & child = *current_scope.namespace_children[i];
            if(!child.inline_namespace && child.name != "<unnamed>") {
              continue;
            }
            TypePtr nested = lookup_scope_and_inline_children(child);
            if(nested) {
              return nested;
            }
          }
          for(size_t i = 0; i < current_scope.using_directives.size(); ++i) {
            TypePtr imported =
                lookup_scope_and_inline_children(*current_scope.using_directives[i]);
            if(imported) {
              return imported;
            }
          }
          return TypePtr();
        };

    for(const Scope * current = class_scope.parent; current; current = current->parent) {
      TypePtr found = lookup_scope_and_inline_children(*current);
      if(found) {
        return found;
      }
    }
    return TypePtr();
  }

  ClassInfo * class_info_for_type(const TypePtr & type) const
  {
    if(collect_metrics_) {
      ++metrics_.class_info_for_type_calls;
    }
    TypePtr base = strip_top_level_cv(type);
    if(!base || base->kind != Type::TK_NAMED) {
      return nullptr;
    }
    callsemantic::TypeRegistryState state =
        const_cast<Analyzer *>(this)->type_registry_state();
    if(class_info_for_type_cache_classes_by_key_version_ !=
       state.classes_by_key_version) {
      class_info_for_type_cache_.clear();
      class_info_for_type_cache_classes_by_key_version_ =
          state.classes_by_key_version;
    }
    const Type * cache_key = base.get();
    std::map<const Type *, ClassInfo *>::const_iterator cached =
        class_info_for_type_cache_.find(cache_key);
    if(cached != class_info_for_type_cache_.end()) {
      if(collect_metrics_) {
        ++metrics_.class_info_for_type_pointer_cache_hits;
      }
      return cached->second;
    }
    std::unordered_map<std::string, std::size_t>::const_iterator epoch_found =
        state.classes_by_key_epochs.find(base->named_key);
    const bool key_present = epoch_found != state.classes_by_key_epochs.end();
    const std::size_t key_epoch = key_present ? epoch_found->second : 0;
    std::unordered_map<std::string, ClassInfoForTypeNamedKeyCacheEntry>::const_iterator
        named_key_cached = class_info_for_type_named_key_cache_.find(base->named_key);
    if(named_key_cached != class_info_for_type_named_key_cache_.end()) {
      const ClassInfoForTypeNamedKeyCacheEntry & entry = named_key_cached->second;
      if(entry.key_present == key_present && entry.epoch == key_epoch) {
        if(collect_metrics_) {
          ++metrics_.class_info_for_type_named_key_cache_hits;
        }
        class_info_for_type_cache_[cache_key] = entry.info;
        return entry.info;
      }
    }
    if(collect_metrics_) {
      ++metrics_.class_info_for_type_map_lookups;
    }
    std::map<std::string, ClassInfo *>::const_iterator found =
        state.classes_by_key.find(base->named_key);
    ClassInfo * result =
        found == state.classes_by_key.end() ? nullptr : found->second;
    if(collect_metrics_) {
      if(result) {
        ++metrics_.class_info_for_type_map_hits;
      } else {
        ++metrics_.class_info_for_type_map_misses;
      }
    }
    ClassInfoForTypeNamedKeyCacheEntry entry;
    entry.info = result;
    entry.epoch = key_epoch;
    entry.key_present = key_present;
    if(!result || key_present) {
      class_info_for_type_named_key_cache_[base->named_key] = entry;
    }
    class_info_for_type_cache_[cache_key] = result;
    return result;
  }

  const map<string, ClassInfo *> & template_named_class_index() const
  {
    callsemantic::TypeRegistryState state =
        const_cast<Analyzer *>(this)->type_registry_state();
    return callsemantic::template_named_class_index(state);
  }

  Scope * scope_for_type(const TypePtr & type) const
  {
    callsemantic::TypeRegistryState state =
        const_cast<Analyzer *>(this)->type_registry_state();
    return callsemantic::scope_for_type(state, type);
  }

  bool is_initializer_list_type(const TypePtr & type,
                                TypePtr * element_type = nullptr,
                                ClassInfo ** info_out = nullptr) const
  {
    callsemantic::TypeRegistryState state =
        const_cast<Analyzer *>(this)->type_registry_state();
    return callsemantic::is_initializer_list_type(state,
                                                  type,
                                                  element_type,
                                                  info_out);
  }

  ClassInfo * create_class_info(Scope & scope,
                                const string & class_kind,
                                const string & name,
                                const CppAstNode * class_node = nullptr)
  {
    callsemantic::TypeRegistryState state = type_registry_state();
    return callsemantic::create_class_info(state,
                                           type_registry_callbacks(),
                                           scope,
                                           class_kind,
                                           name,
                                           class_node);
  }

  bool resolve_declared_class_scope_and_name(Scope & scope,
                                             const string & text,
                                             Scope *& target_scope,
                                             string & class_name)
  {
    target_scope = &scope;
    class_name = text;

    QualifiedName qualified;
    if(!split_qualified_name_text(text, qualified) ||
       (!qualified.rooted && qualified.qualifiers.empty())) {
      return true;
    }

    target_scope = semantic_lookup::resolve_qualified_scope_for_class_or_namespace(
        *this, scope, qualified, true);
    if(!target_scope) {
      return false;
    }
    class_name = qualified.name;
    return true;
  }

  ClassInfo * lookup_declared_class_info(Scope & scope, const string & text)
  {
    Scope * target_scope = nullptr;
    string class_name;
    if(!resolve_declared_class_scope_and_name(scope, text, target_scope, class_name)) {
      return nullptr;
    }
    return class_info_for_type(direct_named_type(*target_scope, class_name));
  }

  TypePtr lookup_non_template_type_name(Scope & scope, const string & text)
  {
    const string normalized_name = normalize_type_lookup_name(text);
    if(normalized_name.empty() ||
       normalized_name.find('<') != string::npos ||
       has_invalid_top_level_qualified_owner_syntax(normalized_name)) {
      return TypePtr();
    }

    const auto lookup_unqualified_type = [&]() -> TypePtr
    {
      return lookup_unqualified_generic<TypePtr>(
          scope,
          normalized_name,
          [this, &scope](Scope & target, const string & lookup_name) -> TypePtr
          {
            const auto direct_lookup = [this, &lookup_name, &scope](Scope & direct_scope) -> TypePtr
            {
              TypePtr direct_named = direct_named_type(direct_scope, lookup_name);
              if(direct_named) {
                return direct_named;
              }
              if(direct_scope.class_info &&
                 direct_scope.class_info->reference_member_collection_in_progress) {
                return make_named(lookup_name,
                                  "dependent alias " +
                                      direct_scope.class_info->qualified_name +
                                      "::" + lookup_name,
                                  true);
              }
              if(direct_scope.class_info) {
                const bool ensure_current_member_references =
                    !(((direct_scope.class_info->full_member_collection_in_progress ||
                        direct_scope.class_info->reference_member_collection_in_progress) &&
                       direct_scope.class_info->member_scope &&
                       scope_is_within(direct_scope,
                                       direct_scope.class_info->member_scope.get())));
                MemberTypeLookupResult member =
                    lookup_member_type(*this,
                                       *direct_scope.class_info,
                                       lookup_name,
                                       ensure_current_member_references,
                                       &scope);
                if(member.type) {
                  return member.type;
                }
              }
              const bool has_lexical_class =
                  !direct_scope.class_info &&
                  direct_scope.function &&
                  direct_scope.function->lexical_access_class;
              if(has_lexical_class) {
                MemberTypeLookupResult member =
                    lookup_member_type(*this,
                                       *direct_scope.function->lexical_access_class,
                                       lookup_name,
                                       true,
                                       &scope);
                if(member.type) {
                  return member.type;
                }
              }
              return TypePtr();
            };

            function<TypePtr(Scope &)> lookup_inline_children =
                [&](Scope & current_scope) -> TypePtr
                {
                  for(size_t i = 0; i < current_scope.namespace_children.size(); ++i) {
                    Scope & child = *current_scope.namespace_children[i];
                    if(!child.inline_namespace && child.name != "<unnamed>") {
                      continue;
                    }
                    TypePtr direct = direct_lookup(child);
                    if(direct) {
                      return direct;
                    }
                    TypePtr nested = lookup_inline_children(child);
                    if(nested) {
                      return nested;
                    }
                  }
                  return TypePtr();
                };

            TypePtr direct = direct_lookup(target);
            if(direct) {
              return direct;
            }
            return lookup_inline_children(target);
          });
    };

    if(is_identifier_text(normalized_name)) {
      return lookup_unqualified_type();
    }

    QualifiedName qualified;
    if(!split_qualified_name_text(normalized_name, qualified) ||
       (!qualified.rooted && qualified.qualifiers.empty())) {
      return TypePtr();
    }

    Scope * current =
        semantic_lookup::resolve_qualified_scope_for_class_or_namespace(
            *this,
            scope,
            qualified,
            false);
    if(!current) {
      return TypePtr();
    }

    TypePtr direct_qualified =
        semantic_lookup::resolve_direct_type_qualifier(*this,
                                                       *current,
                                                       scope,
                                                       qualified.name);
    if(direct_qualified) {
      return direct_qualified;
    }

    if(!current->class_info) {
      return TypePtr();
    }

    const bool ensure_current_member_references =
        !(((current->class_info->full_member_collection_in_progress ||
            current->class_info->reference_member_collection_in_progress) &&
           current->class_info->member_scope &&
           scope_is_within(*current, current->class_info->member_scope.get())));
    MemberTypeLookupResult member =
        lookup_member_type(*this,
                           *current->class_info,
                           qualified.name,
                           ensure_current_member_references,
                           &scope);
    return member.type;
  }

  TypePtr maybe_introduce_elaborated_class_type(Scope & scope, const string & text)
  {
    string class_kind;
    string declared_name;
    if(!parse_elaborated_class_lookup_name(text, class_kind, declared_name) ||
       declared_name.find('<') != string::npos) {
      return TypePtr();
    }

    Scope * target_scope = nullptr;
    string class_name;
    if(!resolve_declared_class_scope_and_name(scope, declared_name, target_scope, class_name) ||
       !target_scope) {
      return TypePtr();
    }

    return create_class_info(*target_scope, class_kind, class_name)->type;
  }

  ClassInfo * create_instantiated_class_info(
      const ClassTemplateInfoCreationRequest & request)
  {
    if(!request.scope) {
      throw logic_error("missing class template instantiation scope");
    }
    callsemantic::TypeRegistryState state = type_registry_state();
    return callsemantic::create_instantiated_class_info_with_internal_name(
        state,
        type_registry_callbacks(),
        *request.scope,
        request.class_kind,
        request.template_name,
        request.specialization_name,
        request.internal_specialization_name.empty() ?
            request.specialization_name :
            request.internal_specialization_name,
        request.template_decl,
        request.output_node,
        request.track_output);
  }

  void reset_instantiated_class_info(ClassInfo & info,
                                     const string & template_name,
                                     const CppAstNode * output_node)
  {
    for(size_t i = 0; i < functions.size(); ++i) {
      FunctionBinding * binding = functions[i].get();
      if(!binding ||
         (!binding->output_emitted && !binding->definition_output_emitted)) {
        continue;
      }
      if(binding->owner_class == &info ||
         (info.member_scope &&
          binding->declaration_scope == info.member_scope.get())) {
        return;
      }
    }
    discard_class_function_bindings(info);
    semantic_class_model::reset_instantiated_class_info(info, template_name, output_node);
  }

  bool is_builtin_initializer_list_template(ClassTemplateDecl & decl) const
  {
    return decl.name == "initializer_list" &&
           decl.declaring_scope &&
           scope_qualified_name(*decl.declaring_scope, decl.name) == "std::initializer_list";
  }

  virtual const CppAstNode * own_synthetic_ast(CppAstNode node)
  {
    synthetic_ast_nodes.push_back(unique_ptr<CppAstNode>(new CppAstNode(std::move(node))));
    return synthetic_ast_nodes.back().get();
  }

  bool is_persistent_scope(const Scope * scope) const
  {
    if(scope == nullptr) {
      return true;
    }
    if(scope == root.get() || scope->namespace_scope) {
      return true;
    }
    if(scope->class_info && scope->class_info->member_scope.get() == scope) {
      return true;
    }
    for(size_t i = 0; i < template_scopes.size(); ++i) {
      if(template_scopes[i].get() == scope) {
        return true;
      }
    }
    for(size_t i = 0; i < captured_local_scopes.size(); ++i) {
      if(captured_local_scopes[i].get() == scope) {
        return true;
      }
    }
    for(size_t i = 0; i < durable_type_scopes.size(); ++i) {
      if(durable_type_scopes[i].get() == scope) {
        return true;
      }
    }
    return false;
  }

  Scope & persistent_enclosing_scope(Scope & scope)
  {
    if(is_persistent_scope(&scope)) {
      return scope;
    }

    const size_t cache_key = scope_cache_key(scope);
    unordered_map<size_t, Scope *>::const_iterator cached =
        cache_state_.captured_local_scope_cache.find(cache_key);
    if(cached != cache_state_.captured_local_scope_cache.end()) {
      return *cached->second;
    }

    Scope * parent =
        scope.parent ? &persistent_enclosing_scope(*scope.parent) : nullptr;
    captured_local_scopes.emplace_back(new Scope(parent, scope.name, scope.namespace_scope));
    Scope & captured = *captured_local_scopes.back();
    cache_state_.captured_local_scope_cache[cache_key] = &captured;
    captured.inline_namespace = scope.inline_namespace;
    captured.class_info = scope.class_info;
    captured.function = scope.function;
    captured.named_types = scope.named_types;
    captured.named_type_packs = scope.named_type_packs;
    captured.named_value_packs = scope.named_value_packs;
    captured.named_pack_sizes = scope.named_pack_sizes;
    captured.template_bound_type_names = scope.template_bound_type_names;
    captured.template_bound_type_pack_names = scope.template_bound_type_pack_names;
    captured.template_bound_value_names = scope.template_bound_value_names;
    captured.template_bound_template_names = scope.template_bound_template_names;
    captured.values = scope.values;
    captured.namespace_bindings = scope.namespace_bindings;
    captured.function_sets = scope.function_sets;
    captured.class_templates = scope.class_templates;
    captured.function_templates = scope.function_templates;
    captured.alias_templates = scope.alias_templates;
    captured.variable_templates = scope.variable_templates;
    captured.using_directives = scope.using_directives;
    return captured;
  }

  static bool scope_is_within(const Scope & scope, const Scope * ancestor)
  {
    for(const Scope * current = &scope; current; current = current->parent) {
      if(current == ancestor) {
        return true;
      }
    }
    return false;
  }

  callsemantic::ClassTemplateReferenceCallbacks class_template_reference_callbacks()
  {
    callsemantic::ClassTemplateReferenceCallbacks callbacks;
    callbacks.source_location_for_token_index =
        [this](std::size_t index) {
          return source_location_for_token_index(index);
        };
    callbacks.token_index_for_source_location =
        [this](const std::string & location,
               const std::string & token_source,
               std::size_t & out_index) {
          return token_index_for_source_location(location, token_source, out_index);
        };
    callbacks.find_next_token_source_on_same_line =
        [this](std::size_t start_index,
               const std::string & token_source,
               std::size_t & out_index) {
          return find_next_token_source_on_same_line(start_index, token_source, out_index);
        };
    callbacks.template_argument_token_ranges_from_open =
        [this](std::size_t open_index,
               std::vector<std::pair<std::size_t, std::size_t> > & ranges) {
          return template_argument_token_ranges_from_open(open_index, ranges);
        };
    callbacks.peek_token =
        [this](std::size_t index) -> const RecogToken & {
          return token_sequence->peek(index);
        };
    callbacks.record_selected_class_template_base_source_uses =
        [this](ClassTemplateDecl & decl,
               const template_api::ClassSpecializationSelection & specialization) {
          record_selected_class_template_base_source_uses(decl, specialization);
        };
    callbacks.emit_nested_class_use_source_events_from_location =
        [this](Scope & scope,
               const std::string & location,
               witness::SourceUseOwnership ownership,
               const std::string & skip_exact_template_name) {
          emit_nested_class_use_source_events_from_location(scope,
                                                            location,
                                                            ownership,
                                                            skip_exact_template_name);
        };
    callbacks.emit_nested_class_use_source_events_from_syntaxes =
        [this](Scope & scope,
               const std::vector<TemplateArgumentSyntax> & syntaxes,
               witness::SourceUseOwnership ownership,
               const std::string & skip_exact_template_name) {
          emit_nested_class_use_source_events_from_syntaxes(scope,
                                                            syntaxes,
                                                            ownership,
                                                            skip_exact_template_name);
        };
    return callbacks;
  }

  std::vector<std::string> class_template_argument_source_locations_for_current_use(
      const std::string & template_name,
      const std::vector<TemplateParameterInfo> & parameters,
      const std::vector<std::string> & arg_texts)
  {
    const callsemantic::ClassTemplateReferenceCallbacks callbacks =
        class_template_reference_callbacks();
    return callsemantic::class_template_argument_source_locations_for_current_use(
        *this, callbacks, template_name, parameters, arg_texts);
  }

  ClassInfo * reference_class_template_instantiation(ClassTemplateDecl & decl,
                                                     Scope & use_scope,
                                                     const vector<string> & arg_texts)
  {
    const callsemantic::ClassTemplateReferenceCallbacks callbacks =
        class_template_reference_callbacks();
    return callsemantic::reference_class_template_instantiation(
        *this, callbacks, decl, use_scope, arg_texts);
  }

  ClassInfo * reference_class_template_instantiation_with_syntax(
      ClassTemplateDecl & decl,
      Scope & use_scope,
      const vector<string> & arg_texts,
      const vector<TemplateArgumentSyntax> * arg_syntaxes,
      template_api::ClassTemplateSourceUseMode source_use_mode =
          template_api::ClassTemplateSourceUseMode::EmitClassUse)
  {
    const callsemantic::ClassTemplateReferenceCallbacks callbacks =
        class_template_reference_callbacks();
    return callsemantic::reference_class_template_instantiation_with_syntax(
        *this,
        callbacks,
        decl,
        use_scope,
        arg_texts,
        arg_syntaxes,
        source_use_mode);
  }

    ClassInfo * reference_selected_class_template_instantiation(
        ClassTemplateDecl & decl,
        Scope & use_scope,
        const vector<TemplateArgument> & arguments,
        const template_api::ClassSpecializationSelection & specialization)
  {
    const callsemantic::ClassTemplateReferenceCallbacks callbacks =
        class_template_reference_callbacks();
    return callsemantic::reference_selected_class_template_instantiation(
        *this,
        callbacks,
        decl,
        use_scope,
        arguments,
        specialization,
        nullptr,
        template_api::ClassTemplateSourceUseMode::EmitClassUse,
        nullptr);
    }

    void record_class_template_base_source_uses(const CppAstNode * class_node,
                                                Scope * pattern_scope) override
    {
      if(!template_api::semantic_source_use_capture_enabled() ||
         !class_node ||
         !pattern_scope) {
        return;
      }
      if(class_template_base_source_use_recorded_.count(class_node) != 0 ||
         class_template_base_source_use_active_.count(class_node) != 0) {
        return;
      }
      const CppAstNode * clause = find_child(*class_node,
                                             CppAstKind::base_clause);
      if(!clause) {
        return;
      }
      class_template_base_source_use_active_.insert(class_node);
      struct ActiveGuard
      {
        ActiveGuard(std::set<const CppAstNode *> & active,
                    const CppAstNode * class_node)
          : active(active), class_node(class_node) {}
        ~ActiveGuard() { active.erase(class_node); }
        std::set<const CppAstNode *> & active;
        const CppAstNode * class_node;
      } active_guard(class_template_base_source_use_active_, class_node);
      class_template_base_source_use_recorded_.insert(class_node);

      for(size_t i = 0; i < clause->children.size(); ++i) {
        const CppAstNode & specifier = clause->children[i];
        const CppAstNode * base_name = find_child(specifier,
                                                  CppAstKind::base_name);
        if(!base_name) {
          continue;
        }
        const TemplateIdSyntax * base_template_syntax =
            cppast_template_id_syntax(*base_name);
        if(!base_template_syntax ||
           base_template_syntax->name.name.empty() ||
           (!lookup_alias_template(*pattern_scope,
                                   base_template_syntax->name) &&
            !lookup_class_template(*pattern_scope,
                                   base_template_syntax->name))) {
          continue;
        }
        const string base_lookup_text =
            template_id_syntax_text_preserving_spacing(*base_template_syntax);
        const bool base_is_alias =
            lookup_alias_template(*pattern_scope,
                                  base_template_syntax->name) != nullptr;

        const string base_identifier =
            unqualified_member_name(base_template_syntax->name.name);
        const std::vector<WitnessUseLocationSearch> base_location_searches = {
          WitnessUseLocationSearch(WITNESS_USE_LOCATION_NAME_IN_SUBTREE,
                                   &specifier),
          WitnessUseLocationSearch(WITNESS_USE_LOCATION_NAME_IN_NODE,
                                   class_node),
          WitnessUseLocationSearch(WITNESS_USE_LOCATION_NODE_START,
                                   base_name)
        };
        const string base_location =
            select_witness_use_location(base_location_searches,
                                        base_identifier);

        try {
          if(base_is_alias) {
            AliasTemplateDecl * alias_template =
                lookup_alias_template(*pattern_scope,
                                      base_template_syntax->name);
            vector<string> source_arg_texts =
                template_id_argument_witness_source_texts(
                    *base_template_syntax);
            vector<TemplateArgument> alias_arguments;
            if(alias_template &&
               witness::source_capture_enabled(template_witness_context()) &&
               resolve_template_arguments(*pattern_scope,
                                          alias_template->parameters,
                                          source_arg_texts,
                                          &base_template_syntax->argument_syntaxes,
                                          alias_arguments,
                                          alias_template->declaring_scope)) {
              vector<string> witness_source_arg_texts = source_arg_texts;
              template_argument_semantics::
                  canonicalize_alias_template_source_argument_texts(
                      alias_template->parameters,
                      witness_source_arg_texts);
              witness::AliasUseEmitRequest request;
              request.use_location = base_location;
              request.template_id_occurrence =
                  witness::make_source_template_id_occurrence(base_location,
                                                              witness_source_arg_texts);
              fill_template_id_source_occurrence_argument_facts(
                  *pattern_scope,
                  witness_source_arg_texts,
                  &base_template_syntax->argument_syntaxes,
                  alias_arguments,
                  request.template_id_occurrence);
              template_api::with_template_services(
                  *this,
                  [&](template_api::TemplateServices & services)
                  {
                    template_argument_semantics::
                        mark_alias_template_value_owner_argument_facts(
                            services,
                            pattern_scope,
                            *alias_template,
                            alias_arguments,
                            &base_template_syntax->argument_syntaxes,
                            request.template_id_occurrence);
                  });
              request.template_name =
                  template_api::alias_template_witness_entity(alias_template);
              const semantic_model::SourceDeclAnchorCache & decl_anchor =
                  semantic_trace::alias_template_decl_anchor(*this,
                                                             alias_template);
              witness::set_selected_decl_anchor(request.selected_decl_location,
                                                request.selected_decl_anchor,
                                                decl_anchor);
              request.selected_decl_anchor_explicit = true;
              request.origin = witness::AliasUseEmissionOrigin::BaseClauseTemplateId;
              template_api::append_template_witness_source_bindings(
                  *this,
                  request.bindings,
                  alias_template->parameters,
                  alias_arguments,
                  witness_source_arg_texts,
                  "explicit",
                  "defaulted",
                  false);
              rewrite_current_specialization_alias_binding_texts(
                  *this,
                  *pattern_scope,
                  alias_template->parameters,
                  alias_arguments,
                  witness_source_arg_texts,
                  &base_template_syntax->argument_syntaxes,
                  request.bindings,
                  &request.template_id_occurrence);
              witness::emit_alias_use(template_witness_context(), request);
            }
            emit_nested_alias_use_source_events_from_location(
                *pattern_scope,
                base_location,
                alias_template ?
                    template_api::alias_template_witness_entity(alias_template) :
                    base_template_syntax->name.name,
                &base_template_syntax->argument_syntaxes);
            emit_nested_class_use_source_events_from_syntaxes(
                *pattern_scope,
                base_template_syntax->argument_syntaxes,
                witness::SourceUseOwnership::SourceOwned,
                alias_template ?
                    template_api::alias_template_witness_entity(alias_template) :
                    base_template_syntax->name.name);
          } else {
            witness::ClassUseSourceDecision decision;
            if(build_class_use_source_decision_from_template_text(*pattern_scope,
                                                                  base_lookup_text,
                                                                  base_location,
                                                                  decision,
                                                                  base_template_syntax)) {
              witness::emit_class_use_decision(
                  decision,
                  witness::SourceUseOwnership::SourceOwned,
                  witness::SourceUseRole::TypeUse,
                  witness::ClassUseEmissionOrigin::NestedSourceTemplateId);
              emit_nested_class_use_source_events_from_syntaxes(
                  *pattern_scope,
                  base_template_syntax->argument_syntaxes,
                  witness::SourceUseOwnership::SourceOwned,
                  decision.template_name);
            }
          }
        } catch(const semantic_fallback_audit::StrictSemanticFallbackError &) {
          throw;
        } catch(const std::exception &) {
          // Source witness recovery must not make declaration collection fatal.
          continue;
        }
      }
    }

    void record_primary_alias_base_source_uses(ClassTemplateDecl & decl) override
    {
      record_class_template_base_source_uses(decl.class_node,
                                             decl.pattern_scope);
    }

    void record_selected_class_template_base_source_uses(
        ClassTemplateDecl & decl,
        const template_api::ClassSpecializationSelection & specialization)
    {
      const CppAstNode * class_node =
          specialization.class_node ? specialization.class_node : decl.class_node;
      Scope * pattern_scope =
          specialization.binding_scope ? specialization.binding_scope :
                                         decl.pattern_scope;
      record_class_template_base_source_uses(class_node, pattern_scope);
    }


    ClassInfo * reference_selected_class_template_instantiation(
        ClassTemplateDecl & decl,
        Scope & use_scope,
        const vector<TemplateArgument> & arguments,
        const template_api::ClassSpecializationSelection & specialization,
        const vector<string> * source_arg_texts = nullptr,
        template_api::ClassTemplateSourceUseMode source_use_mode =
            template_api::ClassTemplateSourceUseMode::EmitClassUse,
        const vector<TemplateArgumentSyntax> * source_arg_syntaxes = nullptr)
  {
    const callsemantic::ClassTemplateReferenceCallbacks callbacks =
        class_template_reference_callbacks();
    return callsemantic::reference_selected_class_template_instantiation(
        *this,
        callbacks,
        decl,
        use_scope,
        arguments,
        specialization,
        source_arg_texts,
        source_use_mode,
        source_arg_syntaxes);
  }

    ClassInfo * reference_selected_class_template_instantiation_with_key(
        ClassTemplateDecl & decl,
        Scope & use_scope,
        const vector<TemplateArgument> & arguments,
        const template_api::ClassSpecializationSelection & specialization,
        const vector<string> * source_arg_texts,
        const string * precomputed_key,
        template_api::ClassTemplateSourceUseMode source_use_mode =
            template_api::ClassTemplateSourceUseMode::EmitClassUse)
  {
    const callsemantic::ClassTemplateReferenceCallbacks callbacks =
        class_template_reference_callbacks();
    return callsemantic::reference_selected_class_template_instantiation_with_key(
        *this,
        callbacks,
        decl,
        use_scope,
        arguments,
        specialization,
        source_arg_texts,
        precomputed_key,
        source_use_mode);
  }

  bool class_member_specifiers_supported(const CppAstNode & specifiers,
                                         bool allow_inline_virtual) const
  {
    return semantic_class_model::class_member_specifiers_supported(specifiers,
                                                                   allow_inline_virtual);
  }

  bool declarator_is_const_method(const CppAstNode & declarator) const
  {
    return semantic_class_model::declarator_is_const_method(declarator);
  }

  bool declarator_is_volatile_method(const CppAstNode & declarator) const
  {
    return semantic_class_model::declarator_is_volatile_method(declarator);
  }

  const CppAstNode * declarator_function_qualifier(const CppAstNode & declarator) const
  {
    return semantic_class_model::declarator_function_qualifier(declarator);
  }

  TypePtr lookup_type_impl(Scope & scope,
                           const string & name,
                           bool reference_class_templates_only,
                           bool allow_dependent_class_qualifiers,
                           template_api::ClassTemplateSourceUseMode source_use_mode =
                               template_api::ClassTemplateSourceUseMode::EmitClassUse)
  {
    DIAG_CONTEXT("lookup_type [" + name + "]");
    const string trimmed_name = trim_space(name);
    const string normalized_name = normalize_type_lookup_name(name);
    const std::string entry_use_location = parser_trace::current_use_location();
    if(parser_trace::enabled("template.resolve") &&
       (normalized_name.find("iterator_traits<_InputIter>") != string::npos ||
        normalized_name == "_InputIter" ||
        normalized_name == "__node_pointer" ||
        normalized_name == "__end_node_pointer" ||
        normalized_name.find("__rebind_pointer_t") != string::npos)) {
      std::ostringstream trace;
      trace << "lookup-type-enter name=" << name
            << " normalized=" << normalized_name
            << " scope=" << scope_qualified_name(scope, "<here>")
            << " bindings=" << describe_scope_bindings_for_diagnostic(scope)
            << " ref-only=" << (reference_class_templates_only ? "yes" : "no")
            << " allow-dependent="
            << (allow_dependent_class_qualifiers ? "yes" : "no");
      parser_trace::note("template.resolve", DiagnosticContext::current_frame(), trace.str());
    }
    if(has_invalid_top_level_qualified_owner_syntax(normalized_name)) {
      return TypePtr();
    }
    const bool common_member_alias_lookup =
        normalized_name == "value_type" ||
        normalized_name == "allocator_type" ||
        normalized_name == "size_type" ||
        normalized_name == "difference_type" ||
        normalized_name == "pointer" ||
        normalized_name == "const_pointer" ||
        normalized_name == "iterator" ||
        normalized_name == "const_iterator" ||
        normalized_name == "reference" ||
        normalized_name == "const_reference";
    if(common_member_alias_lookup &&
       normalized_name.find("::") == string::npos &&
       normalized_name.find('<') == string::npos) {
      for(Scope * current = &scope; current; current = current->parent) {
        if(current->class_info &&
           current->class_info->reference_member_collection_in_progress) {
          TypePtr direct_alias = direct_named_type(*current, normalized_name);
          if(direct_alias) {
            return direct_alias;
          }
          return make_named(normalized_name,
                            "dependent alias " +
                                current->class_info->qualified_name +
                                "::" + normalized_name,
                            true);
        }
      }
    }
    const bool cacheable_qualified_lookup =
        normalized_name.find("::") != string::npos;
    const size_t qualified_lookup_scope_key =
        cacheable_qualified_lookup ? scope_cache_key(scope) : 0;
    const semantic_cache::QualifiedTypeLookupKey qualified_lookup_key =
        cacheable_qualified_lookup ?
            qualified_type_lookup_cache_key(qualified_lookup_scope_key,
                                            normalized_name,
                                            reference_class_templates_only,
                                            allow_dependent_class_qualifiers) :
            semantic_cache::QualifiedTypeLookupKey();
    if(cacheable_qualified_lookup) {
      unordered_map<semantic_cache::QualifiedTypeLookupKey,
                    TypePtr,
                    semantic_cache::QualifiedTypeLookupKeyHash>::const_iterator found =
          cache_state_.qualified_type_lookup_cache.find(qualified_lookup_key);
      if(found != cache_state_.qualified_type_lookup_cache.end()) {
        metrics_.note_cache_hit(semantic_metrics::CK_QUALIFIED_TYPE_LOOKUP);
        if(witness::source_capture_enabled(template_witness_context())) {
          const std::string use_location =
              !entry_use_location.empty() ? entry_use_location :
                                            parser_trace::current_use_location();
          const std::string qualified_alias_identifier =
              unqualified_member_name(
                  strip_trailing_top_level_template_arguments(normalized_name));
          const std::string qualified_alias_start_location =
              source_location_for_qualified_member_start_on_line(
                  use_location,
                  qualified_alias_identifier);
          if(!qualified_alias_start_location.empty()) {
            record_deduced_class_use_for_resolved_alias_type(
                scope,
                found->second,
                qualified_alias_start_location);
          }
        }
        return found->second;
      }
    }
    if(cacheable_qualified_lookup) {
      metrics_.note_cache_miss(semantic_metrics::CK_QUALIFIED_TYPE_LOOKUP);
    }
    const auto cache_qualified_type_result = [&](const TypePtr & result) -> TypePtr
    {
      if(cacheable_qualified_lookup && result) {
        cache_state_.qualified_type_lookup_cache[qualified_lookup_key] = result;
      }
      return result;
    };
    const bool mentions_template_placeholders =
        text_mentions_template_placeholders(scope, normalized_name);
    const bool mentions_non_namespace_bindings =
        text_mentions_non_namespace_binding_names(scope, normalized_name);
    const bool mentions_current_specialization_names =
        text_mentions_current_specialization_names(scope, normalized_name);
    if(mentions_current_specialization_names) {
      TypePtr current_specialization =
          current_specialization_type_for_lookup_text(scope, normalized_name);
      if(current_specialization) {
        return current_specialization;
      }
    }
    const auto note_resolved_alias_class_use =
        [&](const TypePtr & type,
            const vector<string> * explicit_arg_texts) -> void
    {
      if(mentions_template_placeholders ||
         !witness::source_capture_enabled(template_witness_context())) {
        return;
      }
      const std::string use_location =
          !entry_use_location.empty() ? entry_use_location :
                                        parser_trace::current_use_location();
      if(use_location.empty()) {
        return;
      }
      ClassInfo * info =
          class_info_for_type(strip_top_level_cv(remove_reference_type(type)));
      template_api::ClassTemplateUseInfo use;
      if(!template_api::class_template_use_info_for_class(
             *this, scope, info, use)) {
        return;
      }
      if(use.dependent_arguments) {
        return;
      }

      ClassTemplateDecl * class_template = use.origin;
      const std::string qualified_template_name = use.template_name;
      const std::string source_template_name = use.unqualified_name;
      const std::string anchor_identifier =
          !source_template_name.empty() ?
              source_template_name :
              qualified_template_name;
      std::string exact_use_location;
      if(const ExactTemplateTypeLookupAnchor * anchor =
             current_exact_template_type_lookup_anchor()) {
        if(exact_template_type_lookup_anchor_matches(*anchor, normalized_name) ||
           exact_template_type_lookup_anchor_matches_identifier(*anchor,
                                                                anchor_identifier)) {
          exact_use_location = anchor->location;
        }
      }
      if(exact_use_location.empty() && !use_location.empty() &&
         !anchor_identifier.empty()) {
        exact_use_location =
            template_api::normalize_template_witness_source_location(
                template_api::template_witness_detail::
                    source_location_for_identifier_token_on_or_after(
                    template_witness_context(),
                    use_location,
                    anchor_identifier,
                    true,
                    normalized_name.find('<') != std::string::npos));
      }
      const std::string chosen_use_location =
          !exact_use_location.empty() ? exact_use_location :
                                       use_location;
      const bool source_spells_template =
          !anchor_identifier.empty() &&
          source_location_points_at_identifier(chosen_use_location,
                                               anchor_identifier);
      const bool source_spells_lookup_alias =
          is_identifier_text(normalized_name) &&
          source_location_points_at_identifier(chosen_use_location,
                                               normalized_name);
      const std::string qualified_alias_identifier =
          unqualified_member_name(
              strip_trailing_top_level_template_arguments(normalized_name));
      const std::string qualified_alias_start_location =
          source_location_for_qualified_member_start_on_line(
              use_location,
              qualified_alias_identifier);
      const bool source_spells_qualified_alias =
          !qualified_alias_start_location.empty() &&
          (!is_identifier_text(normalized_name) ||
           qualified_alias_start_location != chosen_use_location);
      if(!source_spells_template &&
         !source_spells_lookup_alias &&
         !source_spells_qualified_alias) {
        return;
      }
      if(source_spells_lookup_alias &&
         !source_spells_template &&
         !source_spells_qualified_alias) {
        return;
      }
      if(source_spells_lookup_alias &&
         !source_spells_template &&
         source_location_identifier_followed_by_indirection_type_suffix(
             chosen_use_location,
             normalized_name)) {
        return;
      }
      std::vector<std::string> anchored_arg_texts;
      const std::vector<std::string> * binding_arg_texts = explicit_arg_texts;
      if(exact_template_type_lookup_anchor_arg_texts(normalized_name,
                                                     anchor_identifier,
                                                     anchored_arg_texts)) {
        binding_arg_texts = &anchored_arg_texts;
      }
      {
        if(source_location_points_at_identifier(chosen_use_location,
                                               anchor_identifier) &&
           !binding_arg_texts &&
           source_location_identifier_followed_by(chosen_use_location,
                                                  anchor_identifier,
                                                  '<')) {
          return;
        }
      }

      std::string selected_decl_location;
      witness::TemplateWitnessSourceAnchorKind selected_decl_anchor_kind =
          witness::TemplateWitnessSourceAnchorKind::None;
      {
        std::vector<const CppAstNode *> decl_candidates;
        if(use.selection.kind == template_api::MS_PRIMARY) {
          if(const CppAstNode * member_decl =
                 find_member_class_template_declaration_node(use.origin)) {
            decl_candidates.push_back(member_decl);
          }
          if(use.instance->template_output_node) {
            decl_candidates.push_back(use.instance->template_output_node);
          }
          if(use.origin->class_node) {
            decl_candidates.push_back(use.origin->class_node);
          }
          if(use.selection.class_node) {
            decl_candidates.push_back(use.selection.class_node);
          }
        } else {
          if(use.selection.class_node) {
            decl_candidates.push_back(use.selection.class_node);
          }
          if(use.origin->class_node) {
            decl_candidates.push_back(use.origin->class_node);
          }
          if(use.instance->template_output_node) {
            decl_candidates.push_back(use.instance->template_output_node);
          }
        }
        for(size_t i = 0; i < decl_candidates.size(); ++i) {
          if(!decl_candidates[i]) {
            continue;
          }
          selected_decl_location = source_location_for_name_in_node(
              *decl_candidates[i],
              class_template->name);
          if(!selected_decl_location.empty()) {
            selected_decl_anchor_kind =
                witness::TemplateWitnessSourceAnchorKind::DeclarationName;
          } else {
            selected_decl_location = source_location_for_node(*decl_candidates[i]);
            if(!selected_decl_location.empty()) {
              selected_decl_anchor_kind =
                  witness::TemplateWitnessSourceAnchorKind::ApproximateDeclaration;
            }
          }
          selected_decl_location =
              template_api::normalize_template_witness_source_location(
                  selected_decl_location);
          if(!selected_decl_location.empty()) {
            break;
          }
        }
      }
      if(!selected_decl_location.empty() &&
         chosen_use_location == selected_decl_location) {
        return;
      }

      witness::ClassUseEmitRequest request;
      request.location =
          source_spells_qualified_alias ?
              qualified_alias_start_location :
              chosen_use_location;
      request.template_name = qualified_template_name;
      request.selection = source_selection_kind_for_match_kind(use.selection.kind);
      witness::set_selected_decl_anchor(request.selected_decl_location,
                                        request.selected_decl_anchor,
                                        selected_decl_location,
                                        selected_decl_anchor_kind);
      {
        const std::string source_template_name =
            unqualified_member_name(qualified_template_name);
        const std::string anchor_identifier =
            !source_template_name.empty() ?
                source_template_name :
                qualified_template_name;
        if(source_spells_template) {
          request.use_anchor_present = true;
          request.use_anchor_location = chosen_use_location;
        }
      }
      if(binding_arg_texts) {
        template_api::append_template_witness_source_bindings(
            *this,
            request.bindings,
            *use.parameters,
            *use.arguments,
            *binding_arg_texts,
            "explicit",
            "defaulted");
      } else {
        const std::string source_template_name =
            unqualified_member_name(qualified_template_name);
        if(source_spells_template) {
          return;
        }
        template_api::append_template_witness_source_bindings(
            *this,
            request.bindings,
            *use.parameters,
            *use.arguments,
            "deduced");
      }
      if(use.selection.parameters && use.selection.parameters != use.parameters) {
        template_api::append_template_witness_source_bindings(
            *this,
            request.specialization_bindings,
            *use.selection.parameters,
            use.selection.arguments,
            "deduced");
      }
      request.ownership = witness::SourceUseOwnership::SourceOwned;
      const bool parent_class_use_records =
          witness::class_use_recording_enabled(request.origin);
      witness::emit_class_use(request);
      if(parent_class_use_records) {
        emit_nested_class_use_source_events_from_location(scope,
                                                          request.location,
                                                          witness::SourceUseOwnership::SourceOwned,
                                                          request.template_name);
      }
    };
    const auto note_exact_local_class_use = [&](const TypePtr & type) -> void
    {
      if(!witness::source_capture_enabled(template_witness_context())) {
        return;
      }
      const std::string template_fragment =
          template_lookup_fragment_text(normalized_name);
      const std::string anchor_identifier =
          template_lookup_fragment_identifier(template_fragment);
      vector<string> anchored_arg_texts;
      if(exact_template_type_lookup_anchor_arg_texts(normalized_name,
                                                     anchor_identifier,
                                                     anchored_arg_texts)) {
        for(size_t i = 0; i < anchored_arg_texts.size(); ++i) {
          if(text_mentions_template_placeholders(scope, anchored_arg_texts[i])) {
            return;
          }
        }
        note_resolved_alias_class_use(type, &anchored_arg_texts);
        return;
      }
      const std::string use_location =
          !entry_use_location.empty() ? entry_use_location :
                                        parser_trace::current_use_location();
      if(is_identifier_text(normalized_name) &&
         (!source_location_points_at_identifier(use_location, normalized_name) ||
          source_location_identifier_followed_by(use_location, normalized_name, '('))) {
        return;
      }
      note_resolved_alias_class_use(type, nullptr);
    };
    TypePtr exact_local = lookup_exact_local_type_name(scope, normalized_name);
    if(exact_local) {
      note_exact_local_class_use(exact_local);
      return exact_local;
    }
    TypePtr exact_bound = lookup_exact_bound_type_name(scope, normalized_name);
    if(exact_bound) {
      return exact_bound;
    }
    bool skip_bound_type_text = false;
    if(is_identifier_text(normalized_name)) {
      skip_bound_type_text = has_non_bound_type_name_shadow(scope, normalized_name);
    }
    if(!skip_bound_type_text &&
       (mentions_template_placeholders || mentions_non_namespace_bindings ||
        mentions_current_specialization_names)) {
      TypePtr bound_type_text = lookup_bound_type_by_text(scope, normalized_name);
      if(bound_type_text) {
        note_resolved_alias_class_use(bound_type_text, nullptr);
        return bound_type_text;
      }
    }
    string cv_core_name;
    bool top_const = false;
    bool top_volatile = false;
    if(strip_top_level_cv_text(normalized_name, cv_core_name, top_const, top_volatile)) {
      TypePtr cv_base =
          lookup_type_impl(scope,
                           cv_core_name,
                           reference_class_templates_only,
                           allow_dependent_class_qualifiers);
      if(cv_base) {
        return apply_cv(cv_base, top_const, top_volatile);
      }
    }
    auto lookup_qualified_alias_template_for_type =
        [&](const QualifiedName & qualified_name) -> AliasTemplateDecl *
        {
          Scope * target =
              semantic_lookup::resolve_qualified_scope_for_class_or_namespace(
                  *this, scope, qualified_name);
          if(!target) {
            return nullptr;
          }

          std::function<AliasTemplateDecl *(Scope &)> lookup_inline_children =
              [&](Scope & current) -> AliasTemplateDecl *
              {
                for(size_t i = 0; i < current.namespace_children.size(); ++i) {
                  Scope & child = *current.namespace_children[i];
                  if(!child.inline_namespace) {
                    continue;
                  }
                  map<string, AliasTemplateDecl *>::iterator found =
                      child.alias_templates.find(qualified_name.name);
                  if(found != child.alias_templates.end()) {
                    return found->second;
                  }
                  AliasTemplateDecl * nested = lookup_inline_children(child);
                  if(nested) {
                    return nested;
                  }
                }
                return nullptr;
              };

          map<string, AliasTemplateDecl *>::iterator found =
              target->alias_templates.find(qualified_name.name);
          if(found != target->alias_templates.end()) {
            return found->second;
          }
          if(target->inline_namespace) {
            return nullptr;
          }
          return lookup_inline_children(*target);
        };
    auto lookup_qualified_class_template_for_type =
        [&](const QualifiedName & qualified_name) -> ClassTemplateDecl *
        {
          Scope * target =
              semantic_lookup::resolve_qualified_scope_for_class_or_namespace(
                  *this, scope, qualified_name);
          if(!target) {
            return nullptr;
          }

          std::function<ClassTemplateDecl *(Scope &)> lookup_inline_children =
              [&](Scope & current) -> ClassTemplateDecl *
              {
                for(size_t i = 0; i < current.namespace_children.size(); ++i) {
                  Scope & child = *current.namespace_children[i];
                  if(!child.inline_namespace) {
                    continue;
                  }
                  map<string, ClassTemplateDecl *>::iterator found =
                      child.class_templates.find(qualified_name.name);
                  if(found != child.class_templates.end()) {
                    return found->second;
                  }
                  ClassTemplateDecl * nested = lookup_inline_children(child);
                  if(nested) {
                    return nested;
                  }
                }
                return nullptr;
              };

          map<string, ClassTemplateDecl *>::iterator found =
              target->class_templates.find(qualified_name.name);
          if(found != target->class_templates.end()) {
            return found->second;
          }
          if(target->inline_namespace) {
            return nullptr;
          }
          return lookup_inline_children(*target);
        };
    const auto lookup_unqualified_type =
        [&]() -> TypePtr
        {
          return lookup_unqualified_generic<TypePtr>(
              scope, normalized_name,
              [this, &scope](Scope & target, const string & lookup_name) -> TypePtr
              {
                const auto direct_lookup =
                    [this, &lookup_name, &scope](Scope & direct_scope) -> TypePtr
                    {
                      if(parser_trace::enabled("template.resolve") &&
                         (lookup_name == "allocator_type" ||
                          lookup_name == "value_type")) {
                        std::ostringstream trace;
                        trace << "unqualified-type-direct-enter name=" << lookup_name
                              << " scope="
                              << semantic_trace::scope_name_for_diagnostic(direct_scope)
                              << " class="
                              << (direct_scope.class_info ?
                                      direct_scope.class_info->qualified_name :
                                      std::string("<none>"))
                              << " function="
                              << (direct_scope.function ?
                                      function_output_name(*direct_scope.function) :
                                      std::string("<none>"));
                        parser_trace::note("template.resolve", std::string(), trace.str());
                      }
                      TypePtr direct_named = direct_named_type(direct_scope, lookup_name);
                      if(parser_trace::enabled("template.resolve") &&
                         (lookup_name == "allocator_type" ||
                          lookup_name == "value_type")) {
                        std::ostringstream trace;
                        trace << "unqualified-type-direct-named name=" << lookup_name
                              << " scope="
                              << semantic_trace::scope_name_for_diagnostic(direct_scope)
                              << " type="
                              << (direct_named ? describe_type(direct_named) :
                                                 std::string("<none>"));
                        parser_trace::note("template.resolve", std::string(), trace.str());
                      }
                      if(direct_named) {
                        return direct_named;
                      }
                      if(direct_scope.class_info &&
                         direct_scope.class_info->reference_member_collection_in_progress) {
                        TypePtr enclosing =
                            lookup_enclosing_type_before_reference_placeholder(
                                direct_scope,
                                lookup_name);
                        if(enclosing) {
                          return enclosing;
                        }
                        return make_named(lookup_name,
                                          "dependent alias " +
                                              direct_scope.class_info->qualified_name +
                                              "::" + lookup_name,
                                          true);
                      }
                      if(direct_scope.class_info) {
                        const bool ensure_current_member_references =
                            !(((direct_scope.class_info->full_member_collection_in_progress ||
                                direct_scope.class_info->reference_member_collection_in_progress) &&
                               direct_scope.class_info->member_scope &&
                               scope_is_within(direct_scope,
                                               direct_scope.class_info->member_scope.get())));
                        MemberTypeLookupResult member =
                            lookup_member_type(*this,
                                               *direct_scope.class_info,
                                               lookup_name,
                                               ensure_current_member_references,
                                               &scope);
                        if(parser_trace::enabled("template.resolve") &&
                           (lookup_name == "allocator_type" ||
                            lookup_name == "value_type")) {
                          std::ostringstream trace;
                          trace << "unqualified-type-direct-member name=" << lookup_name
                                << " scope="
                                << semantic_trace::scope_name_for_diagnostic(direct_scope)
                                << " owner=" << direct_scope.class_info->qualified_name
                                << " ensure-current="
                                << (ensure_current_member_references ? "yes" : "no")
                                << " type="
                                << (member.type ? describe_type(member.type) :
                                                  std::string("<none>"));
                          parser_trace::note("template.resolve", std::string(), trace.str());
                        }
                        if(member.type) {
                          return member.type;
                        }
                      }
                      const bool has_lexical_class =
                          !direct_scope.class_info &&
                          direct_scope.function &&
                          direct_scope.function->lexical_access_class;
                      if(has_lexical_class) {
                        MemberTypeLookupResult member =
                            lookup_member_type(*this,
                                               *direct_scope.function->lexical_access_class,
                                               lookup_name,
                                               true,
                                               &scope);
                        if(member.type) {
                          return member.type;
                        }
                      }
                      return TypePtr();
                    };

                function<TypePtr(Scope &)> lookup_inline_children =
                    [&](Scope & current_scope) -> TypePtr
                    {
                      for(size_t i = 0; i < current_scope.namespace_children.size(); ++i) {
                        Scope & child = *current_scope.namespace_children[i];
                        if(!child.inline_namespace && child.name != "<unnamed>") {
                          continue;
                        }
                        TypePtr direct = direct_lookup(child);
                        if(direct) {
                          return direct;
                        }
                        TypePtr nested = lookup_inline_children(child);
                        if(nested) {
                          return nested;
                        }
                      }
                      return TypePtr();
                    };

                TypePtr direct = direct_lookup(target);
                if(direct) {
                  return direct;
                }
                return lookup_inline_children(target);
              });
        };

    if(is_identifier_text(normalized_name)) {
      TypePtr lexical_type = lookup_unqualified_type();
      if(lexical_type) {
        if(witness::source_capture_enabled(template_witness_context())) {
          const std::string use_location = parser_trace::current_use_location();
          if(source_location_points_at_identifier(use_location, normalized_name) &&
             !source_location_identifier_followed_by(use_location, normalized_name, '(')) {
            note_resolved_alias_class_use(lexical_type, nullptr);
          }
        }
        return lexical_type;
      }
    }

    QualifiedName template_id;
    vector<string> arg_texts;
    const vector<TemplateArgumentSyntax> * arg_syntaxes = nullptr;
    bool parsed_template_id =
        normalized_name.find('<') != string::npos &&
        !normalized_name.empty() &&
        normalized_name.back() == '>' &&
        semantic_utils::split_top_level_template_id_text(normalized_name,
                                                         template_id,
                                                         arg_texts);
    if(parsed_template_id) {
      arg_syntaxes =
          exact_template_type_lookup_anchor_arg_syntaxes(normalized_name,
                                                         template_id.name);
    } else if(normalized_name.find('<') != string::npos &&
              !normalized_name.empty() &&
              normalized_name.back() == '>') {
      const ExactTemplateTypeLookupAnchor * anchor =
          current_exact_template_type_lookup_anchor();
      if(anchor &&
         anchor->has_argument_list &&
         anchor->arg_syntaxes.size() == anchor->arg_texts.size() &&
         (exact_template_type_lookup_anchor_matches_syntax(*anchor,
                                                           normalized_name) ||
          compact_lookup_text(anchor->template_text) ==
              compact_lookup_text(normalized_name)) &&
         semantic_utils::split_qualified_name_text(anchor->identifier,
                                                   template_id)) {
        arg_texts = anchor->arg_texts;
        arg_syntaxes = &anchor->arg_syntaxes;
        parsed_template_id = true;
      }
    }
    if(normalized_name.find('<') != string::npos &&
       !normalized_name.empty() &&
       normalized_name.back() == '>' &&
       !parsed_template_id &&
       !scope_has_template_placeholders(scope)) {
      throw logic_error(string("failed template-id parse in type lookup: ") + normalized_name);
    }
    if(parsed_template_id) {
      const auto try_fast_alias_template_id_lookup =
          [&](const QualifiedName & parsed_template_id,
              const vector<string> & parsed_arg_texts,
              TypePtr & fast_alias) -> bool
      {
          fast_alias.reset();
          const auto resolve_exact_type_arg =
              [&](const string & text, TypePtr & resolved) -> bool
          {
            if(!template_api::type::resolve_type_argument_text(
                    *this, scope, text, resolved) ||
               !resolved ||
               type_depends_on_template_parameter(resolved)) {
              return false;
            }
            return true;
          };

          if((parsed_template_id.name == "__libcpp_remove_reference_t" ||
              parsed_template_id.name == "remove_reference_t") &&
             parsed_arg_texts.size() == 1) {
            TypePtr arg_type;
            if(!resolve_exact_type_arg(parsed_arg_texts[0], arg_type)) {
              return false;
            }
            fast_alias = remove_reference_type(arg_type);
            return true;
          }

          if((parsed_template_id.name == "__conditional_t" ||
              parsed_template_id.name == "conditional_t") &&
             parsed_arg_texts.size() == 3) {
            const std::string condition = trim_space(parsed_arg_texts[0]);
            if(condition == "true" || condition == "false") {
              TypePtr selected_type;
              const std::string & selected_text =
                  condition == "true" ? parsed_arg_texts[1] : parsed_arg_texts[2];
              if(resolve_exact_type_arg(selected_text, selected_type)) {
                fast_alias = selected_type;
                return true;
              }
              if(text_mentions_template_placeholders(scope, selected_text) ||
                 text_mentions_dependent_non_namespace_binding_names(scope, selected_text) ||
                 should_defer_unresolved_type_lookup(scope, selected_text)) {
                fast_alias =
                    make_named(selected_text,
                               "dependent type " + selected_text,
                               true);
                return true;
              }
              return false;
            }
            if(text_mentions_template_placeholders(scope, condition) ||
               text_mentions_dependent_non_namespace_binding_names(scope, condition) ||
               should_defer_unresolved_type_lookup(scope, condition)) {
              fast_alias =
                  make_named(normalized_name,
                             "dependent type " + normalized_name,
                             true);
              return true;
            }
          }

          if(parsed_template_id.name == "__index_sequence" ||
             parsed_template_id.name == "index_sequence") {
            ClassTemplateDecl * integer_sequence =
                lookup_class_template(scope, "__integer_sequence");
            if(!integer_sequence) {
              return false;
            }
            vector<string> integer_args;
            integer_args.reserve(parsed_arg_texts.size() + 1);
            integer_args.push_back("size_t");
            integer_args.insert(integer_args.end(),
                                parsed_arg_texts.begin(),
                                parsed_arg_texts.end());
            ClassInfo * direct =
                reference_class_templates_only ?
                    reference_class_template_instantiation(*integer_sequence, scope, integer_args) :
                    instantiate_class_template(*integer_sequence, scope, integer_args);
            if(!direct) {
              return false;
            }
            fast_alias = direct->type;
            return true;
          }

          return false;
        };
      TypePtr fast_alias;
      if(try_fast_alias_template_id_lookup(template_id, arg_texts, fast_alias)) {
        return cache_qualified_type_result(fast_alias);
      }
      TypePtr current_specialization =
          current_specialization_type_for_template_id(scope,
                                                      template_id,
                                                      arg_texts,
                                                      arg_syntaxes);
      if(current_specialization) {
        return current_specialization;
      }
      AliasTemplateDecl * alias_template = nullptr;
        if(template_id.rooted || !template_id.qualifiers.empty()) {
          alias_template = lookup_qualified_alias_template_for_type(template_id);
        } else {
          alias_template = lookup_alias_template(scope, template_id.name);
        }
        if(alias_template) {
          TypePtr alias = instantiate_alias_template_with_syntax(
              *alias_template,
              scope,
              arg_texts,
              arg_syntaxes,
              reference_class_templates_only);
          if(parser_trace::enabled("template.resolve") &&
             normalized_name.find("__rebind_pointer_t") != string::npos) {
            std::ostringstream trace;
            trace << "lookup-type-alias-result name=" << normalized_name
                  << " alias="
                  << (alias ? describe_type(alias) : std::string("<none>"));
            parser_trace::note("template.resolve",
                               DiagnosticContext::current_frame(),
                               trace.str());
          }
          if(alias) {
            return cache_qualified_type_result(alias);
          }
        }

        ClassTemplateDecl * class_template = nullptr;
        if(template_id.rooted || !template_id.qualifiers.empty()) {
          class_template = lookup_qualified_class_template_for_type(template_id);
        } else {
          class_template = lookup_class_template(scope, template_id.name);
        }
        if(class_template) {
          const std::string lookup_use_location =
              !entry_use_location.empty() ? entry_use_location :
                                            parser_trace::current_use_location();
          const std::string source_template_name =
              unqualified_member_name(template_id.name);
          const std::string anchor_identifier =
              !source_template_name.empty() ?
                  source_template_name :
                  template_id.name;
            const auto note_lookup_class_use =
                [&](ClassInfo * resolved_info,
                    const std::string & use_location) -> void
            {
              const bool trace_enabled = parser_trace::enabled("template.resolve");
              const bool source_capture_enabled =
                  witness::source_capture_enabled(template_witness_context());
              if(use_location.empty() || !resolved_info) {
                return;
              }
              if(!trace_enabled && !source_capture_enabled) {
                return;
              }
            if(resolved_info->template_instantiation_in_progress ||
               resolved_info->full_member_collection_in_progress ||
               resolved_info->reference_member_collection_in_progress ||
               !resolved_info->complete) {
              return;
            }
            std::string exact_lookup_location;
            if(const ExactTemplateTypeLookupAnchor * anchor =
                   current_exact_template_type_lookup_anchor()) {
              if(exact_template_type_lookup_anchor_matches(*anchor,
                                                           normalized_name) ||
                 exact_template_type_lookup_anchor_matches_identifier(*anchor,
                                                                      anchor_identifier)) {
                exact_lookup_location = anchor->location;
              }
            }
            if(exact_lookup_location.empty() && !use_location.empty() &&
               !anchor_identifier.empty()) {
              exact_lookup_location =
                  template_api::normalize_template_witness_source_location(
                      template_api::template_witness_detail::
                          source_location_for_identifier_token_on_or_after(
                          template_witness_context(),
                          use_location,
                          anchor_identifier,
                          true,
                          normalized_name.find('<') != std::string::npos));
            }
            const std::string chosen_use_location =
                !exact_lookup_location.empty() ? exact_lookup_location :
                                                use_location;
            std::vector<std::string> anchored_arg_texts;
            const std::vector<std::string> * binding_arg_texts = &arg_texts;
            bool binding_arg_texts_from_source_template_id = false;
              if(exact_template_type_lookup_anchor_arg_texts(normalized_name,
                                                             anchor_identifier,
                                                             anchored_arg_texts)) {
                binding_arg_texts = &anchored_arg_texts;
              }
              const std::vector<std::string> * source_template_arg_texts =
                  template_api::current_template_id_source_arguments_ptr(
                      chosen_use_location,
                      anchor_identifier);
              if(source_template_arg_texts) {
                if(template_argument_texts_mention_enclosing_source_template_parameters(
                       scope,
                       *source_template_arg_texts) ||
                   template_argument_texts_mention_template_bound_scope_names(
                       scope,
                       *source_template_arg_texts)) {
                  return;
                }
                binding_arg_texts = source_template_arg_texts;
                binding_arg_texts_from_source_template_id = true;
              }
              if(template_argument_texts_mention_enclosing_source_template_parameters(
                     scope,
                     *binding_arg_texts)) {
                return;
              }
              if(template_argument_texts_mention_template_bound_scope_names(
                     scope,
                     *binding_arg_texts) ||
                 template_argument_texts_mention_instantiated_class_local_type_aliases(
                     scope,
                     *binding_arg_texts)) {
                return;
              }
            if(!binding_arg_texts_from_source_template_id &&
               template_argument_texts_mention_source_bindings(*this,
                                                               scope,
                                                               *binding_arg_texts)) {
              return;
            }
            const bool source_event_spells_template =
                source_location_points_at_identifier(chosen_use_location,
                                                     anchor_identifier);
            const auto location_within_node = [&](const std::string & location,
                                                  const CppAstNode * node) -> bool
            {
              if(!(node && !location.empty() && node->token_end > node->token_start)) {
                return false;
              }
              const ParsedSourceLocation parsed_location = parse_source_location(location);
              const ParsedSourceLocation parsed_begin =
                  parse_source_location(source_location_for_node(*node));
              const ParsedSourceLocation parsed_end =
                  parse_source_location(source_location_for_token_index(node->token_end - 1));
              if(!parsed_location.valid || !parsed_begin.valid || !parsed_end.valid ||
                 parsed_location.file != parsed_begin.file ||
                 parsed_location.file != parsed_end.file) {
                return false;
              }
              if(parsed_location.line < parsed_begin.line ||
                 parsed_location.line > parsed_end.line) {
                return false;
              }
              if(parsed_location.line == parsed_begin.line &&
                 parsed_location.column < parsed_begin.column) {
                return false;
              }
              if(parsed_location.line == parsed_end.line &&
                 parsed_location.column > parsed_end.column) {
                return false;
              }
              return true;
            };
            vector<TemplateArgument> resolved_arguments_for_note;
            if(!resolve_template_arguments(scope,
                                           class_template->parameters,
                                           arg_texts,
                                             arg_syntaxes,
                                             resolved_arguments_for_note,
                                             class_template->declaring_scope)) {
                return;
              }
              if(template_arguments_are_dependent(resolved_arguments_for_note)) {
                return;
              }
            canonicalize_simple_dependent_argument_texts(resolved_arguments_for_note);
            const std::string resolved_key =
                template_args_identity_key(resolved_arguments_for_note);
            const template_api::specialization::ClassSpecializationSelection selection_for_note =
                template_api::specialization::select_class_specialization(
                    *this,
                    *class_template,
                    scope,
                    resolved_key,
                    resolved_arguments_for_note);
            if(!source_event_spells_template &&
               (location_within_node(chosen_use_location,
                                     selection_for_note.class_node) ||
                location_within_node(chosen_use_location,
                                     class_template_origin_class_node(resolved_info)))) {
                return;
              }
            const std::string note_location =
                prefer_later_source_location(resolved_info->first_qualifier_use_location,
                                             chosen_use_location);
            const std::string public_location =
                template_api::normalize_template_witness_source_location(
                    !note_location.empty() ? note_location : chosen_use_location);
            const std::string source_event_location =
                template_api::normalize_template_witness_source_location(
                    chosen_use_location);

            const std::string qualified_template_name =
                !resolved_info->qualified_name.empty() ?
                    strip_trailing_top_level_template_arguments(
                        resolved_info->qualified_name) :
                    class_template->name;
            std::string selected_decl_location;
            witness::TemplateWitnessSourceAnchorKind selected_decl_anchor_kind =
                witness::TemplateWitnessSourceAnchorKind::None;
            {
              std::vector<const CppAstNode *> decl_candidates;
              if(selection_for_note.kind == template_api::MS_PRIMARY) {
                if(const CppAstNode * member_decl =
                       find_member_class_template_declaration_node(
                           class_template_origin_decl(resolved_info))) {
                  decl_candidates.push_back(member_decl);
                }
                if(resolved_info->template_output_node) {
                  decl_candidates.push_back(resolved_info->template_output_node);
                }
                if(const CppAstNode * origin_node =
                       class_template_origin_class_node(resolved_info)) {
                  decl_candidates.push_back(origin_node);
                }
                if(selection_for_note.class_node) {
                  decl_candidates.push_back(selection_for_note.class_node);
                }
              } else {
                if(selection_for_note.class_node) {
                  decl_candidates.push_back(selection_for_note.class_node);
                }
                if(const CppAstNode * origin_node =
                       class_template_origin_class_node(resolved_info)) {
                  decl_candidates.push_back(origin_node);
                }
                if(resolved_info->template_output_node) {
                  decl_candidates.push_back(resolved_info->template_output_node);
                }
              }
              for(size_t i = 0; i < decl_candidates.size(); ++i) {
                if(!decl_candidates[i]) {
                  continue;
                }
                selected_decl_location = source_location_for_name_in_node(
                    *decl_candidates[i],
                    class_template->name);
                if(!selected_decl_location.empty()) {
                  selected_decl_anchor_kind =
                      witness::TemplateWitnessSourceAnchorKind::DeclarationName;
                } else {
                  selected_decl_location =
                      source_location_for_node(*decl_candidates[i]);
                  if(!selected_decl_location.empty()) {
                    selected_decl_anchor_kind =
                        witness::TemplateWitnessSourceAnchorKind::ApproximateDeclaration;
                  }
                }
                selected_decl_location =
                    template_api::normalize_template_witness_source_location(
                        selected_decl_location);
                if(!selected_decl_location.empty()) {
                  break;
                }
              }
            }
              if(!selected_decl_location.empty() &&
                 source_event_location == selected_decl_location) {
                return;
            }

            if(source_capture_enabled) {
              witness::ClassUseEmitRequest request;
              request.location = source_event_location;
              request.template_name = qualified_template_name;
              request.selection =
                  source_selection_kind_for_match_kind(selection_for_note.kind);
              witness::set_selected_decl_anchor(request.selected_decl_location,
                                                request.selected_decl_anchor,
                                                selected_decl_location,
                                                selected_decl_anchor_kind);
              const std::string source_template_name =
                  unqualified_member_name(qualified_template_name);
              const std::string anchor_identifier =
                  !source_template_name.empty() ?
                      source_template_name :
                      qualified_template_name;
                const bool use_source_spelling =
                    source_location_points_at_identifier(
                        source_event_location,
                        anchor_identifier);
                if(!use_source_spelling) {
                  return;
                }
                request.use_anchor_present = true;
                request.use_anchor_location = source_event_location;
                if(template_argument_texts_mention_instantiated_class_local_type_aliases(
                       scope,
                       *binding_arg_texts)) {
                  return;
                }
                template_api::append_template_witness_source_bindings(
                    *this,
                    request.bindings,
                    class_template->parameters,
                    resolved_arguments_for_note,
                    *binding_arg_texts,
                    "explicit",
                    "defaulted");
              if(selection_for_note.parameters &&
                 selection_for_note.parameters != &class_template->parameters) {
                template_api::append_template_witness_source_bindings(
                    *this,
                    request.specialization_bindings,
                    *selection_for_note.parameters,
                    selection_for_note.arguments,
                    "deduced");
              }
              witness::emit_class_use(request);
              if(arg_syntaxes) {
                emit_nested_class_use_source_events_from_syntaxes(scope,
                                                                  *arg_syntaxes,
                                                                  witness::SourceUseOwnership::NestedDerived,
                                                                  request.template_name);
              } else {
                emit_nested_class_use_source_events_from_location(scope,
                                                                  request.location,
                                                                  witness::SourceUseOwnership::NestedDerived,
                                                                  request.template_name);
              }
            }

            if(trace_enabled) {
              const auto append_binding_map =
                  [&](std::ostringstream & out,
                      const std::vector<TemplateParameterInfo> & parameters_to_use,
                      const std::vector<TemplateArgument> & arguments_to_use) -> void
              {
                for(size_t i = 0;
                    i < parameters_to_use.size() && i < arguments_to_use.size();
                    ++i) {
                  if(i != 0) {
                    out << ",";
                  }
                  out << (parameters_to_use[i].name.empty() ?
                              std::string("$") + std::to_string(i + 1) :
                              parameters_to_use[i].name)
                      << ":" << template_argument_text(arguments_to_use[i]);
                }
              };

              std::ostringstream primary_bindings;
              append_binding_map(primary_bindings,
                                 class_template->parameters,
                                 resolved_arguments_for_note);

              std::ostringstream specialize_bindings;
              if(selection_for_note.parameters &&
                 selection_for_note.parameters != &class_template->parameters) {
                append_binding_map(specialize_bindings,
                                   *selection_for_note.parameters,
                                   selection_for_note.arguments);
              }

              const witness::SourceSelectionKind selection_kind =
                  source_selection_kind_for_match_kind(selection_for_note.kind);
              std::ostringstream trace;
              trace << "class-use"
                    << " name=" << qualified_template_name
                    << " resolved={" << resolved_info->qualified_name << "}"
                    << " kind=" << witness::source_selection_text(selection_kind)
                    << " decl=" << (!selected_decl_location.empty() ?
                                        selected_decl_location :
                                        std::string("<none>"))
                    << " bindings={" << primary_bindings.str() << "}";
              if(!specialize_bindings.str().empty()) {
                trace << " specialize_bindings={" << specialize_bindings.str() << "}";
              }
              parser_trace::note("template.resolve", public_location, trace.str());
            }
          };
          vector<TemplateArgument> resolved_arguments;
          if(!allow_dependent_class_qualifiers &&
             resolve_template_arguments(scope,
                                        class_template->parameters,
                                        arg_texts,
                                        arg_syntaxes,
                                        resolved_arguments,
                                        class_template->declaring_scope) &&
             template_arguments_are_dependent(resolved_arguments)) {
            if(parser_trace::enabled("template.resolve")) {
              std::ostringstream trace;
              trace << "defer-dependent-class-template-id type=" << normalized_name
                    << " template=" << class_template->name;
              parser_trace::note("template.resolve", std::string(), trace.str());
            }
            ClassInfo * info =
                reference_class_template_instantiation_with_syntax(*class_template,
                                                                   scope,
                                                                   arg_texts,
                                                                   arg_syntaxes,
                                                                   source_use_mode);
            return info ?
                       cache_qualified_type_result(info->type) :
                       make_named(normalized_name,
                                  "dependent type " + normalized_name,
                                  true);
          }
          if(should_defer_member_collection_template_recursion(scope,
                                                               normalized_name,
                                                               class_template)) {
            const bool unresolved =
                should_defer_unresolved_type_lookup(scope, normalized_name);
            if(parser_trace::enabled("template.resolve")) {
              std::ostringstream trace;
              trace << "defer-member-template-recursion type=" << normalized_name
                    << " scope=" << scope_qualified_name(scope, "<here>")
                    << " template=" << class_template->name
                    << " mode=" << (unresolved ? "dependent" : "reference");
              parser_trace::note("template.resolve", std::string(), trace.str());
            }
            if(unresolved) {
              return make_named(normalized_name,
                                "dependent type " + normalized_name,
                                true);
            }
            ClassInfo * info =
                reference_class_template_instantiation_with_syntax(*class_template,
                                                                   scope,
                                                                   arg_texts,
                                                                   arg_syntaxes,
                                                                   source_use_mode);
            if(info) {
              if(template_api::class_template_source_use_emits_class_use(
                     source_use_mode)) {
                note_lookup_class_use(info, lookup_use_location);
              }
              return cache_qualified_type_result(info->type);
            }
            return TypePtr();
          }
          ClassInfo * info = reference_class_templates_only ?
              reference_class_template_instantiation_with_syntax(*class_template,
                                                                 scope,
                                                                 arg_texts,
                                                                 arg_syntaxes,
                                                                 source_use_mode) :
              instantiate_class_template(*class_template,
                                         scope,
                                         arg_texts,
                                         arg_syntaxes);
          if(info) {
            if(template_api::class_template_source_use_emits_class_use(
                   source_use_mode)) {
              note_lookup_class_use(info, lookup_use_location);
            }
            return cache_qualified_type_result(info->type);
          }
          if(!scope_has_template_placeholders(scope)) {
            ostringstream out;
            out << "failed class template instantiation: " << normalized_name;
            out << " [scope " << scope_qualified_name(scope, "<here>") << "]";
            out << " [arg_texts";
            for(size_t i = 0; i < arg_texts.size(); ++i) {
              out << (i == 0 ? " " : ", ") << arg_texts[i];
            }
            out << "]";
            out << " [bindings " << describe_scope_bindings_for_diagnostic(scope) << "]";
            throw_substitution_failure(out.str(), std::string(), "callsemantic");
          }
        }
    }

    TypePtr builtin_transform;
    if(try_builtin_type_transform(scope, normalized_name, builtin_transform)) {
      return builtin_transform;
    }

    const vector<TypePtr> * bound_pack = lookup_type_pack(scope, normalized_name);
    if(bound_pack && bound_pack->size() == 1) {
      return (*bound_pack)[0];
    }

    QualifiedName qualified;
    if(split_qualified_name_text(normalized_name, qualified) &&
       (qualified.rooted || !qualified.qualifiers.empty())) {
      const bool trace_node_traits_pointer_lookup =
          parser_trace::enabled("template.resolve") &&
          (normalized_name.find("__node_traits::pointer") != string::npos ||
           normalized_name.find("__node_base_traits::pointer") != string::npos);
      // `typename T::type` still names a dependent type; the `typename`
      // keyword is a disambiguator, not permission to open dependent class
      // qualifiers eagerly.
      Scope * current =
          semantic_lookup::resolve_qualified_scope_for_class_or_namespace(
              *this,
              scope,
              qualified,
              allow_dependent_class_qualifiers);
      if(trace_node_traits_pointer_lookup) {
        std::ostringstream trace;
        trace << "qualified-lookup-scope name=" << normalized_name
              << " resolved-scope="
              << (current ? scope_qualified_name(*current, "<here>") : std::string("<none>"))
              << " resolved-class="
              << (current && current->class_info ? current->class_info->qualified_name
                                                 : std::string("<none>"));
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      if(!current) {
        return should_defer_unresolved_type_lookup(scope, normalized_name)
                   ? make_named(normalized_name,
                                "dependent type " + normalized_name,
                                true)
                   : TypePtr();
      }
      TypePtr direct_qualified =
          semantic_lookup::resolve_direct_type_qualifier(*this,
                                                         *current,
                                                         scope,
                                                         qualified.name);
      if(trace_node_traits_pointer_lookup) {
        std::ostringstream trace;
        trace << "qualified-lookup-direct name=" << normalized_name
              << " member=" << qualified.name
              << " direct="
              << (direct_qualified ? describe_type(direct_qualified) : std::string("<none>"));
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      if(direct_qualified) {
        if(current->namespace_scope || current->class_info) {
          note_resolved_alias_class_use(direct_qualified, nullptr);
        }
        return cache_qualified_type_result(direct_qualified);
      }
      if(current->class_info) {
        const bool ensure_current_member_references =
            !(((current->class_info->full_member_collection_in_progress ||
                current->class_info->reference_member_collection_in_progress) &&
               current->class_info->member_scope &&
               scope_is_within(*current, current->class_info->member_scope.get())));
        MemberTypeLookupResult member =
            lookup_member_type(*this,
                               *current->class_info,
                               qualified.name,
                               ensure_current_member_references,
                               &scope);
        if(trace_node_traits_pointer_lookup) {
          std::ostringstream trace;
          trace << "qualified-lookup-member name=" << normalized_name
                << " member=" << qualified.name
                << " member-type="
                << (member.type ? describe_type(member.type) : std::string("<none>"))
                << " declared-in="
                << (member.declared_in ? member.declared_in->qualified_name
                                       : std::string("<none>"));
          parser_trace::note("template.resolve", std::string(), trace.str());
        }
        if(member.type) {
          note_resolved_alias_class_use(member.type, nullptr);
          return cache_qualified_type_result(member.type);
        }
        if(current->class_info->reference_member_collection_in_progress &&
           (current->class_info->dependent_instantiation ||
            scope_has_template_placeholders(*current->class_info->member_scope))) {
          return make_named(qualified.name,
                            "dependent alias " +
                                current->class_info->qualified_name + "::" + qualified.name,
                            true);
        }
        if(!current->class_info->complete &&
           current->class_info->template_instantiation_in_progress) {
          ensure_class_reference_members(*current->class_info);
          member = lookup_member_type(*this, *current->class_info, qualified.name, true, &scope);
          if(member.type) {
            note_resolved_alias_class_use(member.type, nullptr);
            return cache_qualified_type_result(member.type);
          }
        }
        if(!current->class_info->complete &&
           !current->class_info->template_instantiation_in_progress &&
           !scope_has_template_placeholders(*current->class_info->member_scope)) {
          DIAG_CONTEXT("qualified_member_type_upgrade [" + normalized_name + "]");
          ClassInfo * completed = complete_class_type(current->class_info->type);
          if(completed) {
            current = completed->member_scope.get();
            direct_qualified =
                semantic_lookup::resolve_direct_type_qualifier(*this,
                                                               *current,
                                                               scope,
                                                               qualified.name);
            if(direct_qualified) {
              note_resolved_alias_class_use(direct_qualified, nullptr);
              return cache_qualified_type_result(direct_qualified);
            }
            member = lookup_member_type(*this, *completed, qualified.name, true, &scope);
            if(member.type) {
              note_resolved_alias_class_use(member.type, nullptr);
              return cache_qualified_type_result(member.type);
            }
          }
        }
      }
      TypePtr introduced = maybe_introduce_elaborated_class_type(scope, name);
      if(introduced) {
        return cache_qualified_type_result(introduced);
      }
      return should_defer_unresolved_type_lookup(scope, normalized_name)
                 ? make_named(normalized_name,
                              "dependent type " + normalized_name,
                              true)
                 : TypePtr();
    }

    TypePtr result = lookup_unqualified_type();
    if(result) {
      return result;
    }
    TypePtr introduced = maybe_introduce_elaborated_class_type(scope, name);
    if(introduced) {
      return introduced;
    }
    return should_defer_unresolved_type_lookup(scope, normalized_name)
               ? make_named(normalized_name,
                            "dependent type " + normalized_name,
                            true)
               : TypePtr();
  }

  ClassInfo * resolve_qualified_owner_class(
      Scope & scope,
      const string & owner_name,
      QualifiedOwnerClassResolution resolution =
          QualifiedOwnerClassResolution::Complete) override
  {
    const bool reference_members_only =
        resolution == QualifiedOwnerClassResolution::ReferenceMembers ||
        (scope_has_template_placeholders(scope) &&
         text_mentions_template_placeholders(scope, owner_name));
    TypePtr class_type = lookup_type_impl(scope, owner_name, true, true);
    if(!class_type && !reference_members_only) {
      class_type = lookup_type_impl(scope, owner_name, false, true);
    }
    ClassInfo * info = reference_members_only ?
        class_info_for_type(class_type) :
        complete_class_type(class_type);
    if(info) {
      if(reference_members_only) {
        ensure_class_reference_members(*info);
      }
      return info;
    }

    bool owner_rooted = false;
    vector<string> components;
    {
      string remaining = trim_space(owner_name);
      if(remaining.compare(0, 2, "::") == 0) {
        owner_rooted = true;
        remaining = trim_space(remaining.substr(2));
      }
      vector<string> reversed_components;
      while(!remaining.empty()) {
        const size_t split = semantic_utils::top_level_scope_split(remaining);
        if(split == string::npos) {
          reversed_components.push_back(trim_space(remaining));
          break;
        }
        reversed_components.push_back(trim_space(remaining.substr(split + 2)));
        remaining = trim_space(remaining.substr(0, split));
      }
      components.assign(reversed_components.rbegin(), reversed_components.rend());
    }
    components.erase(remove_if(components.begin(),
                               components.end(),
                               [](const string & component)
                               {
                                 return component.empty();
                               }),
                     components.end());
    if((owner_rooted || components.size() > 1) && !components.empty()) {
      Scope * current_scope = &scope;
      if(owner_rooted) {
        while(current_scope->parent) {
          current_scope = current_scope->parent;
        }
      }
      ClassInfo * current_class = nullptr;
      for(size_t i = 0; i < components.size(); ++i) {
        const string & component = components[i];
        if(!current_class) {
          Scope * as_namespace = semantic_lookup::resolve_direct_namespace(*current_scope,
                                                                           component);
          if(as_namespace) {
            current_scope = as_namespace;
            continue;
          }
        } else {
          ensure_class_reference_members(*current_class);
        }

        TypePtr component_type = lookup_type_impl(*current_scope, component, true, true);
        if(!component_type && !reference_members_only) {
          component_type = lookup_type_impl(*current_scope, component, false, true);
        }

        ClassInfo * next_class = reference_members_only ?
            class_info_for_type(component_type) :
            complete_class_type(component_type);
        if(next_class) {
          if(reference_members_only) {
            ensure_class_reference_members(*next_class);
          }
          current_class = next_class;
          current_scope = current_class->member_scope.get();
          continue;
        }

        return nullptr;
      }

      return current_class;
    }

    return nullptr;
  }

  Scope * resolve_qualified_function_parse_scope(Scope & scope,
                                                 const CppAstNode & declarator)
  {
    const CppAstNode * identifier = find_descendant_kind(declarator, CppAstKind::identifier);
    if(!identifier) {
      return &scope;
    }

    const QualifiedName * qualified = cppast_qualified_name_syntax(*identifier);
    if(!qualified || qualified->qualifiers.empty()) {
      return &scope;
    }

    QualifiedName owner_qualified;
    owner_qualified.rooted = qualified->rooted;
    owner_qualified.qualifiers = qualified->qualifiers;
    Scope * owner_scope =
        semantic_lookup::resolve_qualified_scope_for_class_or_namespace(
            *this, scope, owner_qualified, true);
    if(owner_scope && owner_scope->class_info) {
      return owner_scope;
    }

    string class_name;
    if(qualified->rooted) {
      class_name += "::";
    }
    for(size_t i = 0; i < qualified->qualifiers.size(); ++i) {
      if(!class_name.empty() && class_name != "::") {
        class_name += "::";
      }
      class_name += qualified->qualifiers[i];
    }

    ClassInfo * info = resolve_qualified_owner_class(scope, class_name);
    return info ? info->member_scope.get() : &scope;
  }

  bool try_parse_conversion_operator_result_type(Scope & scope,
                                                 const CppAstNode & declarator,
                                                 TypePtr & out)
  {
    return semantic_class_model::try_parse_conversion_operator_result_type(*this,
                                                                           scope,
                                                                           declarator,
                                                                           out);
  }

  void overlay_direct_scope_bindings(Scope & target, const Scope & source) const
  {
    template_api::binding::overlay_direct_scope_bindings(target, source);
  }

  CppAstNode filtered_class_member_decl_specifiers(const CppAstNode & specifiers) const
  {
    return semantic_class_model::filtered_class_member_decl_specifiers(specifiers);
  }

  CppAstNode filtered_function_declarator(
      const CppAstNode & declarator) const override
  {
    CppAstNode filtered = declarator;
    vector<CppAstNode> kept;
    for(size_t i = 0; i < filtered.children.size(); ++i) {
      if(filtered.children[i].kind == CppAstKind::function_qualifier ||
         filtered.children[i].kind == CppAstKind::nullability_qualifier ||
         filtered.children[i].kind == CppAstKind::trailing_return_type) {
        continue;
      }
      kept.push_back(filtered.children[i]);
    }
    filtered.children.swap(kept);
    return filtered;
  }

  using MethodSyntaxInfo = semantic_class_model::MethodSyntaxInfo;
  using PreparedMethodParseContext = semantic_class_model::PreparedMethodParseContext;

  void validate_method_virtual_syntax(const MethodSyntaxInfo & info) const
  {
    semantic_class_model::validate_method_virtual_syntax(info);
  }

  TypePtr method_function_type(const TypePtr & class_type,
                               bool is_const_method,
                               bool is_volatile_method,
                               const TypePtr & declared_type)
  {
    return semantic_class_model::method_function_type(class_type,
                                                      is_const_method,
                                                      is_volatile_method,
                                                      declared_type);
  }

  TypePtr const_lvalue_reference_to(const TypePtr & type)
  {
    return make_lvalue_reference_raw(make_cv(type, true, false));
  }

  bool is_same_class_reference_parameter(const TypePtr & class_type,
                                         const TypePtr & param_type) const
  {
    TypePtr base = strip_top_level_cv(param_type);
    if(!base || base->kind != Type::TK_LVALUE_REFERENCE) {
      return false;
    }
    return same_type_with_compatible_top_cv(base->inner, class_type);
  }

  bool is_same_class_rvalue_reference_parameter(const TypePtr & class_type,
                                                const TypePtr & param_type) const
  {
    TypePtr base = strip_top_level_cv(param_type);
    if(!base || base->kind != Type::TK_RVALUE_REFERENCE) {
      return false;
    }
    return same_type_with_compatible_top_cv(base->inner, class_type);
  }

  FunctionBinding * register_class_function(ClassInfo & info,
                                            const string & simple_name,
                                            const TypePtr & declared_type,
                                            const vector<pair<string, TypePtr> > & explicit_params,
                                            const vector<const CppAstNode *> & explicit_default_arguments,
                                            const CppAstNode * body,
                                            const CppAstNode * ctor_initializer,
                                            const FunctionSemanticFlags & flags,
                                            const FunctionTemplateRegistrationIdentity
                                                & template_identity =
                                                FunctionTemplateRegistrationIdentity(),
                                            const CppAstNode * declaration_node = nullptr,
                                            const CppAstNode * parameter_syntax_node = nullptr,
                                            bool prefer_overload_suffix = false)
  {
    TypePtr normalized_declared_type = declared_type;
    vector<pair<string, TypePtr> > normalized_explicit_params = explicit_params;
    if(class_has_template_origin(&info) && info.member_scope) {
      TypePtr resolved_declared_type;
      if(normalized_declared_type &&
         resolve_instantiated_dependent_type(*info.member_scope,
                                            normalized_declared_type,
                                            resolved_declared_type) &&
         resolved_declared_type) {
        normalized_declared_type = resolved_declared_type;
      }
      for(size_t i = 0; i < normalized_explicit_params.size(); ++i) {
        TypePtr resolved_param_type;
        if(normalized_explicit_params[i].second &&
           resolve_instantiated_dependent_type(*info.member_scope,
                                              normalized_explicit_params[i].second,
                                              resolved_param_type) &&
           resolved_param_type) {
          normalized_explicit_params[i].second = resolved_param_type;
        }
      }
    }
    const vector<const CppAstNode *> normalized_default_arguments =
        normalize_default_arguments(normalized_explicit_params, explicit_default_arguments);
    TypePtr effective_type;
    if(flags.is_constructor || flags.is_destructor) {
      vector<TypePtr> params;
      params.push_back(make_pointer(info.type));
      for(size_t i = 0; i < normalized_explicit_params.size(); ++i) {
        params.push_back(normalized_explicit_params[i].second);
      }
      effective_type = make_function(make_fundamental(FT_VOID), params, false);
    } else {
      effective_type = method_function_type(info.type,
                                            flags.is_const_method,
                                            flags.is_volatile_method,
                                            normalized_declared_type);
      if(!effective_type) {
        throw logic_error("invalid member function type");
      }
    }

    const string symbol_qualified_name =
        info.qualified_name + "::" + simple_name;
    unique_ptr<FunctionBinding> binding(new FunctionBinding());
    binding->owner_class = &info;
    binding->is_method = true;
    binding->is_constexpr = flags.is_constexpr;
    binding->is_constructor = flags.is_constructor;
    binding->is_destructor = flags.is_destructor;
    binding->is_explicit = flags.is_explicit;
    binding->is_const_method = flags.is_const_method;
    binding->is_volatile_method = flags.is_volatile_method;
    binding->ref_qualifier = flags.ref_qualifier;
    binding->declared_type = normalized_declared_type;
    binding->function_qualifier = flags.function_qualifier;
    binding->is_defaulted = flags.is_defaulted;
    binding->is_virtual_specified = flags.is_virtual_specified;
    binding->is_pure_virtual = flags.is_virtual_specified &&
                               declaration_node_is_pure_virtual(declaration_node);
    binding->is_override_specified = flags.is_override_specified;
    binding->is_final = flags.is_final;
    binding->access = flags.access;
    binding->type = effective_type;
    binding->declaration_scope = info.member_scope.get();
    binding->declaration_node = declaration_node;
    append_function_declaration_abi_tags(binding->declaration_abi_tags, declaration_node);
    binding->exclude_from_explicit_instantiation =
        declaration_marks_exclude_from_explicit_instantiation(declaration_node);
    binding->definition_node =
        body ? (declaration_node ? declaration_node : body) :
               (flags.is_defaulted ? declaration_node : nullptr);
    if(binding->definition_node) {
      append_function_declaration_abi_tags(binding->definition_abi_tags, declaration_node);
    }
    binding->parameter_syntax_node =
        parameter_syntax_node ? parameter_syntax_node : declaration_node;
    binding->body = body;
    binding->has_definition = body != nullptr || flags.is_defaulted;
    binding->ctor_initializer = ctor_initializer;
    binding->display_name = simple_name;
    binding->name = info.qualified_name + "::" + simple_name;
    template_api::record_function_template_identity(*binding, template_identity);
    binding->params.push_back(make_pair(string("this"), effective_type->params[0]));
    binding->default_arguments.push_back(nullptr);
    for(size_t i = 0; i < normalized_explicit_params.size(); ++i) {
      binding->params.push_back(normalized_explicit_params[i]);
      binding->default_arguments.push_back(
          i < normalized_default_arguments.size() ? normalized_default_arguments[i] : nullptr);
    }
    initialize_function_parameter_aliases(*binding);
    if(flags.is_constructor && normalized_explicit_params.size() == 1 &&
       is_same_class_reference_parameter(info.type, normalized_explicit_params[0].second)) {
      binding->is_copy_constructor = true;
    }
    if(flags.is_constructor && normalized_explicit_params.size() == 1 &&
       is_same_class_rvalue_reference_parameter(info.type, normalized_explicit_params[0].second)) {
      binding->is_move_constructor = true;
    }
    if(!flags.is_constructor && !flags.is_destructor && simple_name == "operator=" &&
       normalized_explicit_params.size() == 1 &&
       is_same_class_reference_parameter(info.type, normalized_explicit_params[0].second)) {
      binding->is_copy_assignment = true;
    }
    if(!flags.is_constructor && !flags.is_destructor && simple_name == "operator=" &&
       normalized_explicit_params.size() == 1 &&
       is_same_class_rvalue_reference_parameter(info.type, normalized_explicit_params[0].second)) {
      binding->is_move_assignment = true;
    }

    binding->symbol =
        make_registered_function_symbol(symbol_qualified_name,
                                        simple_name,
                                        effective_type,
                                        false,
                                        function_symbol_linkage(*info.member_scope,
                                                                declaration_node,
                                                                body,
                                        false,
                                                                flags.function_qualifier,
                                                                template_identity,
                                                                flags.is_defaulted,
                                                                nullptr,
                                                                true),
                                        flags.is_const_method,
                                        flags.is_volatile_method,
                                        flags.ref_qualifier,
                                        flags.is_constructor,
                                        flags.is_destructor,
                                        template_identity,
                                        &info,
                                        prefer_overload_suffix,
                                        declaration_node,
                                        binding->definition_node,
                                        info.member_scope.get(),
                                        binding->is_copy_assignment,
                                        binding->is_move_assignment);
    binding->odr_mergeable_definition =
        symbol_linkage::has_weak_linkage(binding->symbol);

    vector<FunctionBinding *> & slot = info.methods[simple_name];
    auto binding_location = [&](const FunctionBinding * existing, bool definition) -> string
    {
      if(!existing) {
        return string("<none>");
      }
      const CppAstNode * node =
          definition ? existing->definition_node : existing->declaration_node;
      return node ? source_location_for_node(*node) : string("<none>");
    };
    auto same_template_origin_definition =
        [&](const FunctionBinding * existing) -> bool
    {
      if(!existing || !existing->definition_node) {
        return false;
      }
      const CppAstNode * incoming_definition =
          declaration_node ? declaration_node : body;
      if(!incoming_definition) {
        return false;
      }
      if(existing->definition_node == incoming_definition) {
        return true;
      }
      if(!class_has_template_origin(&info) && !info.source_template) {
        return false;
      }
      return source_location_for_node(*existing->definition_node) ==
                 source_location_for_node(*incoming_definition) &&
             node_text(*existing->definition_node) == node_text(*incoming_definition);
    };
    auto refresh_binding_type_from_incoming_definition =
        [&](FunctionBinding & target)
    {
      target.declared_type = binding->declared_type;
      target.type = binding->type;
      for(size_t param_index = 0;
          param_index < target.params.size() &&
          param_index < binding->params.size();
          ++param_index) {
        target.params[param_index].second = binding->params[param_index].second;
      }
    };
    for(size_t i = 0; i < slot.size(); ++i) {
      const bool exact_type_match =
          types_equivalent_for_member_binding(slot[i]->type, effective_type);
      const bool signature_match =
          exact_type_match ||
          callsemantic::function_types_equivalent_for_member_signature(slot[i]->type,
                                                                       effective_type);
      if(!signature_match || slot[i]->ref_qualifier != flags.ref_qualifier) {
        continue;
      }
      const bool same_instantiation_identity =
          template_api::function_binding_matches_instantiation_identity(
              *slot[i], template_identity);
      if(parser_trace::enabled("template.resolve") &&
         function_template_trace_has_identity(template_identity.decl, slot[i])) {
        ostringstream trace;
        trace << "class-function-register-check owner=" << info.qualified_name
              << " simple=" << simple_name
              << " existing=" << static_cast<void *>(slot[i])
              << " existing_decl_loc=" << binding_location(slot[i], false)
              << " existing_def_loc=" << binding_location(slot[i], true)
              << " identity_match=" << (same_instantiation_identity ? "yes" : "no");
        append_function_binding_template_trace_fields(trace, "existing", slot[i]);
        append_function_template_decl_trace_fields(trace,
                                                  *this,
                                                  "incoming",
                                                  template_identity.decl,
                                                  template_identity.key);
        const string incoming_decl_details =
            function_template_decl_location_details(*this, template_identity.decl);
        if(!incoming_decl_details.empty()) {
          trace << " incoming_decl_detail=" << incoming_decl_details;
        }
        const string existing_source_decl_details =
            function_binding_source_template_location_details(*this, slot[i]);
        if(function_binding_template_trace_has_source(slot[i])) {
          trace << " existing_source_decl_loc="
                << function_binding_source_template_primary_location(*this, slot[i]);
        }
        if(!existing_source_decl_details.empty()) {
          trace << " existing_source_decl_detail=" << existing_source_decl_details;
        }
        parser_trace::note("template.resolve",
                           function_template_decl_primary_location(*this,
                                                                   template_identity.decl),
                           trace.str());
      }
      if(!same_instantiation_identity) {
        continue;
      }
      if(body && !exact_type_match) {
        refresh_binding_type_from_incoming_definition(*slot[i]);
      }
      if(template_identity.arguments) {
        template_api::record_function_template_arguments_preserving_pack_sizes(
            *slot[i],
            *template_identity.arguments,
            true);
      }
      template_api::adopt_materialized_owner_template_identity(
          *slot[i],
          template_identity,
          info.member_scope.get());
      upgrade_function_symbol_linkage(*slot[i],
                                      symbol_qualified_name,
                                      simple_name,
                                      effective_type,
                                      function_symbol_linkage(*info.member_scope,
                                                              declaration_node,
                                                              body,
                                                              false,
                                                              flags.function_qualifier,
                                                              template_identity,
                                                              flags.is_defaulted,
                                                              nullptr,
                                                              true));
      slot[i]->is_virtual_specified =
          slot[i]->is_virtual_specified || flags.is_virtual_specified;
      slot[i]->is_pure_virtual = slot[i]->is_pure_virtual || binding->is_pure_virtual;
      slot[i]->is_override_specified =
          slot[i]->is_override_specified || flags.is_override_specified;
      slot[i]->is_final = slot[i]->is_final || flags.is_final;
      if(!slot[i]->declaration_node && declaration_node) {
        slot[i]->declaration_node = declaration_node;
      }
      if(!slot[i]->parameter_syntax_node && parameter_syntax_node) {
        slot[i]->parameter_syntax_node = parameter_syntax_node;
      }
      if(parser_trace::enabled("class.collect") &&
         (simple_name == info.name || simple_name == (string("~") + info.name))) {
        ostringstream trace;
        trace << "class-function-merge"
              << " owner=" << info.qualified_name
              << " simple=" << simple_name
              << " binding=" << static_cast<void *>(slot[i])
              << " is-ctor=" << (slot[i]->is_constructor ? "yes" : "no")
              << " is-dtor=" << (slot[i]->is_destructor ? "yes" : "no")
              << " has-definition=" << (slot[i]->has_definition ? "yes" : "no")
              << " declaration="
              << (declaration_node ? source_location_for_node(*declaration_node)
                                   : string("<none>"))
              << " body=" << (body ? source_location_for_node(*body) : string("<none>"))
              << " defaulted=" << (flags.is_defaulted ? "yes" : "no");
        parser_trace::note("class.collect", std::string(), trace.str());
      }
      if(body) {
        if(parser_trace::enabled("class.collect")) {
          ostringstream trace;
          trace << "class-function-define-existing"
                << " owner=" << info.qualified_name
                << " simple=" << simple_name
                << " binding=" << static_cast<void *>(slot[i])
                << " body="
                << (body ? source_location_for_node(*body) : string("<none>"))
                << " declaration="
                << (declaration_node ? source_location_for_node(*declaration_node)
                                     : string("<none>"));
          parser_trace::note("class.collect", std::string(), trace.str());
        }
        if(slot[i]->has_definition) {
          const bool same_definition_node =
              declaration_node && slot[i]->definition_node == declaration_node;
          const bool same_body_node = body && slot[i]->body == body;
          const std::string incoming_definition_location =
              declaration_node ?
                  source_location_for_node(*declaration_node) :
                  (body ? source_location_for_node(*body) : std::string());
          const std::string existing_definition_location =
              slot[i]->definition_node ?
                  source_location_for_node(*slot[i]->definition_node) :
                  (slot[i]->body ? source_location_for_node(*slot[i]->body) :
                                   std::string());
          const bool same_template_replay_location =
              (info.source_template || info.dependent_instantiation) &&
              !incoming_definition_location.empty() &&
              incoming_definition_location == existing_definition_location;
          if(same_template_origin_definition(slot[i]) ||
             same_definition_node ||
             same_body_node ||
             same_template_replay_location) {
            slot[i]->odr_mergeable_definition =
                slot[i]->odr_mergeable_definition ||
                binding->odr_mergeable_definition ||
                symbol_linkage::has_weak_linkage(binding->symbol);
            refresh_definition_parameter_names(*slot[i], *binding);
            if(parameter_syntax_node) {
              slot[i]->parameter_syntax_node = parameter_syntax_node;
            }
            record_definition_parameter_aliases(*slot[i],
                                                normalized_explicit_params);
            ensure_function_parameter_aliases(*slot[i]);
            release_function_symbol_reservation(binding.get(), slot[i]);
            return slot[i];
          }
          ostringstream out;
          out << "duplicate class member definition"
              << " [class " << info.qualified_name
              << " member " << simple_name
              << " type " << describe_type(effective_type) << "]";
          out << semantic_trace::current_location_note(*this, declaration_node);
          out << semantic_trace::previous_function_location_note(
              *this, "previous definition", slot[i]);
          throw_substitution_failure(out.str(), std::string(), "callsemantic");
        }
        refresh_definition_parameter_names(*slot[i], *binding);
        slot[i]->odr_mergeable_definition =
            slot[i]->odr_mergeable_definition ||
            binding->odr_mergeable_definition ||
            symbol_linkage::has_weak_linkage(binding->symbol);
        slot[i]->has_definition = true;
        slot[i]->body = body;
        slot[i]->definition_node = declaration_node ? declaration_node : body;
        if(parameter_syntax_node) {
          slot[i]->parameter_syntax_node = parameter_syntax_node;
        }
        slot[i]->ctor_initializer = ctor_initializer;
        record_definition_parameter_aliases(*slot[i], normalized_explicit_params);
      }
      if(flags.is_defaulted) {
        if(parser_trace::enabled("class.collect")) {
          ostringstream trace;
          trace << "class-function-default-existing"
                << " owner=" << info.qualified_name
                << " simple=" << simple_name
                << " binding=" << static_cast<void *>(slot[i])
                << " declaration="
                << (declaration_node ? source_location_for_node(*declaration_node)
                                     : string("<none>"));
          parser_trace::note("class.collect", std::string(), trace.str());
        }
        slot[i]->has_definition = true;
        if(!slot[i]->definition_node && declaration_node) {
          slot[i]->definition_node = declaration_node;
        }
      }
      if(!slot[i]->function_qualifier && flags.function_qualifier) {
        slot[i]->function_qualifier = flags.function_qualifier;
      }
      slot[i]->is_copy_constructor =
          slot[i]->is_copy_constructor || binding->is_copy_constructor;
      slot[i]->is_move_constructor =
          slot[i]->is_move_constructor || binding->is_move_constructor;
      slot[i]->is_copy_assignment =
          slot[i]->is_copy_assignment || binding->is_copy_assignment;
      slot[i]->is_move_assignment =
          slot[i]->is_move_assignment || binding->is_move_assignment;
      slot[i]->is_explicit = slot[i]->is_explicit || binding->is_explicit;
      slot[i]->is_defaulted = slot[i]->is_defaulted || flags.is_defaulted;
      slot[i]->is_constexpr = slot[i]->is_constexpr || flags.is_constexpr;
      ensure_function_parameter_aliases(*slot[i]);
      if(slot[i]->default_arguments.size() < binding->default_arguments.size()) {
        slot[i]->default_arguments.resize(binding->default_arguments.size(), nullptr);
      }
      for(size_t j = 0; j < slot[i]->params.size() && j < binding->params.size(); ++j) {
        const bool generated_param_name =
            slot[i]->params[j].first.empty() ||
            slot[i]->params[j].first.compare(0, 3, "arg") == 0;
        if(!body && generated_param_name && !binding->params[j].first.empty()) {
          const string old_name = slot[i]->params[j].first;
          slot[i]->params[j].first = binding->params[j].first;
          update_parameter_alias_after_canonical_rename(slot[i]->parameter_aliases,
                                                        j,
                                                        old_name,
                                                        binding->params[j].first);
        }
        if(j < slot[i]->default_arguments.size() &&
           j < binding->default_arguments.size() &&
           !slot[i]->default_arguments[j] &&
           binding->default_arguments[j]) {
          slot[i]->default_arguments[j] = binding->default_arguments[j];
        }
      }
      if(parser_trace::enabled("template.resolve") &&
         function_template_trace_has_identity(template_identity.decl, slot[i])) {
        ostringstream trace;
        trace << "class-function-register-merge owner=" << info.qualified_name
              << " simple=" << simple_name
              << " binding=" << static_cast<void *>(slot[i])
              << " binding_source_decl_loc="
              << function_binding_source_template_primary_location(*this, slot[i]);
        append_function_binding_template_trace_fields(trace, "binding", slot[i]);
        const string binding_source_decl_details =
            function_binding_source_template_location_details(*this, slot[i]);
        if(!binding_source_decl_details.empty()) {
          trace << " binding_source_decl_detail=" << binding_source_decl_details;
        }
        trace
              << " binding_decl_loc=" << binding_location(slot[i], false)
              << " binding_def_loc=" << binding_location(slot[i], true);
        parser_trace::note("template.resolve",
                           function_template_decl_primary_location(*this,
                                                                   template_identity.decl),
                           trace.str());
      }
      release_function_symbol_reservation(binding.get(), slot[i]);
      return slot[i];
    }

    FunctionBinding * raw = binding.get();
    if(parser_trace::enabled("class.collect") &&
       (simple_name == info.name || simple_name == (string("~") + info.name))) {
      ostringstream trace;
      trace << "class-function-insert"
            << " owner=" << info.qualified_name
            << " simple=" << simple_name
            << " binding=" << static_cast<void *>(raw)
            << " is-ctor=" << (raw->is_constructor ? "yes" : "no")
            << " is-dtor=" << (raw->is_destructor ? "yes" : "no")
            << " is-copy-ctor=" << (raw->is_copy_constructor ? "yes" : "no")
            << " is-move-ctor=" << (raw->is_move_constructor ? "yes" : "no")
            << " type="
            << (raw->declared_type ? cpp_decl::describe_type(raw->declared_type) :
                (raw->type ? cpp_decl::describe_type(raw->type) : string("<none>")))
            << " declaration="
            << (declaration_node ? source_location_for_node(*declaration_node)
                                 : string("<none>"))
            << " body=" << (body ? source_location_for_node(*body) : string("<none>"))
            << " defaulted=" << (flags.is_defaulted ? "yes" : "no");
      parser_trace::note("class.collect", std::string(), trace.str());
    }
    if(parser_trace::enabled("class.collect") && (body || flags.is_defaulted)) {
      ostringstream trace;
      trace << "class-function-insert-defined"
            << " owner=" << info.qualified_name
            << " simple=" << simple_name
            << " binding=" << static_cast<void *>(raw)
            << " is-copy-ctor=" << (raw->is_copy_constructor ? "yes" : "no")
            << " is-move-ctor=" << (raw->is_move_constructor ? "yes" : "no")
            << " type="
            << (raw->declared_type ? cpp_decl::describe_type(raw->declared_type) :
                (raw->type ? cpp_decl::describe_type(raw->type) : string("<none>")))
            << " body="
            << (body ? source_location_for_node(*body) : string("<none>"))
            << " declaration="
            << (declaration_node ? source_location_for_node(*declaration_node)
                                 : string("<none>"))
            << " defaulted=" << (flags.is_defaulted ? "yes" : "no");
      parser_trace::note("class.collect", std::string(), trace.str());
    }
    slot.push_back(raw);
    info.method_declaration_order.push_back(raw);
    semantic_lookup::direct_function_set_slot(*info.member_scope, simple_name).push_back(raw);
    if(parser_trace::enabled("template.resolve") &&
       function_template_trace_has_identity(template_identity.decl, raw)) {
      ostringstream trace;
      trace << "class-function-register-insert owner=" << info.qualified_name
            << " simple=" << simple_name
            << " binding=" << static_cast<void *>(raw)
            << " incoming_decl_loc="
            << function_template_decl_primary_location(*this, template_identity.decl);
      append_function_binding_template_trace_fields(trace, "binding", raw);
      const string incoming_decl_details =
          function_template_decl_location_details(*this, template_identity.decl);
      if(!incoming_decl_details.empty()) {
        trace << " incoming_decl_detail=" << incoming_decl_details;
      }
      trace
            << " binding_decl_loc=" << binding_location(raw, false)
            << " binding_def_loc=" << binding_location(raw, true);
      parser_trace::note("template.resolve",
                         function_template_decl_primary_location(*this, template_identity.decl),
                         trace.str());
    }
    functions.push_back(std::move(binding));
    index_function_binding(functions.back().get());
    return raw;
  }

  FunctionBinding * register_class_function(ClassInfo & info,
                                            const string & simple_name,
                                            const TypePtr & declared_type,
                                            const vector<pair<string, TypePtr> > & explicit_params,
                                            const CppAstNode * body,
                                            const CppAstNode * ctor_initializer,
                                            const FunctionSemanticFlags & flags,
                                            const FunctionTemplateRegistrationIdentity
                                                & template_identity =
                                                FunctionTemplateRegistrationIdentity(),
                                            const CppAstNode * declaration_node = nullptr,
                                            const CppAstNode * parameter_syntax_node = nullptr,
                                            bool prefer_overload_suffix = false)
  {
    return register_class_function(info, simple_name, declared_type, explicit_params,
                                   vector<const CppAstNode *>(),
                                   body, ctor_initializer, flags,
                                   template_identity,
                                   declaration_node,
                                   parameter_syntax_node,
                                   prefer_overload_suffix);
  }

  FunctionBinding * register_class_static_function(ClassInfo & info,
                                                   const string & simple_name,
                                                   const TypePtr & declared_type,
                                                   const vector<pair<string, TypePtr> > & params,
                                                   const vector<const CppAstNode *> & default_arguments,
                                                   const CppAstNode * body,
                                                   const FunctionSemanticFlags & flags,
                                                   const FunctionTemplateRegistrationIdentity
                                                       & template_identity =
                                                       FunctionTemplateRegistrationIdentity(),
                                                   const CppAstNode * declaration_node = nullptr,
                                                   const CppAstNode * parameter_syntax_node = nullptr,
                                                   const CppAstNode * function_qualifier = nullptr,
                                                   bool prefer_overload_suffix = false)
  {
    TypePtr normalized_declared_type = declared_type;
    vector<pair<string, TypePtr> > normalized_params = params;
    if(class_has_template_origin(&info) && info.member_scope) {
      TypePtr resolved_declared_type;
      if(normalized_declared_type &&
         resolve_instantiated_dependent_type(*info.member_scope,
                                            normalized_declared_type,
                                            resolved_declared_type) &&
         resolved_declared_type) {
        normalized_declared_type = resolved_declared_type;
      }
      for(size_t i = 0; i < normalized_params.size(); ++i) {
        TypePtr resolved_param_type;
        if(normalized_params[i].second &&
           resolve_instantiated_dependent_type(*info.member_scope,
                                              normalized_params[i].second,
                                              resolved_param_type) &&
           resolved_param_type) {
          normalized_params[i].second = resolved_param_type;
        }
      }
    }
    register_function(*info.member_scope,
                      simple_name,
                      normalized_declared_type,
                      normalized_params,
                      default_arguments,
                      body,
                      declaration_node,
                      parameter_syntax_node,
                      false,
                      info.member_scope.get(),
                      function_qualifier,
                      flags.is_constexpr,
                      template_identity,
                      prefer_overload_suffix,
                      nullptr,
                      nullptr);
    FunctionBinding * binding = find_matching_function(*info.member_scope,
                                                       simple_name,
                                                       normalized_declared_type,
                                                       template_identity);
    if(!binding) {
      throw logic_error("missing static member function binding");
    }
    binding->owner_class = &info;
    binding->access = flags.access;
    upgrade_function_symbol_linkage(
        *binding,
        info.qualified_name + "::" + simple_name,
        simple_name,
        binding->type,
        function_symbol_linkage(*info.member_scope,
                                declaration_node,
                                body,
                                false,
                                function_qualifier,
                                template_identity,
                                false,
                                nullptr,
                                body != nullptr));
    return binding;
  }

  FunctionBinding * register_class_static_function(ClassInfo & info,
                                                   const string & simple_name,
                                                   const TypePtr & declared_type,
                                                   const vector<pair<string, TypePtr> > & params,
                                                   const CppAstNode * body,
                                                   const FunctionSemanticFlags & flags,
                                                   const FunctionTemplateRegistrationIdentity
                                                       & template_identity =
                                                       FunctionTemplateRegistrationIdentity(),
                                                   const CppAstNode * declaration_node = nullptr,
                                                   const CppAstNode * parameter_syntax_node = nullptr,
                                                   const CppAstNode * function_qualifier = nullptr,
                                                   bool prefer_overload_suffix = false)
  {
    return register_class_static_function(info, simple_name, declared_type, params,
                                          vector<const CppAstNode *>(), body, flags,
                                          template_identity,
                                          declaration_node,
                                          parameter_syntax_node,
                                          function_qualifier,
                                          prefer_overload_suffix);
  }

  bool class_member_object_type_supported(const TypePtr & type) const
  {
    TypePtr base = strip_top_level_cv(type);
    if(!base || !type_is_complete(type)) {
      return false;
    }
    if(base->kind == Type::TK_FUNCTION ||
       base->kind == Type::TK_LVALUE_REFERENCE ||
       base->kind == Type::TK_RVALUE_REFERENCE) {
      return false;
    }
    return true;
  }

  bool bit_field_type_supported(const TypePtr & type) const
  {
    TypePtr base = strip_top_level_cv(type);
    if(!base || !type_is_complete(type)) {
      return false;
    }
    return is_integral_type(base) || is_named_enum_type(base);
  }

  size_t evaluate_bit_field_width(ClassInfo & info, const FieldInfo & field)
  {
    if(!field.bit_width_expression) {
      throw logic_error("bit-field missing width expression");
    }

    long long width = 0;
    if(!evaluate_constant_expression(*info.member_scope, *field.bit_width_expression, width)) {
      ostringstream out;
      out << "bit-field width is not a constant expression";
      if(!info.qualified_name.empty()) {
        out << " [class " << info.qualified_name << "]";
      }
      if(!field.name.empty()) {
        out << " [field " << field.name << "]";
      }
      throw logic_error(out.str());
    }
    if(width < 0) {
      throw logic_error("negative bit-field width");
    }
    return static_cast<size_t>(width);
  }

  void collect_class_simple_declaration(ClassInfo & info,
                                        const CppAstNode & node,
                                        MemberAccess access)
  {
    semantic_class_model::collect_class_simple_declaration(*this, info, node, access);
  }

  void collect_class_bit_field_declaration(ClassInfo & info,
                                           const CppAstNode & node,
                                           MemberAccess access)
  {
    semantic_class_model::collect_class_bit_field_declaration(*this, info, node, access);
  }

  void collect_class_reference_simple_declaration(ClassInfo & info,
                                                  const CppAstNode & node,
                                                  MemberAccess access)
  {
    const CppAstNode * specifiers = find_child_kind(node, CppAstKind::decl_specifier_seq);
    const CppAstNode * declarators = find_child_kind(node, CppAstKind::init_declarator_list);
    if(specifiers &&
       any_of(specifiers->children.begin(), specifiers->children.end(),
              [](const CppAstNode & child) { return node_has_simple_type(child, KW_FRIEND); })) {
      return;
    }
    if(!specifiers || !declarators || !class_member_specifiers_supported(*specifiers, true)) {
      return;
    }

    CppAstNode resolved_specifiers;
    if(!prepare_namespace_scope_specifiers(*info.member_scope, *specifiers, declarators, true,
                                           false, resolved_specifiers)) {
      return;
    }

    bool is_typedef = false;
    TypePtr base;
    const bool has_mutable_specifier =
        decl_spec_contains_token(resolved_specifiers, KW_MUTABLE);
    const CppAstNode filtered_specifiers =
        filtered_class_member_decl_specifiers(resolved_specifiers);
    if(!parse_decl_spec(filtered_specifiers, *info.member_scope, is_typedef, base)) {
      return;
    }

    const bool is_static_member = decl_spec_contains_token(resolved_specifiers, KW_STATIC);
    const bool is_constexpr_member = decl_spec_contains_token(resolved_specifiers, KW_CONSTEXPR);
    const bool is_thread_local_member =
        decl_spec_contains_token(resolved_specifiers, KW_THREAD_LOCAL);

    for(size_t j = 0; j < declarators->children.size(); ++j) {
      const CppAstNode & init_decl = declarators->children[j];
      if(init_decl.kind != CppAstKind::init_declarator ||
         init_decl.children.empty()) {
        continue;
      }

      PreparedMethodParseContext prepared_member_method;
      semantic_class_model::prepare_method_parse_context(&resolved_specifiers,
                                                         init_decl.children[0],
                                                         prepared_member_method,
                                                         true,
                                                         true);
      string member_name;
      TypePtr member_type;
      if(!parse_declarator(*info.member_scope,
                           prepared_member_method.parse_declarator_node(),
                           base, member_name, member_type) ||
         member_name.empty()) {
        continue;
      }

      if(is_typedef) {
        info.member_scope->named_types[member_name] =
            canonicalize_member_typedef_type(*info.member_scope, member_type);
        continue;
      }

      TypePtr stripped = strip_top_level_cv(member_type);
      if(stripped && stripped->kind == Type::TK_FUNCTION) {
        continue;
      }

      if(is_static_member || is_constexpr_member) {
        if(has_mutable_specifier) {
          continue;
        }
        ValueBinding binding(ValueBinding::VK_VARIABLE, member_name, member_type);
        binding.access = access;
        binding.owner_class = &info;
        binding.is_thread_local = is_thread_local_member;
        binding.has_storage_definition = false;
        binding.requires_constant_initializer = is_constexpr_member;
        if(init_decl.children.size() > 1 &&
           init_decl.children[1].kind == CppAstKind::initializer &&
           init_decl.children[1].children.size() == 1) {
          binding.constant_initializer = &init_decl.children[1];
          binding.constant_initializer_scope = info.member_scope.get();
          constant_eval::ConstexprValue value;
          if(evaluate_initializer_constant_value(*info.member_scope,
                                                 init_decl.children[1],
                                                 member_type,
                                                 value)) {
            binding.has_constexpr_value = true;
            binding.constexpr_value = value;
            long long integral = 0;
            if(constant_eval::constexpr_value_to_integral(value, integral)) {
              binding.has_constant_value = true;
              binding.constant_value = integral;
            }
          }
        }
        info.member_scope->values[member_name] = binding;
      }
    }
  }

  void collect_class_reference_bit_field_declaration(ClassInfo & info,
                                                     const CppAstNode & node,
                                                     MemberAccess access)
  {
    const CppAstNode * specifiers = find_child_kind(node, CppAstKind::decl_specifier_seq);
    if(!specifiers || !class_member_specifiers_supported(*specifiers, true)) {
      return;
    }

    CppAstNode resolved_specifiers;
    if(!prepare_namespace_scope_specifiers(*info.member_scope, *specifiers, nullptr, true,
                                           false, resolved_specifiers)) {
      return;
    }

    bool is_typedef = false;
    TypePtr base;
    const bool has_mutable_specifier =
        decl_spec_contains_token(resolved_specifiers, KW_MUTABLE);
    const CppAstNode filtered_specifiers =
        filtered_class_member_decl_specifiers(resolved_specifiers);
    if(!parse_decl_spec(filtered_specifiers, *info.member_scope, is_typedef, base) ||
       is_typedef || !bit_field_type_supported(base)) {
      return;
    }

    for(size_t i = 0; i < node.children.size(); ++i) {
      const CppAstNode & child = node.children[i];
      if(child.kind != CppAstKind::bit_field_declarator) {
        continue;
      }

      const CppAstNode * declarator = find_child_kind(child, CppAstKind::declarator);
      const CppAstNode * width =
          !child.children.empty() ? &child.children.back() : nullptr;
      if(!width || width->kind == CppAstKind::declarator) {
        continue;
      }

      string member_name;
      TypePtr member_type = base;
      if(declarator &&
         !parse_declarator(*info.member_scope, *declarator, base, member_name, member_type)) {
        continue;
      }
      if(!declarator) {
        member_type = base;
      }

      FieldInfo field;
      field.name = member_name;
      field.type = member_type;
      field.is_mutable = has_mutable_specifier;
      field.bit_width_expression = width;
      field.is_bit_field = true;
      field.access = access;
      info.fields.push_back(field);

      if(!member_name.empty()) {
        ValueBinding binding(ValueBinding::VK_FIELD, member_name, member_type);
        binding.owner_class = &info;
        binding.access = access;
        binding.is_mutable = has_mutable_specifier;
        binding.is_bit_field = true;
        info.member_scope->values[member_name] = binding;
      }
    }
  }

  void populate_class_reference_members(ClassInfo & info, const CppAstNode & node)
  {
    if(info.reference_members_collected) {
      return;
    }

    function<void(const CppAstNode &, MemberAccess)> collect_reference_template_declaration =
        [&](const CppAstNode & member, MemberAccess access) -> void
    {
      if(member.kind != CppAstKind::template_declaration || member.children.empty()) {
        return;
      }

      const CppAstNode & inner = member.children.back();
      if(inner.kind == CppAstKind::template_declaration) {
        collect_reference_template_declaration(inner, access);
        return;
      }

      if(inner.kind == CppAstKind::class_specifier ||
         inner.kind == CppAstKind::class_forward_declaration) {
        if(!inner.value.empty() &&
           !direct_class_template(*info.member_scope, inner.value)) {
          collect_template_declaration(*info.member_scope, member, access);
        }
        return;
      }

      if(inner.kind == CppAstKind::alias_declaration) {
        if(!inner.value.empty() &&
           !lookup_direct_alias_template(*info.member_scope, inner.value)) {
          collect_template_declaration(*info.member_scope, member, access);
        }
        return;
      }
    };

    MemberAccess current_access = info.default_access;
    for(size_t i = 0; i < node.children.size(); ++i) {
      const CppAstNode & child = node.children[i];
      if(child.kind == CppAstKind::class_key ||
         child.kind == CppAstKind::base_clause ||
         child.kind == CppAstKind::function_definition ||
         child.kind == CppAstKind::special_member_definition ||
         child.kind == CppAstKind::special_member_declaration) {
        continue;
      }
      if(child.kind == CppAstKind::access_specifier) {
        current_access = access_from_node(child);
        continue;
      }
      if(child.kind == CppAstKind::simple_declaration) {
        collect_class_reference_simple_declaration(info, child, current_access);
        continue;
      }
      if(child.kind == CppAstKind::bit_field_declaration) {
        collect_class_reference_bit_field_declaration(info, child, current_access);
        continue;
      }
      if(child.kind == CppAstKind::template_declaration) {
        collect_reference_template_declaration(child, current_access);
        continue;
      }
      if(child.kind == CppAstKind::class_specifier ||
         child.kind == CppAstKind::class_forward_declaration) {
        if(!child.value.empty()) {
          collect_class_declaration(*info.member_scope, child);
        }
        continue;
      }
      if(child.kind == CppAstKind::enum_specifier) {
        collect_enum_declaration(*info.member_scope, child);
        continue;
      }
      if(child.kind == CppAstKind::alias_declaration) {
        const CppAstNode * type_id = find_child_kind(child, CppAstKind::type_id);
        if(!type_id) {
          continue;
        }
        const std::string type_id_text = node_text(*type_id);
        const bool keep_dependent_alias =
            scope_has_template_placeholders(*info.member_scope) &&
            (text_mentions_template_placeholders(*info.member_scope, type_id_text) ||
             text_mentions_dependent_non_namespace_binding_names(*info.member_scope,
                                                                 type_id_text));
        TypePtr alias;
        parse_type_id(*info.member_scope, *type_id, alias, true);
        if(keep_dependent_alias) {
          alias = make_named(child.value,
                             "dependent alias " + info.qualified_name + "::" + child.value,
                             true);
        }
        if(alias) {
          template_api::binding::bind_named_type(*info.member_scope, child.value, alias);
        }
      }
    }

    info.reference_members_collected = true;
  }

  void ensure_class_reference_members(ClassInfo & info)
  {
    semantic_class_model::ensure_class_reference_members(*this, info);
    if(template_api::apply_out_of_class_static_member_definitions_to_reference(
           *this,
           info)) {
      semantic_class_model::finalize_class_constant_members(*this, info);
    }
  }

  void finalize_class_constant_members(ClassInfo & info)
  {
    semantic_class_model::finalize_class_constant_members(*this, info);
  }

  void collect_dependent_class_simple_declaration(ClassInfo & info,
                                                  const CppAstNode & node,
                                                  MemberAccess access)
  {
    semantic_class_model::collect_dependent_class_simple_declaration(*this, info, node, access);
  }

  void collect_dependent_class_bit_field_declaration(ClassInfo & info,
                                                     const CppAstNode & node,
                                                     MemberAccess access)
  {
    semantic_class_model::collect_dependent_class_bit_field_declaration(*this, info, node, access);
  }

  void collect_class_method_definition(ClassInfo & info,
                                       const CppAstNode & node,
                                       MemberAccess access)
  {
    semantic_class_model::collect_class_method_definition(*this, info, node, access);
  }

  void collect_special_member(ClassInfo & info,
                              const CppAstNode & node,
                              MemberAccess access)
  {
    semantic_class_model::collect_special_member(*this, info, node, access);
  }

  void collect_conversion_operator_member(ClassInfo & info,
                                          const CppAstNode & node,
                                          MemberAccess access)
  {
    semantic_class_model::collect_conversion_operator_member(*this, info, node, access);
  }

  void finalize_class_virtuals(ClassInfo & info)
  {
    semantic_class_model::finalize_class_virtuals(*this, info);
  }

  void finalize_class_layout(ClassInfo & info)
  {
    semantic_class_model::finalize_class_layout(*this, info);
  }

  void ensure_implicit_special_members(ClassInfo & info)
  {
    semantic_class_model::ensure_implicit_special_members(*this, info);
  }

  FunctionBinding * ensure_implicit_copy_constructor(ClassInfo & info)
  {
    return semantic_class_model::ensure_implicit_copy_constructor(
        *this, info);
  }

  FunctionBinding * ensure_implicit_move_constructor(ClassInfo & info)
  {
    return semantic_class_model::ensure_implicit_move_constructor(
        *this, info);
  }

  FunctionBinding * ensure_implicit_move_assignment(ClassInfo & info)
  {
    return semantic_class_model::ensure_implicit_move_assignment(
        *this, info);
  }

  bool can_synthesize_aggregate_constructor(const ClassInfo & info) const
  {
    return semantic_class_model::can_synthesize_aggregate_constructor(info);
  }

  bool class_has_bit_fields(const ClassInfo & info) const
  {
    return semantic_class_model::class_has_bit_fields(info);
  }

  FunctionBinding * ensure_implicit_aggregate_constructor(ClassInfo & info)
  {
    return semantic_class_model::ensure_implicit_aggregate_constructor(
        *this, info);
  }

  FunctionBinding * ensure_implicit_copy_assignment(ClassInfo & info)
  {
    return semantic_class_model::ensure_implicit_copy_assignment(
        *this, info);
  }

  void populate_class_info(ClassInfo & info, const CppAstNode & node)
  {
    DIAG_CONTEXT("populate_class_info [" + info.qualified_name + "]" +
                 source_location_for_node(node));
    semantic_class_model::populate_class_info(*this, info, node);
  }

  void collect_class_declaration(Scope & scope, const CppAstNode & node)
  {
    const string class_name = node.value.empty() ? string("<unnamed>") : node.value;
    DIAG_CONTEXT("collect_class_declaration [" + class_name + "]" +
                 source_location_for_node(node));
    if(parser_trace::enabled("class.collect")) {
      ostringstream trace;
      trace << "collect-class-declaration"
            << " scope=" << scope_qualified_name(scope, "<scope>")
            << " name=" << class_name
            << " kind=" << cppast_kind_text(node.kind)
            << " text=" << node_text(node);
      parser_trace::note("class.collect", source_location_for_node(node), trace.str());
    }
    semantic_class_model::collect_class_declaration(*this, scope, node);
  }

  void collect_enum_declaration(Scope & scope, const CppAstNode & node)
  {
    Scope & durable_scope = persistent_enclosing_scope(scope);
    string enum_prefix = "enum";
    if(const CppAstNode * enum_key = find_child_kind(node, CppAstKind::enum_key)) {
      enum_prefix += " ";
      enum_prefix += node_text(*enum_key);
    }
    const bool scoped = find_child_kind(node, CppAstKind::enum_key) != nullptr;

    const string enum_name = node.value.empty() ? next_anonymous_enum_name() : node.value;
    size_t enum_alignment = 4;
    size_t enum_size = 4;
    if(const CppAstNode * underlying = find_child_kind(node, CppAstKind::type_id)) {
      TypePtr underlying_type;
      if(!parse_type_id(scope, *underlying, underlying_type)) {
        throw logic_error("unsupported enum underlying type");
      }
      enum_alignment = type_alignment(underlying_type);
      enum_size = type_size(underlying_type);
    }
    TypePtr enum_type = node.value.empty() ? TypePtr() : direct_named_type(scope, node.value);
    if(enum_type) {
      TypePtr base = strip_top_level_cv(enum_type);
      if(!base || base->kind != Type::TK_NAMED) {
        throw logic_error(string("conflicting enum binding") +
                          semantic_trace::current_location_note(*this, &node) +
                          semantic_trace::previous_class_location_note(
                              *this,
                              "previous declaration",
                              base ? class_info_for_type(base) : nullptr));
      }
    } else {
      enum_type = make_enum_type(scope, enum_prefix, enum_name, true,
                                 enum_alignment, enum_size);
      if(node.value.empty()) {
        assign_anonymous_enum_itanium_metadata(scope, enum_type);
      }
      if(!node.value.empty()) {
        scope.named_types[node.value] = enum_type;
        durable_scope.named_types[node.value] = enum_type;
      }
    }

    Scope * enumerator_scope = &scope;
    if(scoped && !node.value.empty()) {
      map<string, Scope *>::iterator found = type_scopes_by_key.find(enum_type->named_key);
      if(found != type_scopes_by_key.end()) {
        enumerator_scope = found->second;
      } else {
        durable_type_scopes.emplace_back(new Scope(&durable_scope, enum_name, false));
        enumerator_scope = durable_type_scopes.back().get();
        enumerator_scope->named_types[enum_name] = enum_type;
        type_scopes_by_key[enum_type->named_key] = enumerator_scope;
      }
    } else if(!node.value.empty()) {
      type_scopes_by_key[enum_type->named_key] = &durable_scope;
    }

    long long next_value = -1;
    bool next_value_known = true;
    for(size_t i = 0; i < node.children.size(); ++i) {
      const CppAstNode & child = node.children[i];
      if(child.kind == CppAstKind::enum_key) {
        continue;
      }
      if(child.kind == CppAstKind::type_id) {
        continue;
      }
      if(child.kind != CppAstKind::enumerator) {
        throw logic_error("unsupported enum child");
      }

      long long value = next_value + 1;
      bool value_known = next_value_known;
      if(!child.children.empty()) {
        bool evaluated = false;
        if(child.children.size() == 1) {
          try {
            evaluated = evaluate_constant_expression(scope, child.children[0], value);
          } catch(const logic_error &) {
            if(!scope_has_template_placeholders(scope)) {
              throw;
            }
          }
        }
        if(child.children.size() != 1 || !evaluated) {
          if(!scope_has_template_placeholders(scope)) {
            throw logic_error("unsupported enumerator value");
          }
          value_known = false;
        }
      }

      ValueBinding binding(ValueBinding::VK_VARIABLE, child.value, enum_type);
      binding.declaration_node = &child;
      binding.definition_node = &child;
      if(value_known) {
        binding.has_constant_value = true;
        binding.constant_value = value;
        next_value = value;
        next_value_known = true;
      } else {
        binding.dependent_template_value = true;
        next_value_known = false;
      }
      enumerator_scope->values[child.value] = binding;
    }
  }

  ConversionRank inheritance_conversion_rank(const TypePtr & target,
                                             const ExprInfo & arg)
  {
    return semantic_conversion::inheritance_conversion_rank(*this, target, arg);
  }

  ConversionRank conversion_rank(const TypePtr & target, const ExprInfo & arg)
  {
    return semantic_conversion::conversion_rank(*this, target, arg);
  }

  bool can_copy_initialize(const TypePtr & target, const ExprInfo & expr)
  {
    return semantic_conversion::can_copy_initialize(*this, target, expr);
  }

  bool classify_null_pointer_constant(Scope & scope,
                                      const CppAstNode & node,
                                      const ExprInfo & expr)
  {
    if(!expr.type || expr.category != VC_PRVALUE) {
      return false;
    }

    TypePtr base = strip_top_level_cv(expr.type);
    if(!base) {
      return false;
    }

    if(base->kind == Type::TK_FUNDAMENTAL &&
       base->fundamental == FT_NULLPTR_T) {
      return true;
    }

    if(!is_integral_type(base)) {
      return false;
    }

    if(node.kind == CppAstKind::id_expression && node.value == "__null") {
      return true;
    }

    constant_eval::ConstexprValue value;
    try
    {
      if(!evaluate_constant_expression_value(scope, node, value)) {
        return false;
      }
    }
    catch(const logic_error &)
    {
      return false;
    }

    long long integral_value = 0;
    return constant_eval::constexpr_value_to_integral(value, integral_value) &&
           integral_value == 0;
  }

  bool try_analyze_target_aware_expression(Scope & scope,
                                           const CppAstNode & node,
                                           const TypePtr & target,
                                           ExprInfo & out,
                                           const ConstructorSelectionOptions * ctor_options_override)
  {
    const auto collect_constructor_arg_nodes =
        [this, &scope, &target](const CppAstNode & expr,
                                vector<const CppAstNode *> & args) -> bool
        {
          args.clear();
          const CppAstNode * payload = &expr;
          if(expr.kind == CppAstKind::initializer && expr.children.size() == 1) {
            payload = &expr.children[0];
          }

          if(payload->kind == CppAstKind::paren_initializer ||
             payload->kind == CppAstKind::argument_list ||
             payload->kind == CppAstKind::paren_argument_list ||
             payload->kind == CppAstKind::braced_init_list) {
            for(size_t i = 0; i < payload->children.size(); ++i) {
              args.push_back(&payload->children[i]);
            }
            return true;
          }

          if(payload->kind != CppAstKind::call_expression || payload->children.empty() ||
             payload->children[0].kind != CppAstKind::id_expression) {
            return false;
          }

          TypePtr stripped_target = strip_top_level_cv(target);
          if(stripped_target &&
             (stripped_target->kind == Type::TK_LVALUE_REFERENCE ||
              stripped_target->kind == Type::TK_RVALUE_REFERENCE)) {
            return false;
          }

          TypePtr callee_type =
              lookup_type_node(scope,
                               payload->children[0],
                               payload->children[0].value,
                               false);
          if(!callee_type ||
             !same_type_with_compatible_top_cv(strip_top_level_cv(callee_type),
                                               strip_top_level_cv(target))) {
            return false;
          }
          record_class_use_for_resolved_type_node(
              scope,
              payload->children[0],
              callee_type,
              source_location_for_node(payload->children[0]),
              true);

          const CppAstNode * argument_list = find_child_kind(*payload, CppAstKind::argument_list);
          if(!argument_list) {
            argument_list = find_child_kind(*payload, CppAstKind::paren_argument_list);
          }
          if(!argument_list) {
            return true;
          }
          if(argument_list->children.size() == 1 &&
             argument_list->children[0].kind == CppAstKind::braced_init_list) {
            for(size_t i = 0; i < argument_list->children[0].children.size(); ++i) {
              args.push_back(&argument_list->children[0].children[i]);
            }
            return true;
          }
          for(size_t i = 0; i < argument_list->children.size(); ++i) {
            args.push_back(&argument_list->children[i]);
          }
          return true;
        };

    TypePtr element_type;
    if(node.kind == CppAstKind::braced_init_list &&
       is_initializer_list_type(target, &element_type)) {
      out = semantic_expression::make_initializer_list_expression(*this, scope, target, node);
      return true;
    }

    std::function<bool(const TypePtr &, const CppAstNode &, ExprInfo &)>
        try_build_array_expr;
    try_build_array_expr =
        [&](const TypePtr & expr_type,
            const CppAstNode & init_node,
            ExprInfo & expr_out) -> bool
        {
          TypePtr expr_base = strip_top_level_cv(remove_reference_type(expr_type));
          if(!expr_base || expr_base->kind != Type::TK_ARRAY || !expr_base->has_bound ||
             init_node.kind != CppAstKind::braced_init_list ||
             semantic_lifetime::has_designated_braced_init(init_node)) {
            return false;
          }
          if(init_node.children.size() > expr_base->bound) {
            return false;
          }

          ExprInfo result;
          result.type = expr_type;
          result.category = VC_LVALUE;
          result.node = make_dump_node(CallSemKind::braced_init_list);
          set_expr_metadata(result.node, result.type, result.category);

          try
          {
            for(size_t i = 0; i < expr_base->bound; ++i) {
              ExprInfo element;
              if(i < init_node.children.size()) {
                element =
                    analyze_expression_for_target(scope, init_node.children[i], expr_base->inner);
                if(!can_copy_initialize(expr_base->inner, element)) {
                  return false;
                }
              } else {
                TypePtr element_base =
                    strip_top_level_cv(remove_reference_type(expr_base->inner));
                if(element_base &&
                   (element_base->kind == Type::TK_ARRAY ||
                    complete_class_type(element_base))) {
                  CppAstNode empty;
                  empty.kind = CppAstKind::braced_init_list;
                  element = analyze_expression_for_target(scope, empty, expr_base->inner);
                } else {
                  element = make_value_initialized_expr(expr_base->inner);
                }
              }
              result.node.children.push_back(element.node);
            }
          }
          catch(const std::logic_error &)
          {
            return false;
          }

          expr_out = result;
          return true;
        };

    if(node.kind == CppAstKind::braced_init_list &&
       try_build_array_expr(target, node, out)) {
      return true;
    }

    TypePtr target_base = strip_top_level_cv(remove_reference_type(target));
    ClassInfo * target_class = complete_class_type(target_base);
    if(target_class) {
      std::function<bool(const TypePtr &, const CppAstNode &, ExprInfo &)>
          try_build_aggregate_expr;

      try_build_aggregate_expr =
          [&](const TypePtr & expr_type,
              const CppAstNode & init_node,
              ExprInfo & expr_out) -> bool
          {
            TypePtr expr_base = strip_top_level_cv(remove_reference_type(expr_type));
            ClassInfo * expr_info = complete_class_type(expr_base);
            if(!expr_info) {
              return false;
            }

            std::vector<ExprInfo> source_args;
            try
            {
              if(!semantic_lifetime::build_aggregate_constructor_source_args(*this,
                                                                             scope,
                                                                             *expr_info,
                                                                             init_node,
                                                                             source_args)) {
                return false;
              }
            }
            catch(const std::logic_error &)
            {
              return false;
            }

            std::vector<ExprInfo> converted;
            const ConstructorSelectionOptions ctor_options =
                constructor_lifecycle_service::selection_options_for(
                    constructor_lifecycle_service::aggregate_construction_profile(
                        "target-aware aggregate construction"));
            ConstructorSelectionOptions effective_ctor_options = ctor_options;
            effective_ctor_options.use_location =
                preferred_source_location_for_node(init_node);
            ScopedTemplateUseLocation use_location_guard(
                preferred_source_location_for_node(init_node));
            FunctionBinding * ctor =
                select_constructor_from_exprs(scope,
                                              *expr_info,
                                              source_args,
                                              converted,
                                              nullptr,
                                              effective_ctor_options);
            if(!ctor) {
              return false;
            }
            expr_out = make_constructor_conversion_expr(*ctor, expr_type, converted);
            return true;
          };

      if(node.kind == CppAstKind::braced_init_list &&
         try_build_aggregate_expr(target, node, out)) {
        return true;
      }

      vector<const CppAstNode *> ctor_arg_nodes;
      if(collect_constructor_arg_nodes(node, ctor_arg_nodes)) {
        vector<ExprInfo> converted;
        ConstructorSelectionOptions ctor_options;
        if(ctor_options_override) {
          ctor_options = *ctor_options_override;
        }
        constructor_lifecycle_service::apply_selection_profile(
            ctor_options,
            constructor_lifecycle_service::direct_initialization_profile(
                "target-aware construction"));
        ctor_options.use_location = preferred_source_location_for_node(node);
        ScopedTemplateUseLocation use_location_guard(
            preferred_source_location_for_node(node));
        FunctionBinding * ctor =
            (node.kind == CppAstKind::braced_init_list) ?
                select_constructor_for_direct_braced_init(scope,
                                                          *target_class,
                                                          node,
                                                          converted,
                                                          ctor_options) :
                select_constructor(scope,
                                   *target_class,
                                   ctor_arg_nodes,
                                   converted,
                                   ctor_options);
        if(ctor) {
          out = make_constructor_conversion_expr(*ctor, target, converted);
          return true;
        }
      }
    }

    if(node.kind == CppAstKind::id_expression &&
       resolve_function_id_for_target(scope,
                                      node.value,
                                      target,
                                      out,
                                      cppast_qualified_name_syntax(node),
                                      cppast_template_id_syntax(node))) {
      return true;
    }
    return false;
  }

  ExprInfo make_constructor_conversion_expr(FunctionBinding & ctor,
                                            const TypePtr & result_type,
                                            const vector<ExprInfo> & args)
  {
    CallableEmissionDecision emission =
        decide_callable_emission(&ctor, OutputReason::ConstructorUse, !ctor.is_deleted);
    require_function_definition(&ctor,
                                OutputReason::ConstructorUse,
                                emission.mark_output_required_now);
    FunctionBinding * resolved_ctor =
        semantic_output::resolve_output_function_binding(*this, &ctor);
    FunctionBinding & emitted_ctor = resolved_ctor ? *resolved_ctor : ctor;
    ExprInfo result;
    result.type = result_type;
    result.category = VC_PRVALUE;
    result.node = make_dump_node(CallSemKind::call_expression);
    set_expr_metadata(result.node, result.type, result.category);

    DumpNode resolved_callee = make_dump_node(CallSemKind::callee, emitted_ctor.name);
    set_callsem_resolved_name(resolved_callee, function_output_name(emitted_ctor));
    resolved_callee.semantic_type = emitted_ctor.type;
    resolved_callee.is_c_linkage = emitted_ctor.is_c_linkage;
    set_dump_symbol(resolved_callee, emitted_ctor.symbol);
    result.node.children.push_back(resolved_callee);
    for(size_t i = 0; i < args.size(); ++i) {
      result.node.children.push_back(args[i].node);
    }
    return result;
  }

  bool try_argument_conversion(Scope & scope,
                               const TypePtr & target,
                               const ExprInfo & expr,
                               ExprInfo & out,
                               ConversionRank & rank,
                               const ArgumentConversionOptions & options =
                                   ArgumentConversionOptions())
  {
    DIAG_CONTEXT("try_argument_conversion [target " +
                 (target ? describe_type(target) : string("<null-type>")) + "]");
    return semantic_conversion::try_argument_conversion(
        *this,
        scope,
        target,
        expr,
        out,
        rank,
        semantic_policy::apply_analysis_policy(analysis_policy_, options));
  }

  ConversionRank implicit_object_conversion_rank(const TypePtr & target,
                                                 const ExprInfo & arg)
  {
    return semantic_conversion::implicit_object_conversion_rank(*this, target, arg);
  }

  ExprInfo apply_base_subobject_adjustment(const ExprInfo & expr,
                                           const TypePtr & adjusted_type,
                                           const ClassInfo & target_class,
                                           size_t offset) const
  {
    const auto find_actual_subobject_offset =
        [&](const ClassInfo & object_class,
            const ClassInfo * target,
            size_t & out_offset) -> bool
        {
          if(!target) {
            return false;
          }

          bool found = false;
          for(size_t i = 0; i < object_class.complete_subobjects.size(); ++i) {
            const SubobjectInfo & subobject = object_class.complete_subobjects[i];
            if(!subobject.type) {
              continue;
            }
            if(subobject.type != target &&
               subobject.type->qualified_name != target->qualified_name) {
              continue;
            }
            if(found && out_offset != subobject.offset) {
              throw logic_error("ambiguous complete subobject offset for " +
                                target->qualified_name);
            }
            out_offset = subobject.offset;
            found = true;
          }
          if(found) {
            return true;
          }

          MemberAccess access = MA_PUBLIC;
          return find_unique_base_path(object_class, target, out_offset, access);
        };
    const auto base_path_uses_virtual_inheritance =
        [&](const ClassInfo & source_class,
            const ClassInfo & target) -> bool
        {
          set<const ClassInfo *> visited;
          function<int(const ClassInfo &)> walk =
              [&](const ClassInfo & current) -> int
              {
                if(!visited.insert(&current).second) {
                  return -1;
                }
                for(size_t i = 0; i < current.bases.size(); ++i) {
                  const BaseInfo & base = current.bases[i];
                  if(!base.type) {
                    continue;
                  }
                  if(base.type == &target) {
                    return base.is_virtual ? 1 : 0;
                  }
                  const int nested = walk(*base.type);
                  if(nested >= 0) {
                    return (base.is_virtual || nested != 0) ? 1 : 0;
                  }
                }
                return -1;
              };
          return walk(source_class) > 0;
        };
    const auto find_virtual_base_root_for_target =
        [&](const ClassInfo & source_class,
            const ClassInfo & target,
            const ClassInfo *& out_root,
            size_t & out_root_offset) -> bool
        {
          set<const ClassInfo *> visited;
          function<bool(const ClassInfo &, const ClassInfo *&)> walk =
              [&](const ClassInfo & current, const ClassInfo *& first_virtual) -> bool
              {
                if(!visited.insert(&current).second) {
                  return false;
                }
                for(size_t i = 0; i < current.bases.size(); ++i) {
                  const BaseInfo & base = current.bases[i];
                  if(!base.type) {
                    continue;
                  }
                  const ClassInfo * next_virtual =
                      first_virtual ? first_virtual : (base.is_virtual ? base.type : nullptr);
                  if(base.type == &target) {
                    first_virtual = next_virtual;
                    return true;
                  }
                  const ClassInfo * nested_virtual = next_virtual;
                  if(walk(*base.type, nested_virtual)) {
                    first_virtual = nested_virtual;
                    return true;
                  }
                }
                return false;
              };

          const ClassInfo * first_virtual = nullptr;
          if(!walk(source_class, first_virtual) || !first_virtual) {
            return false;
          }
          out_root = first_virtual;
          if(first_virtual == &target) {
            out_root_offset = 0;
            return true;
          }
          return find_subobject_offset(*first_virtual, &target, out_root_offset);
        };
    bool is_virtual_base_subobject = false;
    const ClassInfo * virtual_base_root = nullptr;
    size_t virtual_base_root_offset = 0;
    CallSemVirtualBaseLayout inherited_virtual_base_layout =
        callsem_virtual_base_layout(expr.node);
    for(size_t i = 0; i < inherited_virtual_base_layout.size(); ++i) {
      inherited_virtual_base_layout[i].second += static_cast<unsigned long long>(offset);
    }
    TypePtr source_type = strip_top_level_cv(remove_reference_type(expr.type));
    if(source_type && source_type->kind == Type::TK_POINTER) {
      source_type = strip_top_level_cv(source_type->inner);
    }
    if(source_type) {
      ClassInfo * source_class = class_info_for_type(source_type);
      if(source_class && source_class != &target_class) {
        is_virtual_base_subobject =
            base_path_uses_virtual_inheritance(*source_class, target_class);
        if(is_virtual_base_subobject) {
          find_virtual_base_root_for_target(*source_class,
                                            target_class,
                                            virtual_base_root,
                                            virtual_base_root_offset);
        } else if(inherited_virtual_base_layout.empty() &&
                  !target_class.virtual_base_subobjects.empty()) {
          for(size_t i = 0; i < target_class.virtual_base_subobjects.size(); ++i) {
            const SubobjectInfo & subobject = target_class.virtual_base_subobjects[i];
            if(!subobject.type) {
              continue;
            }
            size_t actual_offset = subobject.offset;
            if(find_actual_subobject_offset(*source_class, subobject.type, actual_offset)) {
              inherited_virtual_base_layout.push_back(
                  make_pair(subobject.type->qualified_name,
                            static_cast<unsigned long long>(actual_offset)));
            }
          }
        }
      }
    }
    TypePtr base_type = strip_top_level_cv(expr.type);
    if(base_type && base_type->kind == Type::TK_POINTER) {
      ExprInfo result = make_base_pointer_expr(expr, target_class, offset);
      result.node.is_virtual_base_subobject = is_virtual_base_subobject;
      set_callsem_resolved_name(result.node, target_class.qualified_name);
      if(!inherited_virtual_base_layout.empty()) {
        set_callsem_virtual_base_layout(result.node, inherited_virtual_base_layout);
      } else if(virtual_base_root) {
        mutable_callsem_virtual_base_layout(result.node).push_back(
            make_pair(virtual_base_root->qualified_name,
                      static_cast<unsigned long long>(virtual_base_root_offset)));
      }
      if(result.node.kind == CallSemKind::unary_expression &&
         callsem_has_token(result.node, OP_AMP) &&
         result.node.children.size() == 1 &&
         result.node.children[0].kind == CallSemKind::member_expression) {
        result.node.children[0].is_base_subobject = true;
        result.node.children[0].is_virtual_base_subobject = is_virtual_base_subobject;
        set_callsem_resolved_name(result.node.children[0], target_class.qualified_name);
        set_callsem_virtual_base_layout(result.node.children[0],
                                        callsem_virtual_base_layout(result.node));
      }
      result.type = adjusted_type ? adjusted_type : make_pointer(target_class.type);
      result.category = VC_PRVALUE;
      set_expr_metadata(result.node, result.type, result.category);
      return result;
    }
    ExprInfo result = make_base_expr(expr, target_class, offset);
    result.node.is_virtual_base_subobject = is_virtual_base_subobject;
    set_callsem_resolved_name(result.node, target_class.qualified_name);
    if(!inherited_virtual_base_layout.empty()) {
      set_callsem_virtual_base_layout(result.node, inherited_virtual_base_layout);
    } else if(virtual_base_root) {
      mutable_callsem_virtual_base_layout(result.node).push_back(
          make_pair(virtual_base_root->qualified_name,
                    static_cast<unsigned long long>(virtual_base_root_offset)));
    }
    TypePtr adjusted_base = strip_top_level_cv(adjusted_type);
    if(adjusted_base &&
       (adjusted_base->kind == Type::TK_LVALUE_REFERENCE ||
        adjusted_base->kind == Type::TK_RVALUE_REFERENCE)) {
      result.type = adjusted_base->inner ? adjusted_base->inner : target_class.type;
    } else {
      result.type = adjusted_type ? adjusted_type : target_class.type;
    }
    result.category = expr.category;
    set_expr_metadata(result.node, result.type, result.category);
    return result;
  }

  bool try_apply_inheritance_conversion(const TypePtr & target,
                                        const ExprInfo & expr,
                                        ExprInfo & out)
  {
    return semantic_conversion::try_apply_inheritance_conversion(*this, target, expr, out);
  }

  template<typename Result, typename DirectLookup>
  Result lookup_unqualified_generic(Scope & scope,
                                    const string & name,
                                    const DirectLookup & direct_lookup)
  {
    return cpp_scope_lookup::lookup_unqualified<Result>(
        scope, name, direct_lookup, lookup_result_present<Result>);
  }

  template<typename Result, typename FinalLookup>
  Result lookup_qualified_generic(Scope & scope,
                                  const QualifiedName & qualified,
                                  const FinalLookup & final_lookup)
  {
    return cpp_scope_lookup::lookup_qualified<Result>(
        *root, qualified,
        [this, &scope](const string & name) -> Scope *
        {
          return lookup_unqualified_generic<Scope *>(
              scope, name,
              [](Scope & target, const string & lookup_name) -> Scope *
              {
                return resolve_direct_namespace(target, lookup_name);
              });
        },
        [](Scope & target, const string & lookup_name) -> Scope *
        {
          return resolve_direct_namespace(target, lookup_name);
        },
        final_lookup);
  }

  string describe_expression_for_diagnostic(const CppAstNode & node) const
  {
    return ::describe_expression_for_diagnostic(node);
  }

  string describe_scope_bindings_for_diagnostic(const Scope & scope) const
  {
    return ::describe_scope_bindings_for_diagnostic(scope);
  }

  vector<FunctionBinding *> lookup_qualified_functions(Scope & scope,
                                                       const QualifiedName & qualified)
  {
    if(!qualified.rooted && qualified.qualifiers.empty()) {
      return vector<FunctionBinding *>();
    }
    return lookup_qualified_generic<vector<FunctionBinding *> >(
        scope, qualified,
        [](Scope & target_scope, const string & lookup_name) -> vector<FunctionBinding *>
        {
          if(target_scope.class_info) {
            return lookup_direct_functions(target_scope, lookup_name);
          }
          vector<FunctionBinding *> out;
          semantic_lookup::lookup_functions_in_scopes(
              vector<Scope *>(1, &target_scope), lookup_name, out);
          return out;
        });
  }

  vector<FunctionTemplateDecl *> lookup_qualified_function_templates(
      Scope & scope,
      const QualifiedName & qualified)
  {
    if(!qualified.rooted && qualified.qualifiers.empty()) {
      return vector<FunctionTemplateDecl *>();
    }
    return lookup_qualified_generic<vector<FunctionTemplateDecl *> >(
        scope, qualified,
        [](Scope & target_scope, const string & lookup_name) -> vector<FunctionTemplateDecl *>
        {
          return semantic_lookup::lookup_direct_function_templates(target_scope, lookup_name);
        });
  }

  void collect_associated_namespace_scopes_for_type(const TypePtr & type,
                                                    vector<Scope *> & out)
  {
    semantic_lookup::collect_associated_namespace_scopes_for_type(*this, type, out);
  }

  ClassTemplateDecl * lookup_class_template(Scope & scope, const string & name)
  {
    return semantic_lookup::lookup_class_template(*this, scope, name);
  }

  ClassTemplateDecl * lookup_class_template(Scope & scope, const QualifiedName & name)
  {
    return semantic_lookup::lookup_class_template(*this, scope, name);
  }

  witness::TemplateWitnessSourceAnchor class_use_selected_decl_anchor(
      ClassTemplateDecl * class_template,
      const template_api::specialization::ClassSpecializationSelection & selection)
  {
    return callsemantic::class_use_selected_decl_anchor(
        *this, class_template, selection);
  }

  std::string value_binding_witness_entity(const ValueBinding & binding) const
  {
    if(binding.declaration_scope &&
       binding.declaration_scope->class_info &&
       !binding.declaration_scope->class_info->qualified_name.empty()) {
      return binding.declaration_scope->class_info->qualified_name + "::" +
          binding.name;
    }
    return binding.name;
  }

  std::string value_binding_witness_decl_location(
      const ValueBinding & binding) const
  {
    if(!binding.declaration_node) {
      return std::string();
    }
    std::string location =
        source_location_for_name_in_node(*binding.declaration_node,
                                         binding.name);
    if(location.empty()) {
      location = source_location_for_node(*binding.declaration_node);
    }
    return template_api::normalize_template_witness_source_location(location);
  }

  const ValueBinding * lookup_value_no_throw(Scope & scope,
                                             const std::string & name)
  {
    try {
      return lookup_value(scope, name);
    } catch(const std::logic_error &) {
      return nullptr;
    }
  }

  void collect_template_argument_value_references(
      Scope & scope,
      const CppAstNode & node,
      std::vector<std::string> & entities,
      std::vector<std::string> & decl_locations)
  {
    if(node.kind == CppAstKind::id_expression) {
      const ValueBinding * binding = lookup_value_no_throw(scope, node.value);
      if(binding &&
         binding->kind == ValueBinding::VK_VARIABLE &&
         binding->owner_class &&
         template_api::value_or_owner_has_template_identity(binding)) {
        const std::string entity = value_binding_witness_entity(*binding);
        if(!entity.empty() &&
           std::find(entities.begin(), entities.end(), entity) == entities.end()) {
          entities.push_back(entity);
        }
        const std::string decl_location =
            value_binding_witness_decl_location(*binding);
        if(!decl_location.empty() &&
           std::find(decl_locations.begin(), decl_locations.end(), decl_location) ==
               decl_locations.end()) {
          decl_locations.push_back(decl_location);
        }
      }
    }
    for(size_t i = 0; i < node.children.size(); ++i) {
      collect_template_argument_value_references(scope,
                                                node.children[i],
                                                entities,
                                                decl_locations);
    }
  }

  void collect_template_argument_value_references(
      Scope & scope,
      const TemplateArgumentSyntax & syntax,
      std::vector<std::string> & entities,
      std::vector<std::string> & decl_locations)
  {
    if(syntax.expression) {
      collect_template_argument_value_references(scope,
                                                *syntax.expression,
                                                entities,
                                                decl_locations);
    }
  }

  void collect_template_argument_value_references(
      Scope & scope,
      const TemplateArgument & argument,
      std::vector<std::string> & entities,
      std::vector<std::string> & decl_locations)
  {
    if(argument.expression) {
      collect_template_argument_value_references(scope,
                                                *argument.expression,
                                                entities,
                                                decl_locations);
    }
  }

  void fill_template_id_source_occurrence_argument_facts(
      Scope & scope,
      const std::vector<std::string> & source_arg_texts,
      const std::vector<TemplateArgumentSyntax> * arg_syntaxes,
      const std::vector<TemplateArgument> & arguments,
      semantic_source_use::SourceTemplateIdOccurrence & occurrence)
  {
    occurrence.in_template_body =
        scope_has_template_placeholders(scope) ||
        scope_is_inside_source_template_context(scope) ||
        witness::current_template_witness_entry_context().origin ==
            witness::TemplateWitnessOrigin::Closure;
    occurrence.has_dependent_argument =
        template_argument_texts_mention_source_bindings(*this,
                                                       scope,
                                                       source_arg_texts) ||
        template_argument_texts_mention_instantiated_class_local_type_aliases(
            scope,
            source_arg_texts) ||
        template_argument_texts_mention_template_bound_scope_names(
            scope,
            source_arg_texts) ||
        template_argument_texts_mention_enclosing_source_template_parameters(
            scope,
            source_arg_texts);
    occurrence.has_current_specialization_argument =
        template_argument_texts_mention_current_specialization_names(scope,
                                                                    source_arg_texts);
    for(size_t i = 0; i < occurrence.arguments.size(); ++i) {
      std::vector<std::string> arg_texts;
      arg_texts.push_back(occurrence.arguments[i].text);
      occurrence.arguments[i].dependent =
          template_argument_texts_mention_source_bindings(*this, scope, arg_texts) ||
          template_argument_texts_mention_instantiated_class_local_type_aliases(
              scope,
              arg_texts) ||
          template_argument_texts_mention_template_bound_scope_names(scope,
                                                                    arg_texts) ||
          template_argument_texts_mention_enclosing_source_template_parameters(
              scope,
              arg_texts);
      occurrence.arguments[i].current_specialization =
          template_argument_texts_mention_current_specialization_names(scope,
                                                                      arg_texts);
      if(occurrence.arguments[i].current_specialization &&
         occurrence.arguments[i].semantic_text.empty() &&
         arg_syntaxes &&
         i < arg_syntaxes->size()) {
        std::string semantic_text;
        if(current_specialization_source_argument_semantic_text(
               *this,
               scope,
               (*arg_syntaxes)[i],
               semantic_text)) {
          if(!callsemantic::template_argument_syntax_contains_template_id(
                 (*arg_syntaxes)[i])) {
            occurrence.arguments[i].semantic_text = semantic_text;
          }
        }
      }
      if((i < arguments.size() &&
          template_argument_preserves_qualified_member(arguments[i])) ||
         (arg_syntaxes &&
          i < arg_syntaxes->size() &&
          template_argument_syntax_preserves_qualified_member((*arg_syntaxes)[i]))) {
        occurrence.arguments[i].preserve_qualified_member = true;
      }
      if(arg_syntaxes && i < arg_syntaxes->size()) {
        collect_template_argument_value_references(
            scope,
            (*arg_syntaxes)[i],
            occurrence.arguments[i].referenced_value_entities,
            occurrence.arguments[i].referenced_value_decl_locations);
      }
      if(i < arguments.size() &&
         (occurrence.arguments[i].referenced_value_entities.empty() ||
          occurrence.arguments[i].referenced_value_decl_locations.empty())) {
        collect_template_argument_value_references(
            scope,
            arguments[i],
            occurrence.arguments[i].referenced_value_entities,
            occurrence.arguments[i].referenced_value_decl_locations);
      }
    }
  }

  bool build_class_use_source_decision_from_template_text(
      Scope & scope,
      const std::string & template_text,
      const std::string & event_location,
      witness::ClassUseSourceDecision & decision,
      const TemplateIdSyntax * source_syntax = nullptr)
  {
    const std::string effective_template_text =
        source_syntax ?
            template_id_syntax_text_preserving_spacing(*source_syntax) :
            template_text;
    const std::pair<const Scope *, std::string> cache_key(&scope,
                                                          effective_template_text);
    std::map<std::pair<const Scope *, std::string>, witness::ClassUseSourceDecision>::const_iterator
        cached = nested_class_use_source_decision_cache_.find(cache_key);
    if(cached != nested_class_use_source_decision_cache_.end()) {
      decision = cached->second;
      decision.location = event_location;
      if(!event_location.empty()) {
        witness::set_use_anchor(decision.location,
                                decision.use_anchor,
                                event_location);
        if(decision.template_id_occurrence.present) {
          decision.template_id_occurrence.name_anchor.location = event_location;
        }
      }
      return true;
    }

    QualifiedName template_id;
    vector<string> arg_texts;
    const vector<TemplateArgumentSyntax> * arg_syntaxes = nullptr;
    if(source_syntax && !source_syntax->name.name.empty()) {
      template_id = source_syntax->name;
      arg_texts = template_id_argument_texts_preserving_spacing(*source_syntax);
      arg_syntaxes = &source_syntax->argument_syntaxes;
    } else {
      if(!semantic_utils::split_top_level_template_id_text(effective_template_text,
                                                           template_id,
                                                           arg_texts)) {
        return false;
      }
    }
    if(template_argument_texts_mention_enclosing_source_template_parameters(
           scope,
           arg_texts)) {
      return false;
    }
    if(template_argument_texts_mention_template_bound_scope_names(scope,
                                                                 arg_texts)) {
      return false;
    }

    ClassTemplateDecl * class_template = lookup_class_template(scope, template_id);
    if(!class_template) {
      return false;
    }

    vector<TemplateArgument> arguments;
    if(!resolve_template_arguments(scope,
                                   class_template->parameters,
                                   arg_texts,
                                   arg_syntaxes,
                                   arguments,
                                   class_template->declaring_scope) ||
       template_arguments_are_dependent(arguments)) {
      return false;
    }
    canonicalize_simple_dependent_argument_texts(arguments);
    const std::string resolved_key = template_args_identity_key(arguments);
    const template_api::specialization::ClassSpecializationSelection selection =
        template_api::specialization::select_class_specialization(
            *this,
            *class_template,
            scope,
            resolved_key,
            arguments);
    std::map<std::string, ClassInfo *>::const_iterator existing_instantiation =
        class_template->instantiations.find(resolved_key);
    ClassInfo * existing_info =
        existing_instantiation != class_template->instantiations.end() ?
            existing_instantiation->second :
            nullptr;
    if(!existing_info) {
      std::map<std::string, ClassInfo *>::const_iterator existing_reference =
          class_template->reference_instantiations.find(resolved_key);
      if(existing_reference != class_template->reference_instantiations.end()) {
        existing_info = existing_reference->second;
      }
    }
    if(selection.kind == template_api::MS_PRIMARY &&
       existing_info &&
       (existing_info->template_instantiation_in_progress ||
        existing_info->full_member_collection_in_progress ||
        existing_info->reference_member_collection_in_progress)) {
      return false;
    }

    decision = witness::ClassUseSourceDecision();
    decision.location = event_location;
    if(!event_location.empty()) {
      witness::set_use_anchor(decision.location,
                              decision.use_anchor,
                              event_location);
    }
    decision.template_id_occurrence =
        witness::make_source_template_id_occurrence(event_location,
                                                    arg_texts);
    fill_template_id_source_occurrence_argument_facts(scope,
                                                     arg_texts,
                                                     arg_syntaxes,
                                                     arguments,
                                                     decision.template_id_occurrence);
    decision.template_name = class_template->declaring_scope ?
        scope_qualified_name(*class_template->declaring_scope,
                             class_template->name) :
        strip_trailing_top_level_template_arguments(effective_template_text);
    decision.selection = source_selection_kind_for_match_kind(selection.kind);
    witness::set_selected_decl_anchor(
        decision.selected_decl_location,
        decision.selected_decl_anchor,
        class_use_selected_decl_anchor(class_template, selection));
    template_api::append_template_witness_source_bindings(
        *this,
        decision.bindings,
        class_template->parameters,
        arguments,
        arg_texts,
        "explicit",
        "defaulted");
    if(selection.parameters && selection.parameters != &class_template->parameters) {
      template_api::append_template_witness_source_bindings(
          *this,
          decision.specialization_bindings,
          *selection.parameters,
          selection.arguments,
          "deduced");
    }
    witness::ClassUseSourceDecision prototype = decision;
    prototype.location.clear();
    prototype.use_anchor.location.clear();
    nested_class_use_source_decision_cache_[cache_key] = prototype;
    return true;
  }

  void emit_nested_class_use_source_events_from_location(
      Scope & scope,
      const std::string & location,
      witness::SourceUseOwnership ownership,
      const std::string & skip_exact_template_name) override
  {
    if(!witness::source_capture_enabled(template_witness_context())) {
      return;
    }
    const std::vector<NestedSourceTemplateIdOccurrence> occurrences =
        nested_template_id_source_occurrences_at_location(location);
    for(size_t i = 0; i < occurrences.size(); ++i) {
      if(!skip_exact_template_name.empty()) {
        const std::string occurrence_unqualified =
            unqualified_member_name(occurrences[i].template_name);
        const std::string skip_unqualified =
            unqualified_member_name(skip_exact_template_name);
        if(occurrences[i].template_name == skip_exact_template_name ||
           (!occurrence_unqualified.empty() &&
            occurrence_unqualified == skip_unqualified)) {
          continue;
        }
      }
      witness::ClassUseSourceDecision decision;
      if(!build_class_use_source_decision_from_template_text(
             scope,
             occurrences[i].text,
             occurrences[i].location,
             decision)) {
        continue;
      }
      witness::emit_class_use_decision(
          decision,
          ownership,
          witness::SourceUseRole::TypeUse,
          witness::nested_class_use_origin_for_ownership(ownership));
    }
  }

  void emit_nested_class_use_source_events_from_template_id_syntax(
      Scope & scope,
      const TemplateIdSyntax & syntax,
      witness::SourceUseOwnership ownership,
      const std::string & skip_exact_template_name)
  {
    if(syntax.name.name.empty()) {
      return;
    }
    if(!skip_exact_template_name.empty()) {
      const std::string syntax_unqualified =
          unqualified_member_name(syntax.name.name);
      const std::string skip_unqualified =
          unqualified_member_name(skip_exact_template_name);
      if(syntax.name.name == skip_exact_template_name ||
         (!syntax_unqualified.empty() &&
          syntax_unqualified == skip_unqualified)) {
        return;
      }
    }

    const std::string location =
        template_api::normalize_template_witness_source_location(
            template_api::template_witness_detail::source_location_for_location_id(
                template_witness_context(),
                syntax.source_location_id));
    witness::ClassUseSourceDecision decision;
    if(build_class_use_source_decision_from_template_text(scope,
                                                          std::string(),
                                                          location,
                                                          decision,
                                                          &syntax)) {
      witness::emit_class_use_decision(
          decision,
          ownership,
          witness::SourceUseRole::TypeUse,
          witness::nested_class_use_origin_for_ownership(ownership));
    }

    emit_nested_class_use_source_events_from_syntaxes(scope,
                                                      syntax.argument_syntaxes,
                                                      ownership,
                                                      skip_exact_template_name);
  }

  void emit_nested_class_use_source_events_from_ast_node(
      Scope & scope,
      const CppAstNode & node,
      witness::SourceUseOwnership ownership,
      const std::string & skip_exact_template_name)
  {
    if(const TemplateIdSyntax * syntax = cppast_template_id_syntax(node)) {
      emit_nested_class_use_source_events_from_template_id_syntax(
          scope,
          *syntax,
          ownership,
          skip_exact_template_name);
    }
    if(const CppAstNode * conversion_type_id =
           cppast_conversion_type_id_syntax(node)) {
      emit_nested_class_use_source_events_from_ast_node(scope,
                                                        *conversion_type_id,
                                                        ownership,
                                                        skip_exact_template_name);
    }
    for(size_t i = 0; i < node.qualifier_template_id_syntaxes.size(); ++i) {
      emit_nested_class_use_source_events_from_template_id_syntax(
          scope,
          node.qualifier_template_id_syntaxes[i],
          ownership,
          skip_exact_template_name);
    }
    for(size_t i = 0; i < node.children.size(); ++i) {
      emit_nested_class_use_source_events_from_ast_node(scope,
                                                        node.children[i],
                                                        ownership,
                                                        skip_exact_template_name);
    }
  }

  void emit_nested_class_use_source_events_from_syntaxes(
      Scope & scope,
      const std::vector<TemplateArgumentSyntax> & syntaxes,
      witness::SourceUseOwnership ownership,
      const std::string & skip_exact_template_name)
  {
    if(!witness::source_capture_enabled(template_witness_context())) {
      return;
    }
    for(size_t i = 0; i < syntaxes.size(); ++i) {
      if(syntaxes[i].template_id) {
        emit_nested_class_use_source_events_from_template_id_syntax(
            scope,
            *syntaxes[i].template_id,
            ownership,
            skip_exact_template_name);
      }
      if(syntaxes[i].type_id) {
        emit_nested_class_use_source_events_from_ast_node(scope,
                                                          *syntaxes[i].type_id,
                                                          ownership,
                                                          skip_exact_template_name);
      }
      if(syntaxes[i].expression) {
        emit_nested_class_use_source_events_from_ast_node(scope,
                                                          *syntaxes[i].expression,
                                                          ownership,
                                                          skip_exact_template_name);
      }
    }
  }

  void emit_nested_class_use_source_events_from_location(
      Scope & scope,
      const std::string & location,
      witness::SourceUseOwnership ownership)
  {
    emit_nested_class_use_source_events_from_location(scope,
                                                      location,
                                                      ownership,
                                                      std::string());
  }

  void emit_nested_alias_use_source_events_from_location(
      Scope & scope,
      const std::string & location,
      const std::string & skip_exact_template_name,
      const vector<TemplateArgumentSyntax> * source_arg_syntaxes = nullptr)
  {
    if(!witness::source_capture_enabled(template_witness_context())) {
      return;
    }
    const std::vector<NestedSourceTemplateIdOccurrence> occurrences =
        nested_template_id_source_occurrences_at_location(location, false);
    for(size_t i = 0; i < occurrences.size(); ++i) {
      if(!skip_exact_template_name.empty()) {
        const std::string occurrence_unqualified =
            unqualified_member_name(occurrences[i].template_name);
        const std::string skip_unqualified =
            unqualified_member_name(skip_exact_template_name);
        if(occurrences[i].template_name == skip_exact_template_name ||
           (!occurrence_unqualified.empty() &&
            occurrence_unqualified == skip_unqualified)) {
          continue;
        }
      }
      QualifiedName template_id;
      vector<string> arg_texts;
      if(!semantic_utils::split_top_level_template_id_text(occurrences[i].text,
                                                            template_id,
                                                            arg_texts)) {
        continue;
      }
      AliasTemplateDecl * alias_template =
          lookup_alias_template(scope, template_id);
      if(!alias_template) {
        continue;
      }
      const TemplateIdSyntax * syntax = nullptr;
      if(source_arg_syntaxes) {
        syntax =
            template_id_syntax_for_anchor_at_or_after_location(
                template_witness_context(),
                *source_arg_syntaxes,
                occurrences[i].template_name,
                occurrences[i].location);
        if(syntax) {
          arg_texts = template_id_argument_witness_source_texts(*syntax);
        }
      }
      if(!syntax) {
        continue;
      }
      const ScopedTemplateUseLocation use_location_guard(
          occurrences[i].location);
      try {
        instantiate_alias_template_with_syntax(
            *alias_template,
            scope,
            arg_texts,
            syntax ? &syntax->argument_syntaxes : nullptr,
            true);
      } catch(...) {
        semantic_fallback_audit::rethrow_if_strict_fallback();
      }
    }
  }

  AliasTemplateDecl * lookup_alias_template(Scope & scope, const string & name)
  {
    return semantic_lookup::lookup_alias_template(*this, scope, name);
  }

  AliasTemplateDecl * lookup_alias_template(Scope & scope, const QualifiedName & name)
  {
    return semantic_lookup::lookup_alias_template(*this, scope, name);
  }

  vector<FunctionTemplateDecl *> lookup_function_templates(Scope & scope, const string & name)
  {
    return semantic_lookup::lookup_function_templates(*this, scope, name);
  }

  vector<FunctionTemplateDecl *> lookup_function_templates_node(Scope & scope,
                                                                const CppAstNode & node,
                                                                const string & name)
  {
    const QualifiedName * qualified = cppast_qualified_name_syntax(node);
    if(!qualified ||
       (!qualified->rooted && qualified->qualifiers.empty()) ||
       !node_has_template_id_qualifier_syntax(node)) {
      if(const TemplateIdSyntax * template_id = cppast_template_id_syntax(node)) {
        if(!template_id->name.name.empty()) {
          return lookup_function_templates(scope, template_id->name);
        }
      }
      if(qualified && (qualified->rooted || !qualified->qualifiers.empty())) {
        return lookup_function_templates(scope, *qualified);
      }
      return lookup_function_templates(scope, name);
    }
    const TemplateIdSyntax * leaf_template_id = cppast_template_id_syntax(node);
    const string leaf_name =
        leaf_template_id && !leaf_template_id->name.name.empty() ?
            leaf_template_id->name.name :
            strip_trailing_top_level_template_arguments(qualified->name);

    Scope * current_scope = &scope;
    if(qualified->rooted) {
      current_scope = root.get();
    }

    for(size_t i = 0; i < qualified->qualifiers.size(); ++i) {
      Scope * namespace_scope =
          resolve_direct_namespace(*current_scope, qualified->qualifiers[i]);
      if(namespace_scope) {
        current_scope = namespace_scope;
        continue;
      }

      const TemplateIdSyntax * qualifier_template_id =
          i < node.qualifier_template_id_syntaxes.size() &&
                  !node.qualifier_template_id_syntaxes[i].name.name.empty() ?
              &node.qualifier_template_id_syntaxes[i] :
              nullptr;
      const string qualifier_lookup_text =
          qualifier_template_id ?
              template_id_syntax_text_preserving_spacing(*qualifier_template_id) :
              qualified->qualifiers[i];
      TypePtr qualifier_type;
      if(qualifier_template_id) {
        ExactTemplateTypeLookupAnchor qualifier_anchor;
        qualifier_anchor.template_text =
            template_id_syntax_text_preserving_spacing(*qualifier_template_id);
        qualifier_anchor.identifier =
            template_lookup_fragment_identifier(qualifier_anchor.template_text);
        if(qualifier_anchor.identifier.empty()) {
          qualifier_anchor.identifier = qualifier_template_id->name.name;
        }
        qualifier_anchor.compact_key =
            compact_lookup_text(qualifier_anchor.template_text);
        qualifier_anchor.arg_texts = qualifier_template_id->arguments;
        qualifier_anchor.arg_syntaxes = qualifier_template_id->argument_syntaxes;
        qualifier_anchor.has_argument_list = true;
        qualifier_anchor.location =
            template_api::normalize_template_witness_source_location(
                template_api::template_witness_detail::source_location_for_location_id(
                    template_witness_context(),
                    qualifier_template_id->source_location_id));
        if(qualifier_anchor.location.empty()) {
          qualifier_anchor.location =
              template_api::normalize_template_witness_source_location(
                  source_location_for_name_in_subtree(
                      node,
                      unqualified_member_name(qualifier_template_id->name.name)));
        }
        const ScopedExactTemplateTypeLookupAnchor qualifier_anchor_guard(
            qualifier_anchor);
        qualifier_type = lookup_type_impl(*current_scope,
                                          qualifier_lookup_text,
                                          true,
                                          true);
      } else {
        qualifier_type = lookup_type_impl(*current_scope,
                                          qualifier_lookup_text,
                                          true,
                                          true);
      }

      if(!qualifier_type || type_depends_on_template_parameter(qualifier_type)) {
        return lookup_function_templates(scope, name);
      }
      ClassInfo * qualifier_info = class_info_for_type(qualifier_type);
      if(!qualifier_info || !qualifier_info->member_scope) {
        qualifier_info = complete_class_type(qualifier_type);
      }
      if(!qualifier_info || !qualifier_info->member_scope) {
        return lookup_function_templates(scope, name);
      }
      current_scope = qualifier_info->member_scope.get();
    }

    vector<FunctionTemplateDecl *> out;
    if(current_scope->class_info) {
      MemberFunctionTemplateLookupResult result =
          semantic_lookup::lookup_visible_member_function_templates(
              *current_scope->class_info,
              leaf_name);
      semantic_lookup::append_unique_function_templates(out, result.templates);
      if(!out.empty()) {
        return out;
      }
    }
    semantic_lookup::collect_direct_function_templates(*current_scope,
                                                       leaf_name,
                                                       out);
    return out;
  }

  vector<FunctionTemplateDecl *> lookup_function_templates(Scope & scope,
                                                           const QualifiedName & name)
  {
    vector<FunctionTemplateDecl *> out;
    semantic_lookup::collect_function_templates(*this, scope, name, out);
    return out;
  }

  VariableTemplateDecl * lookup_variable_template(Scope & scope, const string & name)
  {
    return semantic_lookup::lookup_variable_template(*this, scope, name);
  }

  VariableTemplateDecl * lookup_variable_template(Scope & scope, const QualifiedName & name)
  {
    return semantic_lookup::lookup_variable_template(*this, scope, name);
  }

  template<typename Result, typename FinalLookup>
  Result lookup_qualified_class_or_namespace_generic(Scope & scope,
                                                     const QualifiedName & qualified,
                                                     const FinalLookup & final_lookup)
  {
    Scope * target =
        semantic_lookup::resolve_qualified_scope_for_class_or_namespace(
            *this, scope, qualified);
    if(!target) {
      return Result();
    }
    return final_lookup(*target, qualified.name);
  }

  string describe_static_assert_lookup_for_diagnostic(Scope & scope,
                                                      const CppAstNode & node)
  {
    const CppAstNode * target = &node;
    if(node.kind == CppAstKind::unary_expression &&
       node.children.size() == 1 &&
       node.value == "!") {
      target = &node.children[0];
    }

    if(target->kind != CppAstKind::id_expression) {
      return string();
    }

    const QualifiedName * qualified = cppast_qualified_name_syntax(*target);
    if(!qualified || (!qualified->rooted && qualified->qualifiers.empty())) {
      return string();
    }

    Scope * resolved =
        resolve_qualified_scope_for_node(scope, *qualified, *target, false);
    if(!resolved) {
      return string();
    }

    ostringstream out;
    out << target->value;
    if(resolved->class_info) {
      out << " target=" << resolved->class_info->qualified_name;
      MemberValueLookupResult member =
          lookup_member_value(*resolved->class_info, qualified->name);
      if(member.binding) {
        out << " declared_in=" << member.declared_in->qualified_name;
        if(member.binding->has_constant_value) {
          out << " value=" << member.binding->constant_value;
        }
        return out.str();
      }
    }

    map<string, ValueBinding>::const_iterator found = resolved->values.find(qualified->name);
    if(found != resolved->values.end()) {
      if(found->second.has_constant_value) {
        out << " value=" << found->second.constant_value;
      }
      return out.str();
    }

    return string();
  }

  TypePtr instantiate_alias_template(AliasTemplateDecl & decl,
                                     Scope & use_scope,
                                     const vector<string> & arg_texts,
                                     bool reference_class_templates_only = false)
  {
    return instantiate_alias_template_with_syntax(
        decl, use_scope, arg_texts, nullptr, reference_class_templates_only);
  }

  TypePtr instantiate_alias_template_with_syntax(
      AliasTemplateDecl & decl,
      Scope & use_scope,
      const vector<string> & arg_texts,
      const vector<TemplateArgumentSyntax> * arg_syntaxes,
      bool reference_class_templates_only = false,
      bool suppress_source_capture = false)
  {
    vector<TemplateArgument> arguments;
    if(!resolve_template_arguments(
           use_scope, decl.parameters, arg_texts, arg_syntaxes, arguments, decl.declaring_scope)) {
      if(!scope_has_template_placeholders(use_scope)) {
        const bool verbose_bindings = parser_trace::enabled("template.resolve");
        ostringstream out;
        out << "failed alias template argument resolution";
        if(!decl.name.empty()) {
          out << " for " << decl.name;
        }
        if(!arguments.empty()) {
          out << " [args";
          for(size_t i = 0; i < arguments.size(); ++i) {
            out << (i == 0 ? " " : ", ") << template_argument_text(arguments[i]);
          }
          out << "]";
        }
        out << " [use-scope " << scope_qualified_name(use_scope, "<here>") << "]";
        if(verbose_bindings) {
          out << " [use-bindings " << describe_scope_bindings_for_diagnostic(use_scope) << "]";
        }
        if(decl.declaring_scope) {
          out << " [decl-scope "
              << scope_qualified_name(*decl.declaring_scope, "<here>") << "]";
          if(verbose_bindings) {
            out << " [decl-bindings "
                << describe_scope_bindings_for_diagnostic(*decl.declaring_scope) << "]";
          }
        }
        throw_substitution_failure(out.str(), std::string(), "callsemantic");
      }
      return TypePtr();
    }

    vector<string> source_arg_texts = arg_texts;
    if(arg_syntaxes) {
      const size_t limit = std::min(source_arg_texts.size(),
                                    arg_syntaxes->size());
      for(size_t i = 0; i < limit; ++i) {
        const string source_text =
            template_argument_syntax_witness_source_text((*arg_syntaxes)[i]);
        if(!source_text.empty()) {
          source_arg_texts[i] = source_text;
        }
      }
    }
    return instantiate_resolved_alias_template_impl(
        decl,
        use_scope,
        arguments,
        &source_arg_texts,
        arg_syntaxes,
        reference_class_templates_only,
        suppress_source_capture);
  }

  TypePtr instantiate_resolved_alias_template(
      AliasTemplateDecl & decl,
      Scope & use_scope,
      const vector<TemplateArgument> & resolved_arguments,
      bool reference_class_templates_only = false)
  {
    return instantiate_resolved_alias_template_impl(
        decl, use_scope, resolved_arguments, nullptr, nullptr, reference_class_templates_only);
  }

  TypePtr instantiate_resolved_alias_template_impl(
      AliasTemplateDecl & decl,
      Scope & use_scope,
      const vector<TemplateArgument> & resolved_arguments,
      const vector<string> * source_arg_texts,
      const vector<TemplateArgumentSyntax> * source_arg_syntaxes,
      bool reference_class_templates_only = false,
      bool suppress_source_capture = false)
  {
    map<string, TypePtr> & instantiations =
        reference_class_templates_only ? decl.reference_instantiations : decl.instantiations;

    vector<TemplateArgument> arguments = resolved_arguments;
    canonicalize_simple_dependent_argument_texts(arguments);
    const string key = template_args_identity_key(arguments);
    const bool dependent_arguments = template_arguments_are_dependent(arguments);
    const auto note_alias_use = [&](const TypePtr & resolved_alias) -> void
    {
      const std::string raw_use_location = parser_trace::current_use_location();
      const bool trace_enabled = parser_trace::enabled("template.resolve");
      const bool source_capture_enabled =
          witness::source_capture_enabled(template_witness_context());
      if(raw_use_location.empty() || !resolved_alias) {
        return;
      }
      if(!trace_enabled && !source_capture_enabled) {
        return;
      }
      const bool exact_identifier_location =
          semantic_trace::source_location_points_at_identifier(raw_use_location,
                                                               decl.name);
      std::string source_event_location = raw_use_location;
      if(const ExactTemplateTypeLookupAnchor * anchor =
             current_exact_template_type_lookup_anchor()) {
        if(exact_template_type_lookup_anchor_matches_identifier(*anchor,
                                                                decl.name) &&
           semantic_trace::source_location_points_at_identifier(anchor->location,
                                                                decl.name)) {
          source_event_location = anchor->location;
        }
      }
      if(source_event_location == raw_use_location && !exact_identifier_location) {
        const std::string same_line_location =
            template_api::template_witness_detail::
                source_location_for_identifier_token_on_or_after(
                template_witness_context(),
                raw_use_location,
                decl.name,
                true,
                true);
        if(semantic_trace::source_location_points_at_identifier(same_line_location,
                                                                decl.name)) {
          source_event_location = same_line_location;
        }
      }
      const bool exact_source_event_location =
          semantic_trace::source_location_points_at_identifier(source_event_location,
                                                               decl.name);
      const bool suppress_member_alias_source_capture =
          decl.declaring_scope &&
          decl.declaring_scope->class_info &&
          !exact_source_event_location;
      if(witness::enabled(template_witness_context()) &&
         exact_source_event_location &&
         !suppress_member_alias_source_capture &&
         !suppress_source_capture) {
        witness::AliasUseEmitRequest request;
        request.use_location = source_event_location;
        vector<string> witness_source_arg_texts;
        if(source_arg_texts) {
          witness_source_arg_texts = *source_arg_texts;
          template_argument_semantics::
              canonicalize_alias_template_source_argument_texts(
                  decl.parameters,
                  witness_source_arg_texts);
          request.template_id_occurrence =
              witness::make_source_template_id_occurrence(source_event_location,
                                                          witness_source_arg_texts);
          fill_template_id_source_occurrence_argument_facts(
              use_scope,
              witness_source_arg_texts,
              source_arg_syntaxes,
              arguments,
              request.template_id_occurrence);
          template_api::with_template_services(
              *this,
              [&](template_api::TemplateServices & services)
              {
                template_argument_semantics::
                    mark_alias_template_value_owner_argument_facts(
                        services,
                        &use_scope,
                        decl,
                        arguments,
                        source_arg_syntaxes,
                        request.template_id_occurrence);
              });
        }
        request.template_name =
            template_api::alias_template_witness_entity(&decl);
        request.origin =
            witness::AliasUseEmissionOrigin::QualifiedSourceTemplateId;
        const semantic_model::SourceDeclAnchorCache & decl_anchor =
            semantic_trace::alias_template_decl_anchor(*this, &decl);
        witness::set_selected_decl_anchor(request.selected_decl_location,
                                          request.selected_decl_anchor,
                                          decl_anchor);
        request.selected_decl_anchor_explicit = true;
        {
          const ParsedSourceLocation parsed_decl =
              parse_source_location(request.selected_decl_location);
          if(parsed_decl.valid) {
            std::ostringstream line_start;
            line_start << parsed_decl.file << ":" << parsed_decl.line << ":1";
            request.selected_decl_location = line_start.str();
          }
        }
        request.expanded_to = describe_type(resolved_alias);
        if(source_arg_texts) {
          template_api::append_template_witness_source_bindings(
              *this,
              request.bindings,
              decl.parameters,
              arguments,
              witness_source_arg_texts,
              "explicit",
              "defaulted",
              false);
          rewrite_current_specialization_alias_binding_texts(*this,
                                                             use_scope,
                                                             decl.parameters,
                                                             arguments,
                                                             witness_source_arg_texts,
                                                             source_arg_syntaxes,
                                                             request.bindings,
                                                             &request.template_id_occurrence);
          mark_qualified_member_source_bindings(request.bindings,
                                                arguments,
                                                source_arg_syntaxes);
        } else {
          template_api::append_template_witness_source_bindings(
              *this,
              request.bindings,
              decl.parameters,
              arguments,
              "explicit");
          vector<string> occurrence_args;
          occurrence_args.reserve(request.bindings.size());
          for(size_t i = 0; i < request.bindings.size(); ++i) {
            if(request.bindings[i].source == "explicit") {
              occurrence_args.push_back(request.bindings[i].arg);
            }
          }
          template_argument_semantics::
              canonicalize_alias_template_source_argument_texts(
                  decl.parameters,
                  occurrence_args);
          if(!occurrence_args.empty()) {
            request.template_id_occurrence =
                witness::make_source_template_id_occurrence(
                    source_event_location,
                    occurrence_args);
            fill_template_id_source_occurrence_argument_facts(
                use_scope,
                occurrence_args,
                nullptr,
                arguments,
                request.template_id_occurrence);
            template_api::with_template_services(
                *this,
                [&](template_api::TemplateServices & services)
                {
                  template_argument_semantics::
                      mark_alias_template_value_owner_argument_facts(
                          services,
                          &use_scope,
                          decl,
                          arguments,
                          nullptr,
                          request.template_id_occurrence);
                });
          }
        }
        for(size_t i = 0; i < decl.parameters.size() &&
                           i < request.bindings.size(); ++i) {
          if(!decl.parameters[i].parameter_pack) {
            continue;
          }
          string & arg = request.bindings[i].arg;
          if(arg.size() >= 2 &&
             arg.front() == '<' &&
             arg.back() == '>') {
            const string inner = trim_space(arg.substr(1, arg.size() - 2));
            if(!inner.empty() && !has_top_level_comma(inner)) {
              arg = inner;
            }
          }
        }
        witness::emit_alias_use(template_witness_context(), request);
        emit_nested_alias_use_source_events_from_location(
            use_scope,
            request.use_location,
            request.template_name,
            source_arg_syntaxes);
        emit_nested_class_use_source_events_from_location(use_scope,
                                                          request.use_location,
                                                          witness::SourceUseOwnership::SourceOwned,
                                                          request.template_name);
      }

      if(trace_enabled) {
        std::ostringstream bindings;
        for(size_t i = 0; i < decl.parameters.size() && i < arguments.size(); ++i) {
          if(i != 0) {
            bindings << ",";
          }
          bindings << (decl.parameters[i].name.empty() ?
                           std::string("$") + std::to_string(i + 1) :
                           decl.parameters[i].name)
                   << ":" << template_argument_text(arguments[i]);
        }

        std::ostringstream trace;
        trace << "alias-use"
              << " name=" << decl.name
              << " resolved={" << describe_type(resolved_alias) << "}"
              << " decl="
              << (decl.type_id ? source_location_for_node(*decl.type_id) :
                                 std::string("<none>"))
              << " bindings={" << bindings.str() << "}";
        parser_trace::note("template.resolve", raw_use_location, trace.str());
      }
    };
    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "instantiate-alias-template name=" << decl.name
            << " key=" << key
            << " use-scope=" << scope_qualified_name(use_scope, "<here>")
            << " decl-scope="
            << (decl.declaring_scope ?
                    scope_qualified_name(*decl.declaring_scope, "<here>") :
                    std::string("<none>"))
            << " arg-count=" << arguments.size()
            << " dependent=" << (dependent_arguments ? "yes" : "no");
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
    map<string, TypePtr>::iterator found = instantiations.find(key);

    Scope * inst_scope =
        &bind_template_arguments_for_instantiation(*decl.declaring_scope,
                                                   use_scope,
                                                   decl.parameters,
                                                   arguments);
    const auto make_dependent_alias_specialization = [&]() -> TypePtr
    {
      vector<string> dependent_source_arg_texts =
          source_arg_texts ? *source_arg_texts : vector<string>();
      vector<TemplateArgumentSyntax> dependent_source_arg_syntaxes;
      const vector<TemplateArgumentSyntax> * dependent_source_arg_syntaxes_ptr =
          source_arg_syntaxes;
      if(source_arg_texts && source_arg_texts->size() != arguments.size()) {
        template_argument_semantics::ExpandedTemplateArgumentInputs
            expanded_source_args =
                template_api::with_template_services(
                    *this,
                    [&](template_api::TemplateServices & services)
                    {
                      return template_argument_semantics::
                          expand_template_argument_inputs(services,
                                                          *inst_scope,
                                                          *source_arg_texts,
                                                          source_arg_syntaxes);
                    });
        if(expanded_source_args.texts.size() == arguments.size()) {
          dependent_source_arg_texts = expanded_source_args.texts;
          if(source_arg_syntaxes) {
            dependent_source_arg_syntaxes.reserve(
                expanded_source_args.texts.size());
            for(size_t i = 0; i < expanded_source_args.texts.size(); ++i) {
              if(const TemplateArgumentSyntax * syntax =
                     expanded_source_args.syntax_for(i)) {
                dependent_source_arg_syntaxes.push_back(*syntax);
              } else {
                TemplateArgumentSyntax fallback_syntax;
                fallback_syntax.text = dependent_source_arg_texts[i];
                dependent_source_arg_syntaxes.push_back(fallback_syntax);
              }
            }
            dependent_source_arg_syntaxes_ptr =
                &dependent_source_arg_syntaxes;
          }
        }
      }

      vector<size_t> stored_indices;
      vector<TemplateArgument> stored_arguments;
      stored_indices.reserve(arguments.size());
      stored_arguments.reserve(arguments.size());
      for(size_t i = 0; i < arguments.size(); ++i) {
        if(arguments[i].source_defaulted) {
          continue;
        }
        stored_indices.push_back(i);
        stored_arguments.push_back(arguments[i]);
      }
      const string specialization_name =
          make_template_specialization_name(decl.name, stored_arguments);
      vector<DependentAliasTemplateArgumentSyntax> dependent_arguments;
      dependent_arguments.reserve(stored_indices.size());
      for(size_t i = 0; i < stored_indices.size(); ++i) {
        const size_t source_index = stored_indices[i];
        DependentAliasTemplateArgumentSyntax dependent_argument;
        dependent_argument.text =
            source_index < dependent_source_arg_texts.size() ?
                trim_space(dependent_source_arg_texts[source_index]) :
                template_argument_text(arguments[source_index]);
        if(arguments[source_index].kind == TemplateArgument::TA_TYPE) {
          dependent_argument.type = arguments[source_index].type;
        }
        if(dependent_source_arg_syntaxes_ptr &&
           source_index < dependent_source_arg_syntaxes_ptr->size()) {
          dependent_argument.syntax =
              (*dependent_source_arg_syntaxes_ptr)[source_index];
        } else {
          dependent_argument.syntax.text = dependent_argument.text;
        }
        if(arguments[source_index].kind == TemplateArgument::TA_VALUE &&
           arguments[source_index].expression &&
           !dependent_argument.syntax.expression) {
          dependent_argument.syntax.text = dependent_argument.text;
          dependent_argument.syntax.source_location_id =
              arguments[source_index].expression->source_location_id;
          dependent_argument.syntax.expression.reset(
              new CppAstNode(*arguments[source_index].expression));
        }
        dependent_arguments.push_back(dependent_argument);
      }
      return make_dependent_alias_type(
          specialization_name,
          scope_qualified_name(*decl.declaring_scope, specialization_name),
          &decl,
          dependent_arguments);
    };
    TypePtr alias;
    auto parse_instantiated_alias_text_without_source_capture =
        [&](const string & text, TypePtr & out) -> bool
    {
      const witness::ScopedTemplateWitnessSourceCapturePause
          source_capture_pause;
      return parse_type_text_scoped(*inst_scope,
                                    *inst_scope,
                                    text,
                                    out,
                                    reference_class_templates_only);
    };
	    auto resolve_nested_alias_type_id_syntax =
	        [&](TypePtr & out) -> bool
	    {
	      out.reset();
	      if(!decl.type_id) {
	        return false;
	      }
	      const TemplateIdSyntax * syntax =
	          first_template_id_syntax_in_subtree(*decl.type_id);
	      if(!syntax || syntax->name.name.empty()) {
	        return false;
	      }
	      AliasTemplateDecl * nested_alias =
	          lookup_alias_template(*inst_scope, syntax->name);
	      if(!nested_alias || nested_alias == &decl) {
	        return false;
	      }
	      const vector<string> arg_texts =
	          template_id_argument_texts_preserving_spacing(*syntax);
	      const witness::ScopedTemplateWitnessSourceCapturePause
	          source_capture_pause;
      out = instantiate_alias_template_with_syntax(
          *nested_alias,
          *inst_scope,
          arg_texts,
          &syntax->argument_syntaxes,
          reference_class_templates_only);
      return out && !type_depends_on_template_parameter(out);
	    };
	    auto resolve_direct_alias_type_id_syntax =
	        [&](TypePtr & out) -> bool
	    {
	      out.reset();
	      if(!decl.type_id) {
	        return false;
	      }
	      const TemplateIdSyntax * syntax =
	          first_template_id_syntax_in_subtree(*decl.type_id);
	      if(!syntax || syntax->name.name.empty()) {
	        return false;
	      }
	      const auto compact =
	          [](const string & text) -> string
	      {
	        string out;
	        out.reserve(text.size());
	        for(size_t i = 0; i < text.size(); ++i) {
	          if(!isspace(static_cast<unsigned char>(text[i]))) {
	            out.push_back(text[i]);
	          }
	        }
	        return out;
	      };
	      if(compact(spaced_node_text(*decl.type_id)) !=
	         compact(template_id_syntax_text_preserving_spacing(*syntax))) {
	        return false;
	      }
	      AliasTemplateDecl * nested_alias =
	          lookup_alias_template(*inst_scope, syntax->name);
	      if(!nested_alias || nested_alias == &decl) {
	        return false;
	      }
	      const vector<string> arg_texts =
	          template_id_argument_texts_preserving_spacing(*syntax);
	      const witness::ScopedTemplateWitnessSourceCapturePause
	          source_capture_pause;
	      out = instantiate_alias_template_with_syntax(
	          *nested_alias,
	          *inst_scope,
	          arg_texts,
	          &syntax->argument_syntaxes,
	          reference_class_templates_only);
	      return out && !type_depends_on_template_parameter(out);
	    };
		    auto refine_instantiated_alias = [&](TypePtr candidate) -> TypePtr
		    {
		      if(!candidate || !type_depends_on_template_parameter(candidate)) {
		        return candidate;
	      }

      TypePtr resolved_alias;
      if(resolve_instantiated_dependent_type(*inst_scope, candidate, resolved_alias) &&
         resolved_alias &&
         !type_depends_on_template_parameter(resolved_alias)) {
        return resolved_alias;
      }

      TypePtr nested_syntax_alias;
      if(resolve_nested_alias_type_id_syntax(nested_syntax_alias)) {
        return nested_syntax_alias;
      }
      return candidate;
	    };
	    auto alias_mentions_instantiation_bindings =
	        [&](const TypePtr & candidate) -> bool
	    {
	      if(!candidate) {
	        return false;
	      }
	      const string alias_text =
	          candidate->named_display.empty() ? describe_type(candidate) :
	                                             candidate->named_display;
	      return text_mentions_template_placeholders(*inst_scope, alias_text) ||
	             text_mentions_dependent_non_namespace_binding_names(*inst_scope,
	                                                                 alias_text);
	    };
	    bool alias_has_parameter_pack = false;
	    bool alias_has_non_type_parameter = false;
	    for(size_t i = 0; i < decl.parameters.size(); ++i) {
	      if(decl.parameters[i].parameter_pack) {
	        alias_has_parameter_pack = true;
	      }
	      if(decl.parameters[i].kind != TemplateParameterInfo::TP_TYPE) {
	        alias_has_non_type_parameter = true;
	      }
	    }
	    if(!dependent_arguments && !alias_has_parameter_pack && decl.resolved_type_pattern) {
	      TypePtr direct_syntax_alias;
	      if(resolve_direct_alias_type_id_syntax(direct_syntax_alias)) {
	        instantiations[key] = direct_syntax_alias;
	        if(parser_trace::enabled("template.resolve")) {
	          std::ostringstream trace;
	          trace << "instantiate-alias-template-direct-syntax name=" << decl.name
	                << " key=" << key
	                << " alias=" << describe_type(direct_syntax_alias);
	          parser_trace::note("template.resolve", std::string(), trace.str());
	        }
	        note_alias_use(direct_syntax_alias);
	        return direct_syntax_alias;
	      }
	      TypePtr structural_alias = decl.resolved_type_pattern;
	      {
	        const witness::ScopedTemplateWitnessSourceCapturePause
	            source_capture_pause;
	        TypePtr resolved_structural_alias;
	        if(resolve_instantiated_dependent_type(*inst_scope,
	                                               structural_alias,
	                                               resolved_structural_alias) &&
	           resolved_structural_alias) {
	          structural_alias = resolved_structural_alias;
	        }
	        structural_alias = refine_instantiated_alias(structural_alias);
	      }
	      if(structural_alias &&
	         !type_depends_on_template_parameter(structural_alias) &&
	         !alias_mentions_instantiation_bindings(structural_alias)) {
	        instantiations[key] = structural_alias;
	        if(parser_trace::enabled("template.resolve")) {
	          std::ostringstream trace;
	          trace << "instantiate-alias-template-structural name=" << decl.name
	                << " key=" << key
	                << " alias=" << describe_type(structural_alias);
	          parser_trace::note("template.resolve", std::string(), trace.str());
	        }
	        note_alias_use(structural_alias);
	        return structural_alias;
	      }
	    }
	    const string type_id_text = decl.type_id ? node_text(*decl.type_id) : string();
    const string spaced_type_id_text = decl.type_id ? spaced_node_text(*decl.type_id) : string();
    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "instantiate-alias-template-text name=" << decl.name
            << " spaced=" << spaced_type_id_text;
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
    if(found != instantiations.end()) {
      if(parser_trace::enabled("template.resolve")) {
        std::ostringstream trace;
        trace << "instantiate-alias-template-hit name=" << decl.name
              << " key=" << key;
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      if(!dependent_arguments &&
         found->second &&
         type_depends_on_template_parameter(found->second)) {
        TypePtr repaired = refine_instantiated_alias(found->second);
        if(repaired && !type_depends_on_template_parameter(repaired)) {
          found->second = repaired;
        }
      }
      note_alias_use(found->second);
      return found->second;
    }
    if(decl.type_id &&
       dependent_arguments &&
       (!reference_class_templates_only ||
        alias_has_non_type_parameter ||
        alias_has_parameter_pack)) {
      alias = make_dependent_alias_specialization();
      instantiations[key] = alias;
      if(parser_trace::enabled("template.resolve")) {
        std::ostringstream trace;
        trace << "instantiate-alias-template-defer-dependent name=" << decl.name
              << " key=" << key
              << " type-id=" << spaced_type_id_text;
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      note_alias_use(alias);
      return alias;
    }
    TypePtr parsed_alias;
    bool parsed_direct = false;
    if(!parsed_direct && decl.type_id) {
      const witness::ScopedTemplateWitnessSourceCapturePause
          source_capture_pause(false);
      const witness::ScopedTemplateWitnessFunctionCallSourceCapturePause
          class_source_capture_pause(false);
      parsed_direct =
          parse_type_id(*inst_scope,
                        *decl.type_id,
                        parsed_alias,
                        reference_class_templates_only);
    }
    if(parsed_direct) {
      alias = refine_instantiated_alias(parsed_alias);
    }
    if(!decl.type_id || !parsed_direct) {
      if(decl.type_id && !scope_has_template_placeholders(*inst_scope)) {
        const bool verbose_bindings = parser_trace::enabled("template.resolve");
        ostringstream out;
        out << "failed alias template type-id parse";
        if(!decl.name.empty()) {
          out << " for " << decl.name;
        }
        out << " [type-id " << type_id_text << "]";
        out << " [type-id-spaced " << spaced_type_id_text << "]";
        if(!arguments.empty()) {
          out << " [args";
          for(size_t i = 0; i < arguments.size(); ++i) {
            out << (i == 0 ? " " : ", ") << template_argument_text(arguments[i]);
          }
          out << "]";
        }
        out << " [inst-scope " << scope_qualified_name(*inst_scope, "<here>") << "]";
        if(verbose_bindings) {
          out << " [inst-bindings " << describe_scope_bindings_for_diagnostic(*inst_scope)
              << "]";
        }
        out << " [use-scope " << scope_qualified_name(use_scope, "<here>") << "]";
        if(verbose_bindings) {
          out << " [use-bindings " << describe_scope_bindings_for_diagnostic(use_scope) << "]";
        }
        if(decl.declaring_scope) {
          out << " [decl-scope "
              << scope_qualified_name(*decl.declaring_scope, "<here>") << "]";
          if(verbose_bindings) {
            out << " [decl-bindings "
                << describe_scope_bindings_for_diagnostic(*decl.declaring_scope) << "]";
          }
        }
        throw_substitution_failure(out.str(), std::string(), "callsemantic");
      }
      if(decl.type_id) {
        TypePtr fallback_alias;
        if(parse_instantiated_alias_text_without_source_capture(
               spaced_type_id_text,
               fallback_alias)) {
          alias = refine_instantiated_alias(fallback_alias);
        }
      }
    }
    if(alias &&
       type_depends_on_template_parameter(alias) &&
       dependent_arguments) {
      if(parser_trace::enabled("template.resolve")) {
        std::ostringstream trace;
        trace << (reference_class_templates_only ?
                      "instantiate-alias-template-keep-dependent-target name=" :
                      "instantiate-alias-template-preserve-dependent name=")
              << decl.name
              << " key=" << key
              << " alias=" << describe_type(alias);
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      if(!reference_class_templates_only ||
         alias_has_non_type_parameter ||
         alias_has_parameter_pack) {
        alias = make_dependent_alias_specialization();
      }
    }
    const string alias_text =
        alias ? (alias->named_display.empty() ? describe_type(alias) : alias->named_display) :
                string();
    if(alias &&
       type_depends_on_template_parameter(alias) &&
       !dependent_arguments &&
       !text_mentions_template_placeholders(*inst_scope, alias_text) &&
       !text_mentions_dependent_non_namespace_binding_names(*inst_scope, alias_text)) {
      if(parser_trace::enabled("template.resolve")) {
        std::ostringstream trace;
        trace << "instantiate-alias-template-reject-dependent name=" << decl.name
              << " key=" << key
              << " alias=" << describe_type(alias);
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      alias = TypePtr();
    }
    if(!decl.type_id || !alias) {
      if(dependent_arguments) {
        alias = make_dependent_alias_specialization();
        instantiations[key] = alias;
        note_alias_use(alias);
        return alias;
      }
      ostringstream out;
      out << "unsupported alias template instantiation";
      if(!decl.name.empty()) {
        out << " for " << decl.name;
      }
      if(decl.type_id) {
        out << " [type-id " << node_text(*decl.type_id) << "]";
      }
      if(!arguments.empty()) {
        out << " [args";
        for(size_t i = 0; i < arguments.size(); ++i) {
          out << (i == 0 ? " " : ", ") << template_argument_text(arguments[i]);
        }
        out << "]";
      }
      throw_substitution_failure(out.str(), std::string(), "callsemantic");
    }
    instantiations[key] = alias;
    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "instantiate-alias-template-store name=" << decl.name
            << " key=" << key
            << " alias=" << describe_type(alias);
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
    note_alias_use(alias);
    return alias;
  }

  string template_argument_text(const TemplateArgument & arg) const
  {
    return template_model::template_argument_text(
        arg,
        [this](const TypePtr & type)
        {
          return lookup_text_for_type_argument(type);
        });
  }

  string reparseable_template_argument_text(const TemplateArgument & arg) const
  {
    if(arg.kind == TemplateArgument::TA_TYPE && arg.type) {
      return reparseable_type_argument_text(arg.type);
    }
    return template_model::template_argument_text(
        arg,
        [](const TypePtr & type)
        {
          return reparseable_type_argument_text(type);
        });
  }

  vector<string> canonical_instantiation_arg_texts(
      const vector<TemplateArgument> & arguments) const
  {
    return template_api::canonical_template_argument_texts(
        *const_cast<Analyzer *>(this),
        arguments);
  }

  AliasTemplateDecl * template_argument_alias_template(const TemplateArgument & arg) const
  {
    return arg.kind == TemplateArgument::TA_ALIAS_TEMPLATE ?
               static_cast<AliasTemplateDecl *>(arg.template_decl) :
               nullptr;
  }

  ClassTemplateDecl * template_argument_class_template(const TemplateArgument & arg) const
  {
    return arg.kind == TemplateArgument::TA_CLASS_TEMPLATE ?
               static_cast<ClassTemplateDecl *>(arg.template_decl) :
               nullptr;
  }

  string template_args_identity_key(const vector<TemplateArgument> & args) const
  {
    return template_api::template_argument_identity_key(
        *const_cast<Analyzer *>(this),
        args);
  }

  const vector<TypePtr> * lookup_type_pack(Scope & scope, const string & name)
  {
    return lookup_unqualified_generic<const vector<TypePtr> *>(
        scope, name,
        [](Scope & target, const string & lookup_name) -> const vector<TypePtr> *
        {
          map<string, vector<TypePtr> >::iterator found =
              target.named_type_packs.find(lookup_name);
          return found == target.named_type_packs.end() ? nullptr : &found->second;
        });
  }

  bool expand_bound_type_pack_pattern_text(Scope & scope,
                                           const string & pattern,
                                           vector<string> & out)
  {
    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "expand-bound-type-pack-pattern scope="
            << scope_qualified_name(scope, "<here>")
            << " pattern=" << pattern;
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
    vector<pair<string, const vector<TypePtr> *> > packs;
    set<string> seen_pack_names;
    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope || current->parent == nullptr) {
        break;
      }
      for(const auto & pack : current->named_type_packs) {
        const string & pack_name = pack.first;
        if(pack_name.empty() ||
           seen_pack_names.count(pack_name) != 0 ||
           !contains_identifier_token(pattern, pack_name)) {
          continue;
        }
        seen_pack_names.insert(pack_name);
        packs.push_back(make_pair(pack_name, &pack.second));
      }
    }

    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "expand-bound-type-pack-pattern packs=" << packs.size();
      for(size_t i = 0; i < packs.size(); ++i) {
        trace << " [" << packs[i].first << ":" << packs[i].second->size() << "]";
      }
      parser_trace::note("template.resolve", std::string(), trace.str());
    }

    if(packs.empty()) {
      return false;
    }

    const size_t pack_size = packs[0].second->size();
    for(size_t i = 1; i < packs.size(); ++i) {
      if(packs[i].second->size() != pack_size) {
        return false;
      }
    }

    for(size_t i = 0; i < pack_size; ++i) {
      string rewritten = pattern;
      bool changed = false;
      for(size_t j = 0; j < packs.size(); ++j) {
        rewritten = replace_identifier_token_text(
            rewritten,
            packs[j].first,
            lookup_reparseable_text_for_type_argument(scope,
                                                      (*(packs[j].second))[i]),
            changed);
      }
      out.push_back(changed ? rewritten : pattern);
    }
    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "expand-bound-type-pack-pattern result-count=" << out.size();
      for(size_t i = 0; i < out.size(); ++i) {
        trace << " [" << i << ":" << out[i] << "]";
      }
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
    return true;
  }

  bool normalized_type_lookup_text_matches(const string & lhs,
                                           const string & rhs) const
  {
    return remove_space_chars(normalize_type_lookup_name(lhs)) ==
           remove_space_chars(normalize_type_lookup_name(rhs));
  }

  TypePtr match_bound_type_text_for_lookup(const TypePtr & candidate,
                                           const string & trimmed) const
  {
    if(!candidate) {
      return TypePtr();
    }

    auto match_placeholder_name = [&](const TypePtr & type) -> TypePtr
    {
      if(!type || type->kind != Type::TK_NAMED) {
        return TypePtr();
      }
      const string placeholder_text =
          strip_elaborated_type_prefix(trim_space(type->named_display));
      return !placeholder_text.empty() &&
                     normalized_type_lookup_text_matches(placeholder_text, trimmed) ?
                 candidate :
                 TypePtr();
    };
    if(type_contains_partial_order_artifact(candidate)) {
      TypePtr matched = match_placeholder_name(candidate);
      if(matched) {
        return matched;
      }
      TypePtr base = strip_top_level_cv(remove_reference_type(candidate));
      return match_placeholder_name(base);
    }

    const string canonical_text = lookup_text_for_type_argument(candidate);
    if(!canonical_text.empty() &&
       normalized_type_lookup_text_matches(canonical_text, trimmed)) {
      return candidate;
    }

    const string reparseable_text = reparseable_type_argument_text(candidate);
    if(!reparseable_text.empty() &&
       normalized_type_lookup_text_matches(reparseable_text, trimmed)) {
      return candidate;
    }

    TypePtr base = strip_top_level_cv(remove_reference_type(candidate));
    if(!base) {
      return TypePtr();
    }

    TypePtr matched =
        match_wrapped_type_text(trimmed, lookup_text_for_type_argument(base), base);
    if(matched) {
      return matched;
    }

    const string reparseable_base_text = reparseable_type_argument_text(base);
    if(!reparseable_base_text.empty()) {
      matched = match_wrapped_type_text(trimmed, reparseable_base_text, base);
      if(matched) {
        return matched;
      }
    }

    return TypePtr();
  }

  TypePtr lookup_bound_type_pack_element_by_text(Scope & scope,
                                                 const string & text) const
  {
    const string trimmed = trim_space(text);
    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope || current->parent == nullptr) {
        break;
      }
      for(const auto & pack : current->named_type_packs) {
        for(size_t i = 0; i < pack.second.size(); ++i) {
          TypePtr matched = match_bound_type_text_for_lookup(pack.second[i], trimmed);
          if(matched) {
            return matched;
          }
        }
      }
    }
    return TypePtr();
  }

  bool update_nested_bound_type_match(const TypePtr & candidate,
                                      const string & trimmed,
                                      set<string> & visiting,
                                      TypePtr & found) const
  {
    if(!candidate) {
      return true;
    }
    TypePtr matched = match_bound_type_text_for_lookup(candidate, trimmed);
    if(matched) {
      if(!found) {
        found = matched;
      } else if(describe_type(found) != describe_type(matched)) {
        return false;
      }
    }

    TypePtr unqualified = strip_top_level_cv(remove_reference_type(candidate));
    if(!unqualified) {
      return true;
    }

    if(unqualified->kind == Type::TK_POINTER ||
       unqualified->kind == Type::TK_MEMBER_POINTER ||
       unqualified->kind == Type::TK_BLOCK_POINTER ||
       unqualified->kind == Type::TK_ARRAY) {
      return update_nested_bound_type_match(unqualified->inner, trimmed, visiting, found);
    }

    if(unqualified->kind == Type::TK_FUNCTION) {
      if(!update_nested_bound_type_match(unqualified->inner, trimmed, visiting, found)) {
        return false;
      }
      for(size_t i = 0; i < unqualified->params.size(); ++i) {
        if(!update_nested_bound_type_match(unqualified->params[i],
                                           trimmed,
                                           visiting,
                                           found)) {
          return false;
        }
      }
      return true;
    }

    if(unqualified->kind != Type::TK_NAMED) {
      return true;
    }

    const string & key = unqualified->named_key;
    if(visiting.count(key) != 0) {
      return true;
    }
    visiting.insert(key);

    ClassInfo * info = class_info_for_type(unqualified);
    vector<TypePtr> type_arguments;
    template_api::append_class_template_type_arguments(info, type_arguments);
    for(size_t i = 0; i < type_arguments.size(); ++i) {
      if(!update_nested_bound_type_match(type_arguments[i],
                                         trimmed,
                                         visiting,
                                         found)) {
        return false;
      }
    }
    return true;
  }

  TypePtr lookup_bound_type_by_text(Scope & scope,
                                    const string & text) const
  {
    if(semantic_hotspot::enabled()) {
      semantic_hotspot::note_semantic_query("lookup_bound_type_by_text", text);
    }
    debug_bound_type_trace("lookup_bound_type_by_text", text);

    const string trimmed = trim_space(text);
    TypePtr found;
    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope || current->parent == nullptr) {
        break;
      }
      for(const auto & name : current->template_bound_type_names) {
        map<string, TypePtr>::const_iterator it = current->named_types.find(name);
        if(it == current->named_types.end() ||
           !it->second) {
          continue;
        }
        set<string> visiting;
        if(!update_nested_bound_type_match(it->second, trimmed, visiting, found)) {
          return TypePtr();
        }
      }
      for(const auto & pack : current->named_type_packs) {
        for(size_t i = 0; i < pack.second.size(); ++i) {
          set<string> visiting;
          if(!update_nested_bound_type_match(pack.second[i], trimmed, visiting, found)) {
            return TypePtr();
          }
        }
      }
    }
    if(found) {
      return found;
    }
    return TypePtr();
  }

  TypePtr lookup_exact_bound_type_name(Scope & scope,
                                       const string & name) const
  {
    const string trimmed = trim_space(name);
    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope || current->parent == nullptr) {
        break;
      }
      map<string, TypePtr>::const_iterator found = current->named_types.find(trimmed);
      if(found != current->named_types.end()) {
        if(current->template_bound_type_names.count(trimmed) != 0 && found->second) {
          return found->second;
        }
        break;
      }
    }
    return TypePtr();
  }

  bool lookup_exact_bound_type_name_with_scope(Scope & scope,
                                               const string & name,
                                               Scope *& bound_scope,
                                               TypePtr & bound_type) const
  {
    const string trimmed = trim_space(name);
    bound_scope = nullptr;
    bound_type.reset();
    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope || current->parent == nullptr) {
        break;
      }
      map<string, TypePtr>::const_iterator found = current->named_types.find(trimmed);
      if(found != current->named_types.end()) {
        if(current->template_bound_type_names.count(trimmed) != 0 && found->second) {
          bound_scope = current;
          bound_type = found->second;
          return true;
        }
        break;
      }
    }
    return false;
  }

  TypePtr lookup_exact_local_type_name(Scope & scope,
                                       const string & name) const
  {
    const string trimmed = trim_space(name);
    if(trimmed.find("::") != string::npos) {
      return TypePtr();
    }
    vector<string> candidate_names;
    const auto add_candidate_name = [&](const string & candidate)
    {
      const string normalized = trim_space(strip_elaborated_type_prefix(candidate));
      if(normalized.empty()) {
        return;
      }
      if(find(candidate_names.begin(), candidate_names.end(), normalized) ==
         candidate_names.end()) {
        candidate_names.push_back(normalized);
      }
      const string unqualified = unqualified_member_name(normalized);
      if(!unqualified.empty() &&
         find(candidate_names.begin(), candidate_names.end(), unqualified) ==
             candidate_names.end()) {
        candidate_names.push_back(unqualified);
      }
    };
    add_candidate_name(trimmed);

    const auto is_function_local_type = [&](const TypePtr & type) -> bool
    {
      if(!type) {
        return false;
      }
      TypePtr base = strip_top_level_cv(remove_reference_type(type));
      if(!base) {
        return false;
      }
      ClassInfo * info = class_info_for_type(base);
      if(info && info->enclosing_scope && info->enclosing_scope->function != nullptr) {
        return true;
      }
      const string reparseable = trim_space(reparseable_type_argument_text(type));
      return reparseable.find("__local_") != string::npos;
    };

    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope || current->parent == nullptr) {
        break;
      }
      for(size_t i = 0; i < candidate_names.size(); ++i) {
        map<string, TypePtr>::const_iterator found =
            current->named_types.find(candidate_names[i]);
        if(found == current->named_types.end() ||
           !is_function_local_type(found->second)) {
          continue;
        }
        map<string, TypePtr>::const_iterator parent_found =
            current->parent->named_types.find(candidate_names[i]);
        if(parent_found == current->parent->named_types.end() ||
           !type_equals(found->second, parent_found->second)) {
          return found->second;
        }
      }
      for(map<string, TypePtr>::const_iterator it = current->named_types.begin();
          it != current->named_types.end();
          ++it) {
        if(!is_function_local_type(it->second)) {
          continue;
        }
        const string reparseable = trim_space(reparseable_type_argument_text(it->second));
        for(size_t i = 0; i < candidate_names.size(); ++i) {
          if(reparseable != candidate_names[i]) {
            continue;
          }
          map<string, TypePtr>::const_iterator parent_found =
              current->parent->named_types.find(it->first);
          if(parent_found == current->parent->named_types.end() ||
             !type_equals(it->second, parent_found->second)) {
            return it->second;
          }
        }
      }
    }
    return TypePtr();
  }

  bool has_non_bound_type_name_shadow(Scope & scope,
                                      const string & name) const
  {
    const string trimmed = trim_space(name);
    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope || current->parent == nullptr) {
        break;
      }
      map<string, TypePtr>::const_iterator found = current->named_types.find(trimmed);
      if(found != current->named_types.end()) {
        return current->template_bound_type_names.count(trimmed) == 0;
      }
    }
    return false;
  }

  vector<string> expand_bound_type_pack_texts(Scope & scope,
                                              const vector<string> & texts)
  {
    return template_api::type::expand_bound_type_pack_texts(*this, scope, texts);
  }

  bool ast_node_value_names_pack_identifier(const CppAstNode & node,
                                            const string & name) const
  {
    if(name.empty()) {
      return false;
    }
    switch(node.kind) {
    case CppAstKind::decl_specifier:
    case CppAstKind::type_specifier:
    case CppAstKind::type_name:
    case CppAstKind::id_expression:
      break;
    default:
      return false;
    }

    const QualifiedName * qualified = cppast_qualified_name_syntax(node);
    if(qualified && (qualified->rooted || !qualified->qualifiers.empty())) {
      return false;
    }

    string value = trim_space(node.value);
    if(node.kind == CppAstKind::decl_specifier ||
       node.kind == CppAstKind::type_specifier ||
       node.kind == CppAstKind::type_name ||
       (value.compare(0, 9, "typename ") == 0)) {
      value = trim_space(strip_elaborated_type_prefix(value));
      if(value.compare(0, 9, "typename ") == 0) {
        value = trim_space(value.substr(9));
      }
    }
    return value == name && value.find("::") == string::npos;
  }

  bool template_id_syntax_mentions_pack_identifier(const TemplateIdSyntax & syntax,
                                                   const string & name) const
  {
    for(size_t i = 0; i < syntax.arguments.size(); ++i) {
      if(contains_identifier_token(syntax.arguments[i], name)) {
        return true;
      }
    }
    for(size_t i = 0; i < syntax.argument_syntaxes.size(); ++i) {
      if(contains_identifier_token(syntax.argument_syntaxes[i].text, name)) {
        return true;
      }
      if(syntax.argument_syntaxes[i].template_id &&
         template_id_syntax_mentions_pack_identifier(
             *syntax.argument_syntaxes[i].template_id,
             name)) {
        return true;
      }
    }
    return false;
  }

  bool ast_node_mentions_pack_identifier(const CppAstNode & node,
                                         const string & name) const
  {
    if(ast_node_value_names_pack_identifier(node, name)) {
      return true;
    }
    if(node.template_id_syntax &&
       template_id_syntax_mentions_pack_identifier(*node.template_id_syntax, name)) {
      return true;
    }
    for(size_t i = 0; i < node.qualifier_template_id_syntaxes.size(); ++i) {
      if(template_id_syntax_mentions_pack_identifier(
             node.qualifier_template_id_syntaxes[i],
             name)) {
        return true;
      }
    }
    for(size_t i = 0; i < node.children.size(); ++i) {
      if(ast_node_mentions_pack_identifier(node.children[i], name)) {
        return true;
      }
    }
    return false;
  }

  string typed_value_binding_pack_text(const ValueBinding & binding) const
  {
    return binding.has_constant_value && !binding.dependent_template_value ?
        lookup_text_for_non_type_template_argument(binding.type, binding.constant_value) :
        binding.name;
  }

  CppAstNode make_pack_type_id_node(const TypePtr & type,
                                    const string & text) const
  {
    CppAstNode type_id;
    type_id.kind = CppAstKind::type_id;
    type_id.value = text;

    CppAstNode specifiers;
    specifiers.kind = CppAstKind::type_specifier_seq;
    specifiers.value = text;

    CppAstNode type_name;
    type_name.kind = CppAstKind::type_name;
    type_name.value = text;
    type_name.semantic_type = type;

    specifiers.children.push_back(type_name);
    type_id.children.push_back(specifiers);
    return type_id;
  }

  TemplateArgumentSyntax clone_template_argument_syntax(
      const TemplateArgumentSyntax & source) const
  {
    TemplateArgumentSyntax out;
    out.text = source.text;
    out.pack_expansion = source.pack_expansion;
    out.has_source_token_start = source.has_source_token_start;
    out.source_token_start = source.source_token_start;
    out.source_location_id = source.source_location_id;
    if(source.template_id) {
      out.template_id.reset(new TemplateIdSyntax(
          clone_template_id_syntax(*source.template_id)));
    }
    if(source.type_id) {
      out.type_id.reset(new CppAstNode(
          clone_pack_substitution_node(*source.type_id)));
    }
    if(source.expression) {
      out.expression.reset(new CppAstNode(
          clone_pack_substitution_node(*source.expression)));
    }
    return out;
  }

  TemplateIdSyntax clone_template_id_syntax(const TemplateIdSyntax & source) const
  {
    TemplateIdSyntax out;
    out.name = source.name;
    out.arguments = source.arguments;
    out.argument_syntaxes.reserve(source.argument_syntaxes.size());
    for(size_t i = 0; i < source.argument_syntaxes.size(); ++i) {
      out.argument_syntaxes.push_back(
          clone_template_argument_syntax(source.argument_syntaxes[i]));
    }
    return out;
  }

  CppAstNode clone_pack_substitution_node(const CppAstNode & source) const
  {
    CppAstNode out = source;
    if(source.qualified_name_syntax) {
      out.qualified_name_syntax.reset(
          new QualifiedName(*source.qualified_name_syntax));
    }
    if(source.template_id_syntax) {
      out.template_id_syntax.reset(new TemplateIdSyntax(
          clone_template_id_syntax(*source.template_id_syntax)));
    }
    out.qualifier_template_id_syntaxes.clear();
    out.qualifier_template_id_syntaxes.reserve(
        source.qualifier_template_id_syntaxes.size());
    for(size_t i = 0; i < source.qualifier_template_id_syntaxes.size(); ++i) {
      out.qualifier_template_id_syntaxes.push_back(
          clone_template_id_syntax(source.qualifier_template_id_syntaxes[i]));
    }
    out.qualifier_type_syntaxes.clear();
    out.qualifier_type_syntaxes.reserve(source.qualifier_type_syntaxes.size());
    for(size_t i = 0; i < source.qualifier_type_syntaxes.size(); ++i) {
      out.qualifier_type_syntaxes.push_back(
          clone_pack_substitution_node(source.qualifier_type_syntaxes[i]));
    }
    out.children.clear();
    out.children.reserve(source.children.size());
    for(size_t i = 0; i < source.children.size(); ++i) {
      out.children.push_back(clone_pack_substitution_node(source.children[i]));
    }
    return out;
  }

  void rewrite_template_id_syntax_pack_arguments(
      TemplateIdSyntax & syntax,
      Scope & scope,
      const map<string, TypePtr> & type_replacements,
      const map<string, ValueBinding> & value_replacements) const
  {
    const auto rewrite_text =
        [&](string text) -> string
        {
          bool changed = false;
          for(map<string, TypePtr>::const_iterator it = type_replacements.begin();
              it != type_replacements.end();
              ++it) {
            text = replace_identifier_token_text(
                text,
                it->first,
                lookup_reparseable_text_for_type_argument(scope, it->second),
                changed);
          }
          for(map<string, ValueBinding>::const_iterator it = value_replacements.begin();
              it != value_replacements.end();
              ++it) {
            text = replace_identifier_token_text(
                text,
                it->first,
                typed_value_binding_pack_text(it->second),
                changed);
          }
          return text;
        };

    for(size_t i = 0; i < syntax.arguments.size(); ++i) {
      syntax.arguments[i] = rewrite_text(syntax.arguments[i]);
    }
    for(size_t i = 0; i < syntax.argument_syntaxes.size(); ++i) {
      TemplateArgumentSyntax & argument = syntax.argument_syntaxes[i];
      argument.text = rewrite_text(argument.text);
      if(argument.type_id) {
        for(map<string, TypePtr>::const_iterator it = type_replacements.begin();
            it != type_replacements.end();
            ++it) {
          if(ast_node_mentions_pack_identifier(*argument.type_id, it->first)) {
            argument.type_id.reset(new CppAstNode(
                make_pack_type_id_node(
                    it->second,
                    lookup_reparseable_text_for_type_argument(scope, it->second))));
            break;
          }
        }
      }
      if(argument.template_id) {
        rewrite_template_id_syntax_pack_arguments(*argument.template_id,
                                                  scope,
                                                  type_replacements,
                                                  value_replacements);
      }
    }
  }

  CppAstNode make_pack_value_expression_node(const ValueBinding & binding) const
  {
    CppAstNode node;
    if(binding.has_constant_value && !binding.dependent_template_value) {
      if(is_bool_type(binding.type)) {
        node.kind = CppAstKind::keyword_literal;
        node.value = binding.constant_value != 0 ? "true" : "false";
        node.has_token = true;
        node.token_kind = RT_SIMPLE;
        node.simple_type = binding.constant_value != 0 ? KW_TRUE : KW_FALSE;
        return node;
      }

      unsigned long long magnitude = 0;
      if(binding.constant_value < 0) {
        magnitude =
            static_cast<unsigned long long>(-(binding.constant_value + 1)) + 1ULL;
      } else {
        magnitude = static_cast<unsigned long long>(binding.constant_value);
      }
      CppAstNode literal;
      literal.kind = CppAstKind::literal;
      literal.value = to_string(magnitude);
      if(binding.constant_value >= 0) {
        return literal;
      }

      node.kind = CppAstKind::unary_expression;
      node.value = "-";
      node.has_token = true;
      node.token_kind = RT_SIMPLE;
      node.simple_type = OP_MINUS;
      node.children.push_back(literal);
      return node;
    }

    node.kind = CppAstKind::id_expression;
    node.value = binding.name;
    return node;
  }

  bool substitute_pack_argument_node_ast(
      Scope & scope,
      const CppAstNode & node,
      const map<string, TypePtr> & type_replacements,
      const map<string, ValueBinding> & value_replacements,
      CppAstNode & out) const
  {
    if(node.kind == CppAstKind::id_expression) {
      for(map<string, ValueBinding>::const_iterator it = value_replacements.begin();
          it != value_replacements.end();
          ++it) {
        if(ast_node_value_names_pack_identifier(node, it->first)) {
          out = make_pack_value_expression_node(it->second);
          return !out.value.empty();
        }
      }
    }

    out = clone_pack_substitution_node(node);
    if(out.template_id_syntax) {
      rewrite_template_id_syntax_pack_arguments(*out.template_id_syntax,
                                                scope,
                                                type_replacements,
                                                value_replacements);
      out.value = template_id_syntax_text_preserving_spacing(*out.template_id_syntax);
    }
    for(size_t i = 0; i < out.qualifier_template_id_syntaxes.size(); ++i) {
      rewrite_template_id_syntax_pack_arguments(out.qualifier_template_id_syntaxes[i],
                                                scope,
                                                type_replacements,
                                                value_replacements);
    }

    for(map<string, TypePtr>::const_iterator it = type_replacements.begin();
        it != type_replacements.end();
        ++it) {
      if(ast_node_value_names_pack_identifier(out, it->first)) {
        out.semantic_type = it->second;
        out.value = lookup_reparseable_text_for_type_argument(scope, it->second);
        out.qualified_name_syntax.reset();
        out.template_id_syntax.reset();
        out.qualifier_template_id_syntaxes.clear();
        out.qualifier_type_syntaxes.clear();
        break;
      }
    }

    vector<CppAstNode> children;
    children.reserve(node.children.size());
    for(size_t i = 0; i < node.children.size(); ++i) {
      CppAstNode child;
      if(!substitute_pack_argument_node_ast(scope,
                                            node.children[i],
                                            type_replacements,
                                            value_replacements,
                                            child)) {
        return false;
      }
      children.push_back(child);
    }
    out.children.swap(children);
    return true;
  }

  bool expand_pack_argument_node(Scope & scope,
                                 const CppAstNode & node,
                                 vector<CppAstNode> & out)
  {
    out.clear();
    if(node.kind != CppAstKind::pack_expansion_expression || node.children.size() != 1) {
      out.push_back(node);
      return true;
    }

    const string pattern = node_text(node.children[0]);
    vector<pair<string, const vector<TypePtr> *> > packs;
    vector<pair<string, const vector<ValueBinding> *> > value_packs;
    const auto trace_pack_failure =
        [&](const std::string & reason,
            const std::string & detail = std::string()) -> void
    {
      if(!parser_trace::enabled("template.resolve")) {
        return;
      }
      std::ostringstream trace;
      trace << "expand-pack-argument-node"
            << " reason=" << reason
            << " scope=" << semantic_trace::scope_name_for_diagnostic(scope)
            << " pattern=" << pattern
            << " type-packs=" << packs.size()
            << " value-packs=" << value_packs.size();
      for(size_t i = 0; i < packs.size(); ++i) {
        trace << " [type " << packs[i].first << "=" << packs[i].second->size() << "]";
      }
      for(size_t i = 0; i < value_packs.size(); ++i) {
        trace << " [value " << value_packs[i].first << "=" << value_packs[i].second->size()
              << "]";
      }
      if(!detail.empty()) {
        trace << " detail=" << detail;
      }
      parser_trace::note("template.resolve", std::string(), trace.str());
    };
    set<string> seen_type_pack_names;
    set<string> seen_value_pack_names;
    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope || current->parent == nullptr) {
        break;
      }
      for(const auto & pack : current->named_type_packs) {
        const string & pack_name = pack.first;
        if(pack_name.empty() ||
           seen_type_pack_names.count(pack_name) != 0 ||
           !ast_node_mentions_pack_identifier(node.children[0], pack_name)) {
          continue;
        }
        seen_type_pack_names.insert(pack_name);
        packs.push_back(make_pair(pack_name, &pack.second));
      }
      for(const auto & pack : current->named_value_packs) {
        const string & pack_name = pack.first;
        if(pack_name.empty() ||
           seen_value_pack_names.count(pack_name) != 0 ||
           !ast_node_mentions_pack_identifier(node.children[0], pack_name)) {
          continue;
        }
        seen_value_pack_names.insert(pack_name);
        value_packs.push_back(make_pair(pack_name, &pack.second));
      }
    }

    if(packs.empty() && value_packs.empty()) {
      out.push_back(node.children[0]);
      return true;
    }

    size_t pack_size = packs.empty() ? value_packs[0].second->size() : packs[0].second->size();
    for(size_t i = 1; i < packs.size(); ++i) {
      if(packs[i].second->size() != pack_size) {
        trace_pack_failure("type-pack-size-mismatch");
        return false;
      }
    }
    for(size_t i = 0; i < value_packs.size(); ++i) {
      if(value_packs[i].second->size() != pack_size) {
        trace_pack_failure("value-pack-size-mismatch");
        return false;
      }
    }

    for(size_t i = 0; i < pack_size; ++i) {
      map<string, TypePtr> type_replacements;
      map<string, ValueBinding> value_replacements;
      for(size_t j = 0; j < packs.size(); ++j) {
        type_replacements[packs[j].first] = (*(packs[j].second))[i];
      }
      for(size_t j = 0; j < value_packs.size(); ++j) {
        value_replacements[value_packs[j].first] = (*(value_packs[j].second))[i];
      }
      CppAstNode expr;
      if(!substitute_pack_argument_node_ast(scope,
                                            node.children[0],
                                            type_replacements,
                                            value_replacements,
                                            expr)) {
        trace_pack_failure("structured-substitution-failed", pattern);
        return false;
      }
      out.push_back(expr);
    }
    return true;
  }

  bool resolve_template_argument(Scope & argument_scope,
                                 Scope & parameter_scope,
                                 const TemplateParameterInfo & parameter,
                                 const string & text,
                                 const TemplateArgumentSyntax * syntax,
                                 TemplateArgument & out)
  {
    return template_api::resolution::resolve_template_argument(
        *this,
        argument_scope,
        parameter_scope,
        parameter,
        text,
        syntax,
        out);
  }

  bool resolve_template_argument(Scope & argument_scope,
                                 Scope & parameter_scope,
                                 const TemplateParameterInfo & parameter,
                                 const string & text,
                                 TemplateArgument & out)
  {
    return resolve_template_argument(
        argument_scope, parameter_scope, parameter, text, nullptr, out);
  }

  bool resolve_template_arguments(Scope & scope,
                                  const vector<TemplateParameterInfo> & parameters,
                                  const vector<string> & texts,
                                  const vector<TemplateArgumentSyntax> * syntaxes,
                                  vector<TemplateArgument> & out,
                                  Scope * default_argument_declaring_scope = nullptr)
  {
    return template_api::resolve_template_arguments(
        *this, scope, parameters, texts, syntaxes, out, default_argument_declaring_scope);
  }

  bool resolve_template_arguments(Scope & scope,
                                  const vector<TemplateParameterInfo> & parameters,
                                  const vector<string> & texts,
                                  vector<TemplateArgument> & out,
                                  Scope * default_argument_declaring_scope = nullptr)
  {
    return resolve_template_arguments(
        scope, parameters, texts, nullptr, out, default_argument_declaring_scope);
  }

  bool fill_trailing_default_template_argument_texts(
      Scope & pattern_scope,
      const vector<TemplateParameterInfo> & parameters,
      const vector<string> & texts,
      Scope * default_argument_declaring_scope,
      vector<string> & out) override
  {
    out = expand_bound_type_pack_texts(pattern_scope, texts);
    for(size_t i = 0; i < out.size(); ++i) {
      out[i] = repair_compacted_template_argument_expression_spacing(out[i]);
    }
    const bool trailing_pack =
        !parameters.empty() && parameters.back().parameter_pack;
    if((!trailing_pack && out.size() > parameters.size()) ||
       (trailing_pack && parameters.empty())) {
      return false;
    }

    vector<pair<string, string> > bound_parameter_texts;
    for(size_t i = 0; i < out.size() && i < parameters.size(); ++i) {
      const TemplateParameterInfo & parameter =
          (trailing_pack && i >= parameters.size() - 1)
              ? parameters.back()
              : parameters[i];
      if(parameter.name.empty()) {
        continue;
      }
      bool replaced = false;
      for(size_t j = 0; j < bound_parameter_texts.size(); ++j) {
        if(bound_parameter_texts[j].first == parameter.name) {
          bound_parameter_texts[j].second = out[i];
          replaced = true;
          break;
        }
      }
      if(!replaced) {
        bound_parameter_texts.push_back(make_pair(parameter.name, out[i]));
      }
    }

    const auto rewrite_default_text =
        [&](const string & original) -> string
        {
          string rewritten = original;
          vector<pair<string, string> > placeholders;
          for(size_t i = 0; i < bound_parameter_texts.size(); ++i) {
            const string & name = bound_parameter_texts[i].first;
            const string & replacement = bound_parameter_texts[i].second;
            if(name.empty() || replacement.empty() ||
               !contains_identifier_token(rewritten, name)) {
              continue;
            }

            const string placeholder =
                "__cppgm_default_arg_placeholder_" + to_string(i) + "__";
            bool changed = false;
            rewritten = replace_elaborated_identifier_token_text(
                rewritten, name, placeholder, changed);
            rewritten = replace_identifier_token_text(
                rewritten, name, placeholder, changed);
            if(changed) {
              placeholders.push_back(make_pair(placeholder, replacement));
            }
          }

          for(size_t i = 0; i < placeholders.size(); ++i) {
            bool changed = false;
            rewritten = replace_identifier_token_text(
                rewritten,
                placeholders[i].first,
                placeholders[i].second,
                changed);
          }
          return rewritten;
        };

    const auto default_argument_text =
        [&](const TemplateParameterInfo & parameter,
            const CppAstNode & default_child) -> string
        {
          string text = node_text(default_child);
          if(!text.empty()) {
            return text;
          }
          if(parameter.kind == TemplateParameterInfo::TP_TYPE &&
             default_child.kind == CppAstKind::type_id) {
            TypePtr type;
            Scope & default_scope =
                default_argument_declaring_scope ? *default_argument_declaring_scope :
                                                   pattern_scope;
            if(parse_type_id(default_scope, default_child, type) && type) {
              text = template_api::type::lookup_text_for_type_argument(*this, type);
              if(text.empty()) {
                text = describe_type(type);
              }
            }
          }
          return text;
        };

    out.reserve(parameters.size());
    for(size_t i = out.size(); i < parameters.size(); ++i) {
      if(parameters[i].parameter_pack) {
        return i + 1 == parameters.size();
      }
      if(!parameters[i].default_argument ||
         parameters[i].default_argument->children.empty()) {
        return false;
      }

      string default_text =
          default_argument_text(parameters[i],
                                parameters[i].default_argument->children[0]);
      if(default_argument_declaring_scope) {
        default_text = rewrite_default_text(default_text);
      }
      if(default_text.empty()) {
        return false;
      }
      out.push_back(default_text);
      if(!parameters[i].name.empty()) {
        bound_parameter_texts.push_back(make_pair(parameters[i].name, default_text));
      }
    }
    return true;
  }

  bool trailing_pack_accepts_argument_count(const vector<TemplateParameterInfo> & parameters,
                                            size_t argument_count) const
  {
    return template_api::resolution::trailing_pack_accepts_argument_count(parameters,
                                                                        argument_count);
  }

  Scope & bind_template_arguments(Scope & declaring_scope,
                                  const vector<TemplateParameterInfo> & parameters,
                                  const vector<TemplateArgument> & arguments)
  {
    return template_api::binding::bind_template_arguments(
        *this,
        declaring_scope,
        parameters,
        arguments);
  }

  void overlay_instantiation_use_scope_bindings(Scope & target,
                                                const Scope & use_scope,
                                                const Scope * declaring_scope) const
  {
    template_api::binding::overlay_instantiation_use_scope_bindings(target,
                                                                   use_scope,
                                                                   declaring_scope);
  }

  Scope & bind_template_arguments_for_instantiation(
      Scope & declaring_scope,
      Scope & use_scope,
      const vector<TemplateParameterInfo> & parameters,
      const vector<TemplateArgument> & arguments,
      const std::map<std::string, std::size_t> * pack_sizes = nullptr,
      ClassInfo * active_owner = nullptr)
  {
    return template_api::binding::bind_template_arguments_for_instantiation(
        *this,
        declaring_scope,
        use_scope,
        parameters,
        arguments,
        pack_sizes,
        active_owner);
  }

  void bind_single_template_argument_into_scope(Scope & scope,
                                                const TemplateParameterInfo & parameter,
                                                const TemplateArgument & argument)
  {
    if(parameter.parameter_pack || parameter.name.empty()) {
      return;
    }
    vector<TemplateParameterInfo> parameters(1, parameter);
    vector<TemplateArgument> arguments(1, argument);
    template_api::binding::bind_template_arguments_into_scope(
        *this,
        scope,
        parameters,
        arguments);
  }

  void bind_template_arguments_into_scope(Scope & scope,
                                          const vector<TemplateParameterInfo> & parameters,
                                          const vector<TemplateArgument> & arguments)
  {
    template_api::binding::bind_template_arguments_into_scope(
        *this,
        scope,
        parameters,
        arguments);
  }

  vector<FunctionBinding *> lookup_functions(Scope & scope,
                                             const string & name,
                                             const semantic_overload::CallAnalysisOptions & options =
                                                 semantic_overload::CallAnalysisOptions())
  {
    (void)options;
    auto filter_ordinary_candidates =
        [](vector<FunctionBinding *> & bindings)
        {
          bindings.erase(
              remove_if(bindings.begin(),
                        bindings.end(),
                        [](FunctionBinding * binding)
                        {
                          return template_api::function_binding_has_source_template_identity(
                              binding);
                        }),
              bindings.end());
        };
    const string normalized_name = normalize_type_lookup_name(name);
    const auto lookup_class_scoped_with_implicit =
        [this](ClassInfo & info,
               const string & lookup_name) -> MemberFunctionLookupResult
    {
      if(lookup_name == "operator=") {
        ensure_implicit_special_members(info);
        ensure_implicit_copy_assignment(info);
        ensure_implicit_move_assignment(info);
      }
      return lookup_class_scoped_functions(info, lookup_name);
    };

    QualifiedName qualified;
    if(!is_conversion_function_name(normalized_name) &&
       split_qualified_name_text(normalized_name, qualified) &&
       (qualified.rooted || !qualified.qualifiers.empty())) {
      vector<FunctionBinding *> out =
          lookup_qualified_class_or_namespace_generic<vector<FunctionBinding *> >(
          scope, qualified,
          [&lookup_class_scoped_with_implicit](Scope & target,
                                               const string & lookup_name)
              -> vector<FunctionBinding *>
          {
            if(target.class_info) {
              MemberFunctionLookupResult result =
                  lookup_class_scoped_with_implicit(*target.class_info, lookup_name);
              if(!result.functions.empty()) {
                return result.functions;
              }
              return lookup_direct_functions(target, lookup_name);
            }
            vector<FunctionBinding *> out;
            semantic_lookup::lookup_functions_in_scopes(vector<Scope *>(1, &target), lookup_name, out);
            return out;
          });
      filter_ordinary_candidates(out);
      return out;
    }

    vector<Scope *> scope_path;
    for(Scope * current = &scope; current; current = current->parent) {
      scope_path.push_back(current);
    }

    const auto using_directive_injection_scope =
        [](Scope & origin_scope, Scope & target_scope) -> Scope *
    {
      set<Scope *> target_namespaces;
      for(Scope * current = &target_scope; current; current = current->parent) {
        if(current->namespace_scope) {
          target_namespaces.insert(current);
        }
      }

      for(Scope * current = &origin_scope; current; current = current->parent) {
        if(current->namespace_scope && target_namespaces.count(current) != 0) {
          return current;
        }
      }

      return &origin_scope;
    };

    const auto collect_imported_functions_for_scope =
        [&](Scope & origin_scope,
            Scope & lookup_scope,
            vector<FunctionBinding *> & out)
    {
      set<const Scope *> visited;
      function<void(Scope &)> visit = [&](Scope & current_scope)
      {
        if(!visited.insert(&current_scope).second) {
          return;
        }

        for(size_t i = 0; i < current_scope.using_directives.size(); ++i) {
          Scope * imported = current_scope.using_directives[i];
          if(using_directive_injection_scope(origin_scope, *imported) == &lookup_scope) {
            semantic_lookup::append_unique_functions(out,
                                                     lookup_direct_functions(*imported,
                                                                             normalized_name));
          }
          visit(*imported);
        }
      };
      visit(origin_scope);
    };

    for(size_t scope_index = 0; scope_index < scope_path.size(); ++scope_index) {
      Scope * current = scope_path[scope_index];
      vector<FunctionBinding *> out = lookup_direct_functions(*current, normalized_name);
      if(out.empty() && current->class_info) {
        out = lookup_class_scoped_with_implicit(*current->class_info,
                                                normalized_name).functions;
      }
      if(out.empty() && current->class_info) {
        out = lookup_member_functions(*current->class_info, normalized_name).functions;
      }
      const bool has_lexical_class =
          !current->class_info && current->function && current->function->lexical_access_class;
      const bool lexical_only =
          has_lexical_class &&
          (!current->function->is_method ||
           current->function->owner_class != current->function->lexical_access_class);
      ClassInfo * lexical_class = current->class_info;
      if(!lexical_class && has_lexical_class) {
        lexical_class = current->function->lexical_access_class;
      }
      if(out.empty() && lexical_class) {
        out = lookup_class_scoped_with_implicit(*lexical_class,
                                                normalized_name).functions;
      }
      if(out.empty() && lexical_class) {
        out = lookup_member_functions(*lexical_class, normalized_name).functions;
      }
      if(lexical_only) {
        out.erase(remove_if(out.begin(),
                            out.end(),
                            [](FunctionBinding * binding)
                            {
                              return binding && binding->owner_class && binding->is_method;
                            }),
                  out.end());
      }
      const bool collect_imports_at_current =
          current->namespace_scope ||
          (!current->parent && !current->namespace_scope &&
           !current->using_directives.empty());
      if(collect_imports_at_current) {
        for(size_t origin_index = 0; origin_index <= scope_index; ++origin_index) {
          collect_imported_functions_for_scope(*scope_path[origin_index], *current, out);
        }
      }
      filter_ordinary_candidates(out);
      if(!out.empty()) {
        if(parser_trace::enabled("template.resolve")) {
          std::ostringstream trace;
          trace << "lookup-functions-hit name=" << name
                << " normalized=" << normalized_name
                << " scope=" << scope_qualified_name(*current, "<here>")
                << " count=" << out.size();
          for(size_t i = 0; i < out.size(); ++i) {
            trace << " [" << i
                  << " binding=" << (out[i] ? out[i]->name : std::string("<null>"))
                  << " ctor=" << ((out[i] && out[i]->is_constructor) ? "yes" : "no")
                  << " dtor=" << ((out[i] && out[i]->is_destructor) ? "yes" : "no")
                  << "]";
          }
          parser_trace::note("template.resolve", std::string(), trace.str());
        }
        return out;
      }
    }

    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "lookup-functions-empty name=" << name
            << " normalized=" << normalized_name;
      for(Scope * current = &scope; current; current = current->parent) {
        trace << " [scope " << scope_qualified_name(*current, "<here>") << " sets";
        if(current->function_sets.empty()) {
          trace << " <none>";
        } else {
          bool first = true;
          for(const auto & entry : current->function_sets) {
            trace << (first ? " " : ",") << entry.first << ":" << entry.second.size();
            first = false;
          }
        }
        trace << "]";
      }
      parser_trace::note("template.resolve", std::string(), trace.str());
    }

    return vector<FunctionBinding *>();
  }

  vector<FunctionBinding *> lookup_functions_node(
      Scope & scope,
      const CppAstNode & node,
      const string & name,
      const semantic_overload::CallAnalysisOptions & options =
          semantic_overload::CallAnalysisOptions())
  {
    const QualifiedName * qualified = cppast_qualified_name_syntax(node);
    if(!qualified ||
       (!qualified->rooted && qualified->qualifiers.empty()) ||
       !node_has_template_id_qualifier_syntax(node)) {
      return lookup_functions(scope, name, options);
    }
    const string leaf_name = strip_trailing_top_level_template_arguments(qualified->name);

    auto filter_ordinary_candidates =
        [](vector<FunctionBinding *> & bindings)
        {
          bindings.erase(
              remove_if(bindings.begin(),
                        bindings.end(),
                        [](FunctionBinding * binding)
                        {
                          return template_api::function_binding_has_source_template_identity(
                              binding);
                        }),
              bindings.end());
        };

    Scope * current_scope = &scope;
    if(qualified->rooted) {
      current_scope = root.get();
    }

    for(size_t i = 0; i < qualified->qualifiers.size(); ++i) {
      Scope * namespace_scope =
          resolve_direct_namespace(*current_scope, qualified->qualifiers[i]);
      if(namespace_scope) {
        current_scope = namespace_scope;
        continue;
      }

      const TemplateIdSyntax * qualifier_template_id =
          i < node.qualifier_template_id_syntaxes.size() &&
                  !node.qualifier_template_id_syntaxes[i].name.name.empty() ?
              &node.qualifier_template_id_syntaxes[i] :
              nullptr;
      TypePtr qualifier_type;
      const string qualifier_lookup_text =
          qualifier_template_id ?
              template_id_syntax_text_preserving_spacing(*qualifier_template_id) :
              qualified->qualifiers[i];
      if(qualifier_template_id) {
        ExactTemplateTypeLookupAnchor qualifier_anchor;
        qualifier_anchor.template_text =
            template_id_syntax_text_preserving_spacing(*qualifier_template_id);
        qualifier_anchor.identifier =
            template_lookup_fragment_identifier(qualifier_anchor.template_text);
        if(qualifier_anchor.identifier.empty()) {
          qualifier_anchor.identifier = qualifier_template_id->name.name;
        }
        qualifier_anchor.compact_key =
            compact_lookup_text(qualifier_anchor.template_text);
        qualifier_anchor.arg_texts = qualifier_template_id->arguments;
        qualifier_anchor.arg_syntaxes = qualifier_template_id->argument_syntaxes;
        qualifier_anchor.has_argument_list = true;
        qualifier_anchor.location =
            template_api::normalize_template_witness_source_location(
                template_api::template_witness_detail::source_location_for_location_id(
                    template_witness_context(),
                    qualifier_template_id->source_location_id));
        if(qualifier_anchor.location.empty()) {
          qualifier_anchor.location =
              template_api::normalize_template_witness_source_location(
                  source_location_for_name_in_subtree(
                      node,
                      unqualified_member_name(qualifier_template_id->name.name)));
        }
        const ScopedExactTemplateTypeLookupAnchor qualifier_anchor_guard(
            qualifier_anchor);
        qualifier_type = lookup_type_impl(*current_scope,
                                          qualifier_lookup_text,
                                          true,
                                          true);
      } else {
        qualifier_type = lookup_type_impl(*current_scope,
                                          qualifier_lookup_text,
                                          true,
                                          true);
      }

      if(!qualifier_type || type_depends_on_template_parameter(qualifier_type)) {
        return lookup_functions(scope, name, options);
      }
      ClassInfo * qualifier_info = class_info_for_type(qualifier_type);
      if(!qualifier_info || !qualifier_info->member_scope) {
        qualifier_info = complete_class_type(qualifier_type);
      }
      if(!qualifier_info || !qualifier_info->member_scope) {
        return lookup_functions(scope, name, options);
      }
      current_scope = qualifier_info->member_scope.get();
    }

    vector<FunctionBinding *> out;
    if(current_scope->class_info) {
      MemberFunctionLookupResult result =
          semantic_lookup::lookup_class_scoped_functions(*current_scope->class_info,
                                                         leaf_name);
      if(!result.functions.empty()) {
        out = result.functions;
      } else {
        out = lookup_direct_functions(*current_scope, leaf_name);
      }
    } else {
      semantic_lookup::lookup_functions_in_scopes(
          vector<Scope *>(1, current_scope), leaf_name, out);
    }
    filter_ordinary_candidates(out);
    return out;
  }

  vector<FunctionBinding *> lookup_function_template_id(
      Scope & scope,
      const TemplateIdSyntax & template_id_syntax,
      const semantic_overload::CallAnalysisOptions & options =
          semantic_overload::CallAnalysisOptions())
  {
    const semantic_overload::CallAnalysisOptions effective_options =
        semantic_policy::apply_analysis_policy(analysis_policy_, options);
    const bool instantiate_bodies = effective_options.instantiate_bodies;
    const QualifiedName & template_id = template_id_syntax.name;
    const vector<string> & arg_texts = template_id_syntax.arguments;
    if(template_id.name.empty()) {
      return vector<FunctionBinding *>();
    }

    vector<FunctionTemplateDecl *> templates;
    if(template_id.rooted || !template_id.qualifiers.empty()) {
      templates =
          lookup_qualified_class_or_namespace_generic<vector<FunctionTemplateDecl *> >(
              scope,
              template_id,
              [](Scope & target, const string & lookup_name) -> vector<FunctionTemplateDecl *>
              {
                return semantic_lookup::lookup_direct_function_templates(target, lookup_name);
              });
    } else {
      templates = lookup_function_templates(scope, template_id.name);
    }

    vector<FunctionBinding *> out;
    for(size_t i = 0; i < templates.size(); ++i) {
      vector<TemplateArgument> arguments;
      try
      {
        if(!resolve_template_arguments(scope,
                                       templates[i]->parameters,
                                       arg_texts,
                                       &template_id_syntax.argument_syntaxes,
                                       arguments,
                                       templates[i]->declaring_scope)) {
          continue;
        }
      }
      catch(const TemplateSubstitutionFailure &)
      {
        continue;
      }
      if(!template_model::template_arguments_fully_bind_parameters(
             templates[i]->parameters,
             arguments)) {
        continue;
      }
      if(!template_api::resolution::explicit_function_template_arguments_determine_signature(
             *this,
             *templates[i],
             arg_texts.size())) {
        continue;
      }
      FunctionBinding * binding = nullptr;
      try
      {
        binding = acquire_function_template(*templates[i],
                                            arguments,
                                            &scope,
                                            nullptr,
                                            instantiate_bodies).function_binding;
      }
      catch(const TemplateSubstitutionFailure &)
      {
        continue;
      }
      if(binding) {
        out.push_back(binding);
      }
    }
    return out;
  }

  TypePtr lookup_type(Scope & scope, const string & name, bool reference_class_templates_only = false)
  {
    return lookup_type_impl(scope, name, reference_class_templates_only, false);
  }

  TypePtr maybe_introduce_elaborated_type(Scope & scope, const string & text)
  {
    return maybe_introduce_elaborated_class_type(scope, text);
  }

  const ValueBinding * lookup_value(Scope & scope, const string & name)
  {
    vector<Scope *> scope_path;
    for(Scope * current = &scope; current; current = current->parent) {
      scope_path.push_back(current);
    }

    const auto using_directive_injection_scope =
        [](Scope & origin_scope, Scope & target_scope) -> Scope *
    {
      set<Scope *> target_namespaces;
      for(Scope * current = &target_scope; current; current = current->parent) {
        if(current->namespace_scope) {
          target_namespaces.insert(current);
        }
      }

      for(Scope * current = &origin_scope; current; current = current->parent) {
        if(current->namespace_scope && target_namespaces.count(current) != 0) {
          return current;
        }
      }

      return &origin_scope;
    };

    const auto collect_imported_value_for_scope =
        [&](Scope & origin_scope,
            Scope & lookup_scope) -> semantic_lookup::ValueLookupFromUsingDirectivesResult
    {
      semantic_lookup::ValueLookupFromUsingDirectivesResult result;
      set<const Scope *> visited;
      function<void(Scope &)> visit = [&](Scope & current_scope)
      {
        if(result.ambiguous || !visited.insert(&current_scope).second) {
          return;
        }

        for(size_t i = 0; i < current_scope.using_directives.size(); ++i) {
          Scope * imported = current_scope.using_directives[i];
          if(using_directive_injection_scope(origin_scope, *imported) == &lookup_scope) {
            const ValueBinding * direct = semantic_lookup::lookup_direct_value(*imported, name);
            if(direct) {
              if(!result.binding) {
                result.binding = direct;
              } else if(!semantic_lookup::same_value_binding_entity(result.binding, direct)) {
                result.ambiguous = true;
                return;
              }
            }
          }
          visit(*imported);
          if(result.ambiguous) {
            return;
          }
        }
      };
      visit(origin_scope);
      return result;
    };

    for(size_t scope_index = 0; scope_index < scope_path.size(); ++scope_index) {
      Scope * current = scope_path[scope_index];
      const ValueBinding * direct = semantic_lookup::lookup_direct_value(*current, name);
      if(!current->namespace_scope && direct) {
        return direct;
      }
      const bool has_lexical_class =
          !current->class_info && current->function && current->function->lexical_access_class;
      const bool lexical_only =
          has_lexical_class &&
          (!current->function->is_method ||
           current->function->owner_class != current->function->lexical_access_class);
      ClassInfo * lexical_class = current->class_info;
      if(!lexical_class && has_lexical_class) {
        lexical_class = current->function->lexical_access_class;
      }
      if(lexical_class) {
        MemberValueLookupResult member = lookup_member_value(*lexical_class, name);
        if(lexical_only && member.binding && member.binding->kind == ValueBinding::VK_FIELD) {
          member.binding = nullptr;
        }
        if(member.binding) {
          return member.binding;
        }
      }
      if(current->namespace_scope) {
        semantic_lookup::ValueLookupFromUsingDirectivesResult imported;
        for(size_t origin_index = 0; origin_index <= scope_index; ++origin_index) {
          semantic_lookup::ValueLookupFromUsingDirectivesResult partial =
              collect_imported_value_for_scope(*scope_path[origin_index], *current);
          if(partial.ambiguous) {
            return nullptr;
          }
          if(partial.binding) {
            if(!imported.binding) {
              imported.binding = partial.binding;
            } else if(!semantic_lookup::same_value_binding_entity(imported.binding,
                                                                  partial.binding)) {
              return nullptr;
            }
          }
        }
        if(direct && imported.binding &&
           !semantic_lookup::same_value_binding_entity(direct, imported.binding)) {
          return nullptr;
        }
        if(direct) {
          return direct;
        }
        if(imported.binding) {
          return imported.binding;
        }
      }
    }
    return nullptr;
  }

  bool lookup_member_value_in_scope_chain(Scope & scope,
                                          const string & name,
                                          MemberValueLookupResult & out)
  {
    for(Scope * current = &scope; current; current = current->parent) {
      map<string, ValueBinding>::const_iterator found = current->values.find(name);
      if(found != current->values.end()) {
        return false;
      }
      const bool has_lexical_class =
          !current->class_info && current->function && current->function->lexical_access_class;
      const bool lexical_only =
          has_lexical_class &&
          (!current->function->is_method ||
           current->function->owner_class != current->function->lexical_access_class);
      ClassInfo * lexical_class = current->class_info;
      if(!lexical_class && has_lexical_class) {
        lexical_class = current->function->lexical_access_class;
      }
      if(lexical_class) {
        MemberValueLookupResult member = lookup_member_value(*lexical_class, name);
        if(lexical_only && member.binding && member.binding->kind == ValueBinding::VK_FIELD) {
          member.binding = nullptr;
        }
        if(member.binding) {
          out = member;
          return true;
        }
      }
    }
    return false;
  }

  TypePtr target_function_type(const TypePtr & target)
  {
    if(!target) {
      return TypePtr();
    }

    TypePtr base = strip_top_level_cv(target);
    if(base->kind == Type::TK_FUNCTION) {
      return base;
    }
    if((base->kind == Type::TK_LVALUE_REFERENCE ||
        base->kind == Type::TK_RVALUE_REFERENCE) &&
       strip_top_level_cv(base->inner)->kind == Type::TK_FUNCTION) {
      return strip_top_level_cv(base->inner);
    }
    if((base->kind == Type::TK_POINTER || base->kind == Type::TK_BLOCK_POINTER) &&
       strip_top_level_cv(base->inner)->kind == Type::TK_FUNCTION) {
      return strip_top_level_cv(base->inner);
    }
    return TypePtr();
  }

  bool resolve_function_id_for_target(Scope & scope,
                                      const string & name,
                                      const TypePtr & target,
                                      ExprInfo & out,
                                      const QualifiedName * name_syntax = nullptr,
                                      const TemplateIdSyntax * name_template_id_syntax = nullptr)
  {
    return semantic_overload::resolve_function_id_for_target(*this,
                                                             scope,
                                                             name,
                                                             target,
                                                             out,
                                                             name_syntax,
                                                             name_template_id_syntax);
  }

  void append_function_template_call_candidates(Scope & lookup_scope,
                                                Scope & argument_scope,
                                                const string & name,
                                                const vector<const CppAstNode *> & arg_nodes,
                                                vector<FunctionBinding *> & out,
                                                const semantic_overload::CallAnalysisOptions & options =
                                                    semantic_overload::CallAnalysisOptions())
  {
    semantic_overload::append_function_template_call_candidates(*this,
                                                                lookup_scope,
                                                                argument_scope,
                                                                name,
                                                                arg_nodes,
                                                                out,
                                                                options);
  }

  Scope & append_template_scope(Scope & parent)
  {
    template_scopes.emplace_back(new Scope(&parent, "", false));
    template_scopes.back()->class_info = parent.class_info;
    template_scopes.back()->function = parent.function;
    return *template_scopes.back();
  }

  bool parse_template_parameters(const CppAstNode & clause,
                                 vector<TemplateParameterInfo> & out,
                                 Scope * placeholder_scope = nullptr,
                                 string * failure_reason = nullptr) override
  {
    out.clear();
    auto fail = [&failure_reason](const string & reason) -> bool
    {
      if(failure_reason) {
        *failure_reason = reason;
      }
      return false;
    };
    const CppAstNode * list = find_child_kind(clause, CppAstKind::template_parameter_list);
    if(!list) {
      return true;
    }

    for(size_t i = 0; i < list->children.size(); ++i) {
      const CppAstNode & parameter = list->children[i];
      TemplateParameterInfo info;
      if(parameter.kind == CppAstKind::type_parameter) {
        info.parameter_pack = find_child_kind(parameter, CppAstKind::parameter_pack) != nullptr;

        const CppAstNode * identifier = find_child_kind(parameter, CppAstKind::identifier);
        info.name = identifier ? identifier->value : string();
        info.placeholder_key = string("template-parameter ") +
                               (info.name.empty() ? string("<unnamed>") : info.name) + "#" +
                               to_string(out.size());
        info.default_argument = find_child_kind(parameter, CppAstKind::default_template_argument);
        const CppAstNode * template_template =
            find_child_kind(parameter, CppAstKind::template_template_parameter);
        if(template_template) {
          info.kind = TemplateParameterInfo::TP_TEMPLATE_TEMPLATE;
          const CppAstNode * inner_clause =
              find_child_kind(parameter, CppAstKind::template_parameter_clause);
          const CppAstNode * inner_list =
              inner_clause ? find_child_kind(*inner_clause, CppAstKind::template_parameter_list) :
                             nullptr;
          bool accepts_any_arity = false;
          if(inner_list && !inner_list->children.empty()) {
            const CppAstNode & trailing = inner_list->children.back();
            accepts_any_arity =
                find_child_kind(trailing, CppAstKind::parameter_pack) != nullptr;
          }
          info.template_parameter_count =
              accepts_any_arity ? static_cast<size_t>(-1) :
                                  (inner_list ? inner_list->children.size() : 0);
        }

        if(placeholder_scope) {
          if(info.kind == TemplateParameterInfo::TP_TYPE) {
            if(!info.name.empty()) {
              template_scope::bind_named_type(
                  *placeholder_scope,
                  info.name,
                  make_named(string("typename ") + info.name,
                             info.placeholder_key,
                             true));
            }
          } else {
            if(!info.name.empty()) {
              template_scope::bind_template_template_placeholder(*placeholder_scope,
                                                                info.name);
            }
          }
        }
        out.push_back(info);
        continue;
      }

      if(parameter.kind == CppAstKind::non_type_template_parameter) {
        if(!placeholder_scope) {
          return fail("non-type-template-parameter without placeholder scope");
        }
        const CppAstNode * specifiers = find_child_kind(parameter, CppAstKind::decl_specifier_seq);
        const CppAstNode * declarator = find_child_kind(parameter, CppAstKind::declarator);
        const CppAstNode * abstract =
            find_child_kind(parameter, CppAstKind::abstract_declarator);
        info.parameter_pack =
            find_child_kind(parameter, CppAstKind::parameter_pack) != nullptr ||
            (declarator && declarator_has_parameter_pack(*declarator)) ||
            (abstract && declarator_has_parameter_pack(*abstract));
        if(!specifiers) {
          return fail("non-type-template-parameter missing decl-specifier-seq");
        }

        bool is_typedef = false;
        TypePtr base;
        if(!parse_decl_spec(*specifiers, *placeholder_scope, is_typedef, base) || is_typedef) {
          ostringstream out;
          out << "failed non-type parameter decl-specifier parse: " << node_text(*specifiers);
          return fail(out.str());
        }

        string name;
        TypePtr type = base;
        if(declarator) {
          if(!parse_declarator(*placeholder_scope, *declarator, base, name, type)) {
            ostringstream out;
            out << "failed non-type parameter declarator parse: " << node_text(*declarator);
            return fail(out.str());
          }
        } else if(abstract) {
          if(!parse_abstract_declarator(*placeholder_scope, *abstract, base, type)) {
            ostringstream out;
            out << "failed non-type parameter abstract-declarator parse: "
                << node_text(*abstract);
            return fail(out.str());
          }
        }

        if(!is_supported_non_type_template_parameter_type(out, type)) {
          ostringstream out;
          out << "unsupported non-type parameter type: " << describe_type(type)
              << " [specifiers " << node_text(*specifiers) << "]"
              << " [specifier-children";
          for(size_t i = 0; i < specifiers->children.size(); ++i) {
            out << (i == 0 ? " " : ", ")
                << cppast_kind_text(specifiers->children[i].kind);
            if(!specifiers->children[i].value.empty()) {
              out << ":" << specifiers->children[i].value;
            }
          }
          out << "]";
          return fail(out.str());
        }

        info.kind = TemplateParameterInfo::TP_NON_TYPE;
        info.name = name;
        info.value_type = type;
        info.non_type_decl_specifier_seq = specifiers;
        info.non_type_decl_specifier_text = node_text(*specifiers);
        info.non_type_declarator = declarator;
        info.non_type_abstract_declarator = abstract;
        info.default_argument = find_child_kind(parameter, CppAstKind::default_template_argument);
        if(placeholder_scope && !name.empty()) {
          template_scope::bind_non_type_value(*placeholder_scope, name, type, 0, true);
        }
        out.push_back(info);
        continue;
      }

      ostringstream out;
      out << "unsupported template parameter kind " << cppast_kind_text(parameter.kind);
      return fail(out.str());
    }
    return true;
  }

  void add_lightweight_template_parameter_placeholders(
      const CppAstNode & clause,
      Scope & placeholder_scope)
  {
    const CppAstNode * list =
        find_child_kind(clause, CppAstKind::template_parameter_list);
    if(!list) {
      return;
    }
    for(size_t i = 0; i < list->children.size(); ++i) {
      const CppAstNode & parameter = list->children[i];
      if(parameter.kind == CppAstKind::type_parameter) {
        const CppAstNode * identifier =
            find_child_kind(parameter, CppAstKind::identifier);
        const string name = identifier ? identifier->value : string();
        if(name.empty()) {
          continue;
        }
        if(find_child_kind(parameter,
                           CppAstKind::template_template_parameter)) {
          template_scope::bind_template_template_placeholder(placeholder_scope, name);
        } else {
          template_scope::bind_named_type(
              placeholder_scope,
              name,
              make_named(string("typename ") + name,
                         string("template-parameter ") + name,
                         true));
        }
        continue;
      }
      if(parameter.kind == CppAstKind::non_type_template_parameter) {
        const CppAstNode * declarator =
            find_child_kind(parameter, CppAstKind::declarator);
        string name;
        if(!declarator || !declarator_declared_identifier(*declarator, name) ||
           name.empty()) {
          continue;
        }
        template_scope::bind_non_type_value(placeholder_scope,
                                            name,
                                            make_fundamental(FT_INT),
                                            0,
                                            true);
      }
    }
  }

  void record_template_parameter_clause_source_uses(Scope & scope,
                                                    const CppAstNode & node) override
  {
    if(!witness::source_capture_enabled(template_witness_context())) {
      return;
    }

    const template_api::ScopedTemplateWitnessLifecyclePause lifecycle_pause;

    if(node.kind != CppAstKind::template_declaration || node.children.empty()) {
      return;
    }

    const CppAstNode * parameters =
        find_child_kind(node, CppAstKind::template_parameter_clause);
    if(!parameters) {
      return;
    }

    Scope & pattern_scope = append_template_scope(scope);
    vector<TemplateParameterInfo> template_parameters;
    string failure_reason;
    bool parsed_parameters = false;
    try {
      {
        const witness::ScopedTemplateWitnessSourceCapturePause
            source_capture_pause;
        parsed_parameters =
            parse_template_parameters(*parameters,
                                      template_parameters,
                                      &pattern_scope,
                                      &failure_reason);
      }
    } catch(...) {
      semantic_fallback_audit::rethrow_if_strict_fallback();
      parsed_parameters = false;
    }
    if(!parsed_parameters) {
      add_lightweight_template_parameter_placeholders(*parameters,
                                                      pattern_scope);
    }

    std::set<std::string> recorded_alias_uses;
    std::set<std::string> recorded_class_uses;
    std::function<void(const CppAstNode &)> record_alias_uses =
        [&](const CppAstNode & current) -> void
    {
      std::string template_text;
      if(!current.value.empty()) {
        template_text = template_lookup_fragment_text(current.value);
      }
      if(template_text.empty()) {
        template_text = template_lookup_fragment_text(node_text(current));
      }
      if(!template_text.empty()) {
        const std::string identifier =
            template_lookup_fragment_identifier(template_text);
        if(!identifier.empty()) {
          const std::vector<WitnessUseLocationSearch> location_searches = {
            WitnessUseLocationSearch(WITNESS_USE_LOCATION_NAME_IN_NODE,
                                     &current),
            WitnessUseLocationSearch(WITNESS_USE_LOCATION_NAME_IN_SUBTREE,
                                     &current)
          };
          std::string exact_location =
              select_witness_use_location(location_searches, identifier);
          if(exact_location.empty()) {
            const std::string current_location =
                template_api::normalize_template_witness_source_location(
                    source_location_for_node(current));
            const std::string same_line_location =
                template_api::template_witness_detail::
                    source_location_for_identifier_token_on_or_after(
                    template_witness_context(),
                    current_location,
                    identifier,
                    true,
                    true);
            if(source_location_points_at_identifier(same_line_location,
                                                    identifier)) {
              exact_location = same_line_location;
            }
          }
          if(exact_location.empty() ||
             !source_location_points_at_identifier(exact_location, identifier)) {
            if(const TemplateIdSyntax * direct_syntax =
                   first_template_id_syntax_in_subtree(current)) {
              const std::string syntax_identifier =
                  unqualified_member_name(direct_syntax->name.name);
              if(syntax_identifier == identifier) {
                const std::string syntax_location =
                    template_api::normalize_template_witness_source_location(
                        template_api::template_witness_detail::
                            source_location_for_location_id(
                            template_witness_context(),
                            direct_syntax->source_location_id));
                if(source_location_points_at_identifier(syntax_location,
                                                        identifier)) {
                  exact_location = syntax_location;
                }
              }
            }
          }
          if(!exact_location.empty() &&
             source_location_points_at_identifier(exact_location, identifier) &&
             source_location_identifier_followed_by(exact_location,
                                                    identifier,
                                                    '<') &&
             recorded_alias_uses.insert(exact_location).second) {
            QualifiedName template_id;
            vector<string> arg_texts;
            const TemplateIdSyntax * syntax =
                template_id_syntax_for_anchor_at_or_after_location(
                    template_witness_context(),
                    current,
                    identifier,
                    exact_location);
            if(!syntax) {
              const TemplateIdSyntax * direct_syntax =
                  first_template_id_syntax_in_subtree(current);
              if(direct_syntax &&
                 unqualified_member_name(direct_syntax->name.name) ==
                     identifier) {
                syntax = direct_syntax;
              }
            }
            bool have_template_id = false;
            if(syntax) {
              template_id = syntax->name;
              arg_texts = template_id_argument_witness_source_texts(*syntax);
              have_template_id = !template_id.name.empty();
            } else {
              have_template_id =
                  semantic_utils::split_top_level_template_id_text(
                      template_text,
                      template_id,
                      arg_texts);
            }
            if(have_template_id) {
              if(AliasTemplateDecl * alias_template =
                     lookup_alias_template(pattern_scope, template_id)) {
                vector<string> source_arg_texts = arg_texts;
                const vector<TemplateArgumentSyntax> * source_arg_syntaxes =
                    syntax ? &syntax->argument_syntaxes : nullptr;
                if(syntax) {
                  source_arg_texts =
                      template_id_argument_witness_source_texts(*syntax);
                }
                vector<TemplateArgument> alias_arguments;
                if(witness::source_capture_enabled(template_witness_context()) &&
                   resolve_template_arguments(
                       pattern_scope,
                       alias_template->parameters,
                       arg_texts,
                       source_arg_syntaxes,
                       alias_arguments,
                       alias_template->declaring_scope)) {
                  vector<string> witness_source_arg_texts = source_arg_texts;
                  template_argument_semantics::
                      canonicalize_alias_template_source_argument_texts(
                          alias_template->parameters,
                          witness_source_arg_texts);
                  witness::AliasUseEmitRequest request;
                  request.use_location = exact_location;
                  request.template_id_occurrence =
                      witness::make_source_template_id_occurrence(
                          exact_location,
                          witness_source_arg_texts);
                  fill_template_id_source_occurrence_argument_facts(
                      pattern_scope,
                      witness_source_arg_texts,
                      source_arg_syntaxes,
                      alias_arguments,
                      request.template_id_occurrence);
                  template_api::with_template_services(
                      *this,
                      [&](template_api::TemplateServices & services)
                      {
                        template_argument_semantics::
                            mark_alias_template_value_owner_argument_facts(
                                services,
                                &pattern_scope,
                                *alias_template,
                                alias_arguments,
                                source_arg_syntaxes,
                                request.template_id_occurrence);
                      });
                  request.template_name =
                      template_api::alias_template_witness_entity(alias_template);
                  request.origin =
                      witness::AliasUseEmissionOrigin::ResolvedAliasTemplateId;
                  const semantic_model::SourceDeclAnchorCache & decl_anchor =
                      semantic_trace::alias_template_decl_anchor(*this,
                                                                 alias_template);
                  witness::set_selected_decl_anchor(
                      request.selected_decl_location,
                      request.selected_decl_anchor,
                      decl_anchor);
                  request.selected_decl_anchor_explicit = true;
                  template_api::append_template_witness_source_bindings(
                      *this,
                      request.bindings,
                      alias_template->parameters,
                      alias_arguments,
                      witness_source_arg_texts,
                      "explicit",
                      "defaulted",
                      false);
                  rewrite_current_specialization_alias_binding_texts(
                      *this,
                      pattern_scope,
                      alias_template->parameters,
                      alias_arguments,
                      witness_source_arg_texts,
                      source_arg_syntaxes,
                      request.bindings,
                      &request.template_id_occurrence);
                  witness::emit_alias_use(template_witness_context(), request);
                }
                const ScopedTemplateUseLocation use_location_guard(exact_location);
                try {
                  instantiate_alias_template_with_syntax(
                      *alias_template,
                      pattern_scope,
                      arg_texts,
                      syntax ? &syntax->argument_syntaxes : nullptr,
                      true);
                } catch(...) {
                  semantic_fallback_audit::rethrow_if_strict_fallback();
                }
              }
              const vector<NestedSourceTemplateIdOccurrence> nested_occurrences =
                  nested_template_id_source_occurrences_at_location(
                      exact_location,
                      false);
              for(size_t nested_index = 0;
                  nested_index < nested_occurrences.size();
                  ++nested_index) {
                QualifiedName nested_template_id;
                vector<string> nested_arg_texts;
                if(!semantic_utils::split_top_level_template_id_text(
                       nested_occurrences[nested_index].text,
                       nested_template_id,
                       nested_arg_texts)) {
                  continue;
                }
                AliasTemplateDecl * nested_alias_template =
                    lookup_alias_template(pattern_scope, nested_template_id);
                const TemplateIdSyntax * nested_syntax =
                    template_id_syntax_for_anchor_at_or_after_location(
                        template_witness_context(),
                        current,
                        nested_occurrences[nested_index].template_name,
                        nested_occurrences[nested_index].location);
                if(nested_syntax) {
                  nested_arg_texts =
                      template_id_argument_witness_source_texts(
                          *nested_syntax);
                }
                if(!nested_alias_template) {
                  ClassTemplateDecl * nested_class_template =
                      lookup_class_template(pattern_scope, nested_template_id);
                  if(!nested_class_template ||
                     !recorded_class_uses.insert(
                         nested_occurrences[nested_index].location).second) {
                    continue;
                  }
                  witness::ClassUseSourceDecision decision;
                  if(witness::class_use_recording_enabled() &&
                     build_class_use_source_decision_from_template_text(
                         pattern_scope,
                         nested_occurrences[nested_index].text,
                         nested_occurrences[nested_index].location,
                         decision,
                         nested_syntax)) {
                    witness::emit_class_use_decision(
                        decision,
                        witness::SourceUseOwnership::SourceOwned,
                        witness::SourceUseRole::TypeUse,
                        witness::ClassUseEmissionOrigin::NestedSourceTemplateId);
                  }
                  continue;
                }
                if(!recorded_alias_uses.insert(
                       nested_occurrences[nested_index].location).second) {
                  continue;
                }
                const ScopedTemplateUseLocation nested_use_location_guard(
                    nested_occurrences[nested_index].location);
                try {
                  instantiate_alias_template_with_syntax(
                      *nested_alias_template,
                      pattern_scope,
                      nested_arg_texts,
                      nested_syntax ? &nested_syntax->argument_syntaxes :
                                      nullptr,
                      true);
                } catch(...) {
                  semantic_fallback_audit::rethrow_if_strict_fallback();
                }
              }
            }
          }
        }
      }

      for(size_t i = 0; i < current.children.size(); ++i) {
        if(current.children[i].kind == CppAstKind::template_declaration) {
          continue;
        }
        record_alias_uses(current.children[i]);
      }
    };
    record_alias_uses(*parameters);

    const CppAstNode & inner = node.children.back();
    if(inner.kind == CppAstKind::template_declaration) {
      record_template_parameter_clause_source_uses(pattern_scope, inner);
      return;
    }

    record_alias_uses(inner);

    for(size_t i = 0; i < inner.children.size(); ++i) {
      if(inner.children[i].kind == CppAstKind::template_declaration) {
        record_template_parameter_clause_source_uses(pattern_scope,
                                                     inner.children[i]);
      }
    }
  }

  callsemantic::ConstantValueLookupCallbacks constant_value_lookup_callbacks()
  {
    callsemantic::ConstantValueLookupCallbacks callbacks;
    callbacks.source_location_for_name_in_subtree =
        [this](const CppAstNode & node,
               const std::string & name,
               bool prefer_last) {
          return source_location_for_name_in_subtree(node, name, prefer_last);
        };
    callbacks.earliest_qualified_use_location_for_prefix =
        [this](const std::string & prefix) {
          return earliest_qualified_use_location_for_prefix(prefix);
        };
    callbacks.earliest_qualified_use_location_for_value =
        [this](const std::string & value) {
          return earliest_qualified_use_location_for_value(value);
        };
    callbacks.emit_nested_class_use_source_events_from_location =
        [this](Scope & scope,
               const std::string & location,
               witness::SourceUseOwnership ownership,
               const std::string & skip_exact_template_name) {
          emit_nested_class_use_source_events_from_location(scope,
                                                            location,
                                                            ownership,
                                                            skip_exact_template_name);
        };
    callbacks.class_template_argument_source_locations_for_current_use =
        [this](const std::string & template_name,
               const std::vector<TemplateParameterInfo> & parameters,
               const std::vector<std::string> & arg_texts) {
          return class_template_argument_source_locations_for_current_use(
              template_name, parameters, arg_texts);
        };
    return callbacks;
  }

  bool materialize_constant_binding_value(ValueBinding & binding,
                                          constant_eval::ConstexprValue & value)
  {
    const callsemantic::ConstantValueLookupCallbacks callbacks =
        constant_value_lookup_callbacks();
    return callsemantic::materialize_constant_binding_value(*this,
                                                            callbacks,
                                                            binding,
                                                            value);
  }

  bool lookup_constant_template_id_value(
      Scope & scope,
      const TemplateIdSyntax & template_id,
      const string & display_name,
      constant_eval::ConstexprValue & out) override
  {
    const callsemantic::ConstantValueLookupCallbacks callbacks =
        constant_value_lookup_callbacks();
    return callsemantic::lookup_constant_template_id_value(*this,
                                                           callbacks,
                                                           scope,
                                                           template_id,
                                                           display_name,
                                                           out);
  }

  void record_constexpr_direct_function_call_source_use(
      Scope & scope,
      const CppAstNode & callee,
      FunctionBinding & binding,
      const TemplateIdSyntax * template_id_syntax,
      std::size_t explicit_arg_count)
  {
    const callsemantic::ConstantValueLookupCallbacks callbacks =
        constant_value_lookup_callbacks();
    callsemantic::record_constexpr_direct_function_call_source_use(
        *this,
        callbacks,
        scope,
        callee,
        binding,
        template_id_syntax,
        explicit_arg_count);
  }

  bool lookup_constant_template_member_value(
      Scope & scope,
      const TemplateIdSyntax & qualifier_template_id,
      const string & member_name,
      const string & display_name,
      constant_eval::ConstexprValue & out) override
  {
    const callsemantic::ConstantValueLookupCallbacks callbacks =
        constant_value_lookup_callbacks();
    return callsemantic::lookup_constant_template_member_value(*this,
                                                               callbacks,
                                                               scope,
                                                               qualifier_template_id,
                                                               member_name,
                                                               display_name,
                                                               out);
  }

  bool lookup_constant_value_node(Scope & scope,
                                  const string & name,
                                  const CppAstNode * node,
                                  constant_eval::ConstexprValue & out) override
  {
    const callsemantic::ConstantValueLookupCallbacks callbacks =
        constant_value_lookup_callbacks();
    return callsemantic::lookup_constant_value_node(*this,
                                                    callbacks,
                                                    scope,
                                                    name,
                                                    node,
                                                    out);
  }

  bool lookup_constant_value(Scope & scope,
                             const string & name,
                             constant_eval::ConstexprValue & out) override
  {
    const callsemantic::ConstantValueLookupCallbacks callbacks =
        constant_value_lookup_callbacks();
    return callsemantic::lookup_constant_value(*this,
                                               callbacks,
                                               scope,
                                               name,
                                               out);
  }

  bool lookup_constant_value(Scope & scope, const string & name, long long & out)
  {
    const callsemantic::ConstantValueLookupCallbacks callbacks =
        constant_value_lookup_callbacks();
    return callsemantic::lookup_constant_value(*this,
                                               callbacks,
                                               scope,
                                               name,
                                               out);
  }

  Scope * resolve_qualified_scope_for_node(Scope & scope,
                                           const QualifiedName & qualified,
                                           const CppAstNode & node,
                                           bool allow_dependent_class_qualifiers)
  {
    const callsemantic::ConstantValueLookupCallbacks callbacks =
        constant_value_lookup_callbacks();
    return callsemantic::resolve_qualified_scope_for_node(*this,
                                                          callbacks,
                                                          scope,
                                                          qualified,
                                                          node,
                                                          allow_dependent_class_qualifiers);
  }

  bool evaluate_constant_call_expression_value(
      Scope & scope,
      constant_eval::Evaluator & evaluator,
      const CppAstNode & node,
      const vector<constant_eval::ConstexprValue> & args,
      constant_eval::ConstexprValue & out)
  {
    callsemantic::ConstantCallEvaluationState state = { functions };
    callsemantic::ConstantCallEvaluationCallbacks callbacks;
    callbacks.record_constexpr_direct_function_call_source_use =
        [this](Scope & target_scope,
               const CppAstNode & callee,
               FunctionBinding & binding,
               const TemplateIdSyntax * template_id_syntax,
               std::size_t explicit_arg_count) {
          record_constexpr_direct_function_call_source_use(target_scope,
                                                           callee,
                                                           binding,
                                                           template_id_syntax,
                                                           explicit_arg_count);
        };
    return callsemantic::evaluate_constant_call_expression_value(
        *this, state, callbacks, scope, evaluator, node, args, out);
  }

  bool evaluate_constant_expression_value(Scope & scope,
                                          const CppAstNode & node,
                                          constant_eval::ConstexprValue & out)
  {
    semantic_hotspot::note_constant_evaluation(node, source_location_for_node(node));
    return semantic_consteval::evaluate_expression_value(*this, scope, node, out);
  }

  bool evaluate_constant_expression(Scope & scope,
                                    const CppAstNode & node,
                                    long long & out)
  {
    constant_eval::ConstexprValue value;
    return evaluate_constant_expression_value(scope, node, value) &&
           constant_eval::constexpr_value_to_integral(value, out);
  }

  bool evaluate_initializer_constant_value(Scope & scope,
                                           const CppAstNode & initializer,
                                           constant_eval::ConstexprValue & value)
  {
    try
    {
      return semantic_consteval::evaluate_initializer_value(*this, scope, initializer, value);
    }
    catch(const logic_error &)
    {
      return false;
    }
  }

  bool evaluate_initializer_constant_value(Scope & scope,
                                           const CppAstNode & initializer,
                                           const TypePtr & target,
                                           constant_eval::ConstexprValue & value)
  {
    try
    {
      return semantic_consteval::evaluate_initializer_value(*this,
                                                            scope,
                                                            initializer,
                                                            target,
                                                            value);
    }
    catch(const logic_error &)
    {
      return false;
    }
  }

  void note_binding_constexpr_value(ValueBinding & binding,
                                    const constant_eval::ConstexprValue & value)
  {
    binding.has_constexpr_value = true;
    binding.constexpr_value = value;
    binding.has_constant_value = false;
    binding.constant_value = 0;
    long long integral = 0;
    if(constant_eval::constexpr_value_to_integral(value, integral)) {
      binding.has_constant_value = true;
      binding.constant_value = integral;
    }
  }

  bool try_note_binding_initializer_constexpr_value(ValueBinding & binding,
                                                    Scope & scope,
                                                    const CppAstNode & initializer,
                                                    const TypePtr & target)
  {
    constant_eval::ConstexprValue value;
    if(!evaluate_initializer_constant_value(scope, initializer, target, value)) {
      return false;
    }
    note_binding_constexpr_value(binding, value);
    return true;
  }

  bool try_note_binding_default_initialized_constexpr_value(ValueBinding & binding,
                                                            Scope & scope,
                                                            const TypePtr & type)
  {
    constant_eval::ConstexprValue value;
    if(!semantic_consteval::evaluate_default_initialized_value(*this, scope, type, value)) {
      return false;
    }
    note_binding_constexpr_value(binding, value);
    return true;
  }

  bool evaluate_initializer_constant(Scope & scope,
                                     const CppAstNode & initializer,
                                     long long & value)
  {
    return semantic_consteval::evaluate_initializer_integral(*this,
                                                             scope,
                                                             initializer,
                                                             value);
  }

  TypePtr lookup_qualified_template_type_node(Scope & scope,
                                              const CppAstNode & node,
                                              const string & lookup_name,
                                              bool reference_class_templates_only)
  {
    const string normalized_lookup_name = normalize_type_lookup_name(lookup_name);
    const TemplateIdSyntax * lone_template_id = cppast_template_id_syntax(node);
    const QualifiedName * qualified_syntax = cppast_qualified_name_syntax(node);
    if(node.qualifier_template_id_syntaxes.empty() &&
       !lone_template_id &&
       !(qualified_syntax &&
         (qualified_syntax->rooted || !qualified_syntax->qualifiers.empty()))) {
      return TypePtr();
    }

    QualifiedName qualified;
    QualifiedName lookup_qualified;
    const bool have_lookup_qualified =
        !normalized_lookup_name.empty() &&
        split_qualified_name_text(normalized_lookup_name, lookup_qualified) &&
        (lookup_qualified.rooted || !lookup_qualified.qualifiers.empty());
    if(have_lookup_qualified) {
      qualified = lookup_qualified;
    } else if(qualified_syntax &&
       (qualified_syntax->rooted || !qualified_syntax->qualifiers.empty())) {
      qualified = *qualified_syntax;
    } else if(!split_qualified_name_text(normalized_lookup_name, qualified)) {
      return TypePtr();
    }
    if(qualified.qualifiers.empty()) {
      return TypePtr();
    }

    Scope * current_scope = &scope;
    if(qualified.rooted) {
      while(current_scope->parent) {
        current_scope = current_scope->parent;
      }
    }
    bool qualifier_chain_starts_from_template_binding = false;
    bool qualified_member_alias_source_owner_is_dependent = false;

    const auto local_template_id_for_current_scope =
        [](const TemplateIdSyntax & source) -> TemplateIdSyntax
    {
      TemplateIdSyntax local = source;
      if(local.name.rooted || !local.name.qualifiers.empty()) {
        local.name.rooted = false;
        local.name.qualifiers.clear();
        local.name.name = unqualified_member_name(local.name.name);
      }
      return local;
    };

    for(size_t i = 0; i < qualified.qualifiers.size(); ++i) {
      const TemplateIdSyntax * qualifier_template_id =
          i < node.qualifier_template_id_syntaxes.size() &&
                  !node.qualifier_template_id_syntaxes[i].name.name.empty() ?
              &node.qualifier_template_id_syntaxes[i] :
              nullptr;
      if(!qualifier_template_id &&
         i == 0 &&
         lone_template_id &&
         !lone_template_id->name.name.empty()) {
        const string qualifier_head =
            unqualified_member_name(
                strip_trailing_top_level_template_arguments(
                    qualified.qualifiers[i]));
        if(qualifier_head == lone_template_id->name.name) {
          qualifier_template_id = lone_template_id;
        }
      }
      const string source_qualifier_identifier =
          qualifier_template_id ?
              unqualified_member_name(qualifier_template_id->name.name) :
              unqualified_member_name(qualified.qualifiers[i]);
      if(qualifier_identifier_uses_template_binding(*current_scope,
                                                    source_qualifier_identifier)) {
        qualifier_chain_starts_from_template_binding = true;
        qualified_member_alias_source_owner_is_dependent = true;
      }
      if(!qualifier_template_id && is_identifier_text(qualified.qualifiers[i])) {
        Scope * namespace_scope = nullptr;
        if(i == 0 && !qualified.rooted) {
          for(Scope * lexical = current_scope; lexical; lexical = lexical->parent) {
            namespace_scope =
                semantic_lookup::resolve_direct_namespace(*lexical,
                                                          qualified.qualifiers[i]);
            if(namespace_scope) {
              break;
            }
          }
        } else {
          namespace_scope =
              semantic_lookup::resolve_direct_namespace(*current_scope,
                                                        qualified.qualifiers[i]);
        }
        if(namespace_scope) {
          current_scope = namespace_scope;
          continue;
        }
      }
      TemplateIdSyntax local_qualifier_template_id;
      const TemplateIdSyntax * lookup_qualifier_template_id = qualifier_template_id;
      if(qualifier_template_id) {
        local_qualifier_template_id =
            local_template_id_for_current_scope(*qualifier_template_id);
        lookup_qualifier_template_id = &local_qualifier_template_id;
        if(template_id_syntax_mentions_template_binding(
               scope,
               *qualifier_template_id)) {
          qualified_member_alias_source_owner_is_dependent = true;
        }
      }
      TypePtr qualifier_type;
      const string qualifier_lookup_text =
          lookup_qualifier_template_id ?
              template_id_syntax_text_preserving_spacing(*lookup_qualifier_template_id) :
              qualified.qualifiers[i];
      const template_api::ClassTemplateSourceUseMode qualifier_source_use_mode =
          qualifier_chain_starts_from_template_binding ?
              template_api::ClassTemplateSourceUseMode::NestedArgumentsOnly :
              template_api::ClassTemplateSourceUseMode::EmitClassUse;
      if(lookup_qualifier_template_id) {
        ExactTemplateTypeLookupAnchor qualifier_anchor;
        qualifier_anchor.template_text =
            template_id_syntax_text_preserving_spacing(*lookup_qualifier_template_id);
        qualifier_anchor.identifier =
            template_lookup_fragment_identifier(qualifier_anchor.template_text);
        if(qualifier_anchor.identifier.empty()) {
          qualifier_anchor.identifier = lookup_qualifier_template_id->name.name;
        }
        qualifier_anchor.compact_key =
            compact_lookup_text(qualifier_anchor.template_text);
        qualifier_anchor.arg_texts = lookup_qualifier_template_id->arguments;
        qualifier_anchor.arg_syntaxes = lookup_qualifier_template_id->argument_syntaxes;
        qualifier_anchor.has_argument_list = true;
        qualifier_anchor.location =
            template_api::normalize_template_witness_source_location(
                template_api::template_witness_detail::source_location_for_location_id(
                    template_witness_context(),
                    qualifier_template_id->source_location_id));
        if(qualifier_anchor.location.empty()) {
          qualifier_anchor.location =
            template_api::normalize_template_witness_source_location(
                source_location_for_name_in_subtree(
                    node,
                    unqualified_member_name(lookup_qualifier_template_id->name.name)));
        }
        if(qualifier_anchor.location.empty()) {
          qualifier_anchor.location =
              template_api::normalize_template_witness_source_location(
                  parser_trace::current_use_location());
        }
        if(qualifier_anchor.location.empty()) {
          qualifier_anchor.location =
              template_api::normalize_template_witness_source_location(
                  source_location_for_node(node));
        }
        const ScopedExactTemplateTypeLookupAnchor qualifier_anchor_guard(
            qualifier_anchor);
        const vector<string> arg_texts =
            template_id_argument_texts_preserving_spacing(*lookup_qualifier_template_id);
        const ScopedTemplateUseLocation use_location_guard(
            qualifier_anchor.location);
        if(AliasTemplateDecl * alias_template =
               semantic_lookup::lookup_direct_alias_template(
                   *current_scope,
                   lookup_qualifier_template_id->name.name)) {
          qualifier_type =
              instantiate_alias_template_with_syntax(
                  *alias_template,
                  scope,
                  arg_texts,
                  &lookup_qualifier_template_id->argument_syntaxes,
                  true);
        } else if(ClassTemplateDecl * class_template =
                      semantic_lookup::lookup_direct_class_template(
                          *current_scope,
                          lookup_qualifier_template_id->name.name)) {
          ClassInfo * info =
              reference_class_template_instantiation_with_syntax(
                  *class_template,
                  scope,
                  arg_texts,
                  &lookup_qualifier_template_id->argument_syntaxes,
                  qualifier_source_use_mode);
          if(info) {
            qualifier_type = info->type;
          }
        }
        if(!qualifier_type) {
          qualifier_type = lookup_type_impl(*current_scope,
                                            qualifier_lookup_text,
                                            true,
                                            true,
                                            qualifier_source_use_mode);
        }
      } else {
        qualifier_type = lookup_type_impl(*current_scope,
                                          qualifier_lookup_text,
                                          true,
                                          true);
      }
      if(!qualifier_type) {
        return TypePtr();
      }
      if(type_depends_on_template_parameter(qualifier_type)) {
        vector<string> member_path;
        for(size_t j = i + 1; j < qualified.qualifiers.size(); ++j) {
          member_path.push_back(qualified.qualifiers[j]);
        }
        member_path.push_back(qualified.name);
        return make_dependent_qualified_member_type(normalized_lookup_name,
                                                    qualifier_type,
                                                    member_path,
                                                    node.has_leading_typename,
                                                    remaining_dependent_member_template_ids(
                                                        node,
                                                        qualified,
                                                        i));
      }

      ClassInfo * qualifier_info = class_info_for_type(qualifier_type);
      if(!qualifier_info || !qualifier_info->member_scope) {
        qualifier_info = complete_class_type(qualifier_type);
      }
      if(!qualifier_info || !qualifier_info->member_scope) {
        return TypePtr();
      }
      ensure_class_reference_members(*qualifier_info);
      current_scope = qualifier_info->member_scope.get();
    }

	    // `current_scope` is the resolved qualifier target; access must still be
	    // checked from the original lexical `scope`.
	    const string leaf_lookup_name =
	        strip_trailing_top_level_template_arguments(qualified.name);
	    const TemplateIdSyntax * leaf_template_id = cppast_template_id_syntax(node);
	    if((!leaf_template_id ||
	        leaf_template_id->name.name != leaf_lookup_name) &&
	       qualified.qualifiers.size() < node.qualifier_template_id_syntaxes.size()) {
	      const TemplateIdSyntax & candidate =
	          node.qualifier_template_id_syntaxes[qualified.qualifiers.size()];
	      if(!candidate.name.name.empty() &&
	         candidate.name.name == leaf_lookup_name) {
	        leaf_template_id = &candidate;
	      }
	    }
	    if(leaf_template_id && !leaf_template_id->name.name.empty()) {
	      if(leaf_lookup_name == leaf_template_id->name.name) {
	        TemplateIdSyntax local_leaf_template_id =
	            local_template_id_for_current_scope(*leaf_template_id);
	        AliasTemplateDecl * alias_template =
	            semantic_lookup::lookup_direct_alias_template(
	                *current_scope,
	                local_leaf_template_id.name.name);
	        if(alias_template) {
	          const vector<string> arg_texts =
	              template_id_argument_texts_preserving_spacing(local_leaf_template_id);
	          const ScopedTemplateUseLocation use_location_guard(
	              template_api::normalize_template_witness_source_location(
	                  source_location_for_name_in_subtree(
	                      node,
	                      local_leaf_template_id.name.name)));
	          TypePtr alias =
	              instantiate_alias_template_with_syntax(
	                  *alias_template,
	                  scope,
	                  arg_texts,
	                  &local_leaf_template_id.argument_syntaxes,
	                  reference_class_templates_only,
	                  qualified_member_alias_source_owner_is_dependent);
	          if(alias) {
	            return alias;
	          }
	        }
	        ClassTemplateDecl * class_template =
	            semantic_lookup::lookup_direct_class_template(
	                *current_scope,
	                local_leaf_template_id.name.name);
	        if(class_template) {
	          const vector<string> arg_texts =
	              template_id_argument_texts_preserving_spacing(local_leaf_template_id);
	          const ScopedTemplateUseLocation use_location_guard(
	              template_api::normalize_template_witness_source_location(
	                  source_location_for_name_in_subtree(
	                      node,
	                      local_leaf_template_id.name.name)));
	          ClassInfo * info = reference_class_templates_only ?
	              reference_class_template_instantiation_with_syntax(
	                  *class_template,
	                  scope,
	                  arg_texts,
	                  &local_leaf_template_id.argument_syntaxes) :
	              instantiate_class_template(
	                  *class_template,
	                  scope,
	                  arg_texts,
	                  &local_leaf_template_id.argument_syntaxes);
	          if(info) {
	            return info->type;
	          }
	        }
	      }
	    }
	    TypePtr direct =
	        semantic_lookup::resolve_direct_type_qualifier(*this,
	                                                       *current_scope,
	                                                       scope,
                                                       qualified.name);
    if(direct) {
      return direct;
    }
    if(current_scope->class_info) {
      const bool ensure_current_member_references =
          !(((current_scope->class_info->full_member_collection_in_progress ||
              current_scope->class_info->reference_member_collection_in_progress) &&
             current_scope->class_info->member_scope &&
             scope_is_within(*current_scope,
                             current_scope->class_info->member_scope.get())));
      MemberTypeLookupResult member =
          lookup_member_type(*this,
                             *current_scope->class_info,
                             qualified.name,
                             ensure_current_member_references,
                             &scope);
      if(member.type) {
        return member.type;
      }
    }
    return TypePtr();
  }

  TypePtr lookup_type_node(Scope & scope,
                           const CppAstNode & node,
                           const string & lookup_name,
                           bool reference_class_templates_only = false)
  {
    const template_api::TemplateWitnessContext witness_context =
        template_witness_context();
    const bool witness_capture_enabled =
        witness::source_capture_enabled(witness_context);
    const string inherited_use_location =
        template_api::normalize_template_witness_source_location(
            template_public_use_location_or(string()));
    const string source_lookup_text = lookup_name;
    string lookup_source_name =
        unqualified_member_name(
            strip_trailing_top_level_template_arguments(lookup_name));
    if(lookup_source_name.empty()) {
      lookup_source_name = lookup_name;
    }
    const TemplateIdSyntax * direct_template_id_syntax =
        cppast_template_id_syntax(node);
    string node_use_location;
    bool node_use_location_from_direct_template_id = false;
    if(direct_template_id_syntax &&
       !direct_template_id_syntax->name.name.empty() &&
       unqualified_member_name(direct_template_id_syntax->name.name) ==
           lookup_source_name &&
       direct_template_id_syntax->source_location_id != 0) {
      node_use_location =
          template_api::template_witness_detail::source_location_for_location_id(
              witness_context,
              direct_template_id_syntax->source_location_id);
      node_use_location_from_direct_template_id = !node_use_location.empty();
    }
    if(node_use_location.empty() && !lookup_source_name.empty()) {
      node_use_location = source_location_for_name_in_node(node, lookup_source_name);
    }
    if(node_use_location.empty() && !lookup_source_name.empty()) {
      node_use_location =
          source_location_for_name_in_subtree(node, lookup_source_name);
    }
    node_use_location =
        template_api::normalize_template_witness_source_location(
            node_use_location);
    if(witness_capture_enabled &&
       !lookup_source_name.empty() &&
       !source_location_points_at_identifier(node_use_location,
                                            lookup_source_name)) {
      node_use_location.clear();
    }
    if(node_use_location.empty()) {
      node_use_location =
          template_api::normalize_template_witness_source_location(
              template_api::preferred_fragment_use_location(
                  witness_context,
                  node));
    }
    if(node_use_location.empty()) {
      node_use_location =
          template_api::normalize_template_witness_source_location(
              preferred_source_location_for_node(node));
    }
    const string effective_use_location =
        node_use_location_from_direct_template_id ?
            node_use_location :
            prefer_later_source_location(inherited_use_location, node_use_location);
    ExactTemplateTypeLookupAnchor exact_lookup_anchor;
    exact_lookup_anchor.template_text =
        template_lookup_fragment_text(
            !source_lookup_text.empty() ? source_lookup_text : lookup_name);
    exact_lookup_anchor.identifier =
        template_lookup_fragment_identifier(
            exact_lookup_anchor.template_text);
    exact_lookup_anchor.compact_key =
        compact_lookup_text(exact_lookup_anchor.template_text);
    const TemplateIdSyntax * template_id_syntax = nullptr;
    if(direct_template_id_syntax &&
       !direct_template_id_syntax->name.name.empty() &&
       unqualified_member_name(direct_template_id_syntax->name.name) ==
           exact_lookup_anchor.identifier) {
      template_id_syntax = direct_template_id_syntax;
    }
    if(!exact_lookup_anchor.template_text.empty()) {
      if(!template_id_syntax) {
        template_id_syntax =
            witness_capture_enabled ?
                template_id_syntax_for_anchor_at_or_after_location(
                    witness_context,
                    node,
                    exact_lookup_anchor.identifier,
                    effective_use_location) :
                template_id_syntax_for_anchor(node,
                                              exact_lookup_anchor.identifier);
      }
      if(!template_id_syntax &&
         !node.qualifier_template_id_syntaxes.empty()) {
        template_id_syntax = first_template_id_syntax_in_subtree(node);
      }
      if(template_id_syntax) {
        exact_lookup_anchor.arg_texts = template_id_syntax->arguments;
        exact_lookup_anchor.arg_syntaxes =
            template_id_syntax->argument_syntaxes;
        exact_lookup_anchor.has_argument_list = true;
      }
    }
    if(!exact_lookup_anchor.identifier.empty()) {
      const bool require_template_open =
          exact_lookup_anchor.template_text.find('<') != std::string::npos;
      if(witness_capture_enabled) {
        exact_lookup_anchor.location =
            template_api::normalize_template_witness_source_location(
                template_api::template_witness_detail::
                    source_location_for_identifier_token_on_or_after(
                    witness_context,
                    effective_use_location,
                    exact_lookup_anchor.identifier,
                    false,
                    require_template_open));
      } else if(!node_use_location.empty()) {
        exact_lookup_anchor.location = node_use_location;
      }
      if(exact_lookup_anchor.location.empty() && template_id_syntax) {
        exact_lookup_anchor.location =
            template_api::normalize_template_witness_source_location(
                template_api::template_witness_detail::source_location_for_location_id(
                    witness_context,
                    template_id_syntax->source_location_id));
      }
      if(exact_lookup_anchor.location.empty() && template_id_syntax) {
        const std::string syntax_identifier =
            unqualified_member_name(template_id_syntax->name.name);
        if(!syntax_identifier.empty()) {
          exact_lookup_anchor.location =
              template_api::normalize_template_witness_source_location(
                  source_location_for_name_in_subtree(node,
                                                      syntax_identifier));
        }
      }
      if(witness_capture_enabled &&
         exact_lookup_anchor.location.empty() &&
         source_location_points_at_identifier(node_use_location,
                                              exact_lookup_anchor.identifier)) {
        exact_lookup_anchor.location = node_use_location;
      }
    }
    const ScopedTemplateUseLocation use_location_guard(
        !effective_use_location.empty() ? effective_use_location :
                                         inherited_use_location);
    const ScopedExactTemplateTypeLookupAnchor exact_lookup_anchor_guard(
        exact_lookup_anchor);
    const template_api::ScopedTemplateWitnessSourceTypeLookup
        source_type_lookup_guard;
    const QualifiedName * qualified_lookup =
        cppast_qualified_name_syntax(node);
    const template_api::ScopedTemplateWitnessQualifiedMemberTypeLookup
        qualified_member_type_lookup_guard(
            qualified_lookup &&
            !qualified_lookup->qualifiers.empty() &&
            unqualified_member_name(qualified_lookup->name) == "type");
    TypePtr structured_qualified =
        lookup_qualified_template_type_node(scope,
                                            node,
                                            lookup_name,
                                            reference_class_templates_only);
    if(structured_qualified) {
      return structured_qualified;
    }
    if(direct_template_id_syntax &&
       !direct_template_id_syntax->name.name.empty() &&
       (unqualified_member_name(direct_template_id_syntax->name.name) ==
            lookup_source_name ||
        (!exact_lookup_anchor.identifier.empty() &&
         unqualified_member_name(direct_template_id_syntax->name.name) ==
             exact_lookup_anchor.identifier))) {
      const vector<string> arg_texts =
          template_id_argument_texts_preserving_spacing(
              *direct_template_id_syntax);
      const string direct_use_location =
          !exact_lookup_anchor.location.empty() ?
              exact_lookup_anchor.location :
              (!effective_use_location.empty() ? effective_use_location :
                                                 inherited_use_location);
      const ScopedTemplateUseLocation direct_use_location_guard(
          direct_use_location);
      if(AliasTemplateDecl * alias_template =
             lookup_alias_template(scope, direct_template_id_syntax->name)) {
        TypePtr alias =
            instantiate_alias_template_with_syntax(
                *alias_template,
                scope,
                arg_texts,
                &direct_template_id_syntax->argument_syntaxes,
                reference_class_templates_only);
        if(alias) {
          return alias;
        }
      }
      if(ClassTemplateDecl * class_template =
             lookup_class_template(scope, direct_template_id_syntax->name)) {
        ClassInfo * info = reference_class_templates_only ?
            reference_class_template_instantiation_with_syntax(
                *class_template,
                scope,
                arg_texts,
                &direct_template_id_syntax->argument_syntaxes) :
            instantiate_class_template(
                *class_template,
                scope,
                arg_texts,
                &direct_template_id_syntax->argument_syntaxes);
        if(info) {
          return info->type;
        }
      }
    }
    QualifiedName fallback_lookup_name;
    const bool fallback_lookup_is_qualified =
        split_qualified_name_text(lookup_name, fallback_lookup_name) &&
        (fallback_lookup_name.rooted || !fallback_lookup_name.qualifiers.empty());
    if(node_has_template_id_qualifier_syntax(node) &&
       fallback_lookup_is_qualified) {
      return TypePtr();
    }
    return lookup_type(scope, lookup_name, reference_class_templates_only);
  }

  AstDeclHooks make_decl_hooks(Scope & scope,
                               bool reference_class_templates_only = false)
  {
    AstDeclHooks hooks = callsemantic_phase_bridge::make_decl_hooks(
        *this,
        scope,
        reference_class_templates_only);
    hooks.lookup_type_node =
        [this, &scope, reference_class_templates_only](const CppAstNode & node)
        {
          const string lookup_name =
              node.kind == CppAstKind::type_name && !node.value.empty() ?
                  node.value :
                  node_text(node);
          TypePtr result =
              lookup_type_node(scope,
                               node,
                               lookup_name,
                               reference_class_templates_only);
          if(result) {
            for(size_t i = 0; i < node.qualifier_template_id_syntaxes.size(); ++i) {
              record_dependent_pattern_class_use_for_template_id_syntax(
                  scope,
                  node,
                  node.qualifier_template_id_syntaxes[i]);
            }
          }
          return result;
        };
    hooks.lookup_type = [this, &scope, reference_class_templates_only](const string & name) {
      TypePtr exact_local = lookup_exact_local_type_name(scope, name);
      if(exact_local) {
        return exact_local;
      }
      return lookup_type(scope, name, reference_class_templates_only);
    };
    hooks.parse_decltype_specifier = [this, &scope](const CppAstNode & node, TypePtr & out) {
      return parse_decltype_specifier(scope, node, out);
    };
    return hooks;
  }

  bool parse_type_text(Scope & scope,
                       const string & text,
                       TypePtr & type,
                       bool reference_class_templates_only = false)
  {
    return parse_type_text_scoped(scope,
                                  scope,
                                  text,
                                  type,
                                  reference_class_templates_only);
  }

  bool parse_type_text_scoped(Scope & parse_scope,
                              Scope & semantic_scope,
                              const string & text,
                              TypePtr & type,
                              bool reference_class_templates_only)
  {
    semantic_hotspot::note_fragment_request("type", text);
    if(parsed_type_text_cache_enabled()) {
      const semantic_cache::ParsedTypeTextCacheKey key =
          parsed_type_text_cache_key(parse_scope,
                                     semantic_scope,
                                     text,
                                     reference_class_templates_only);
      unordered_map<semantic_cache::ParsedTypeTextCacheKey,
                    ParsedTypeTextCacheEntry,
                    semantic_cache::ParsedTypeTextCacheKeyHash>::const_iterator found =
          cache_state_.parsed_type_text_cache.find(key);
      if(found != cache_state_.parsed_type_text_cache.end()) {
        metrics_.note_cache_hit(semantic_metrics::CK_PARSED_TYPE_TEXT);
        type = found->second.type;
        return found->second.parsed;
      }
      metrics_.note_cache_miss(semantic_metrics::CK_PARSED_TYPE_TEXT);

      ParsedTypeTextCacheEntry entry;
      entry.parsed =
          parse_type_text_scoped_uncached(parse_scope,
                                          semantic_scope,
                                          text,
                                          entry.type,
                                          reference_class_templates_only);
      const ParsedTypeTextCacheEntry & cached =
          cache_state_.parsed_type_text_cache.insert(make_pair(key, entry)).first->second;
      type = cached.type;
      return cached.parsed;
    }

    return parse_type_text_scoped_uncached(parse_scope,
                                           semantic_scope,
                                           text,
                                           type,
                                           reference_class_templates_only);
  }

  bool parse_type_text_scoped_uncached(Scope & parse_scope,
                                       Scope & semantic_scope,
                                       const string & text,
                                       TypePtr & type,
                                       bool reference_class_templates_only)
  {
    (void)parse_scope;
    (void)reference_class_templates_only;
    return template_api::type::resolve_type_argument_text(
        *this, semantic_scope, text, type);
  }

  bool explicit_function_nothrow(FunctionBinding & binding, bool & out)
  {
    refresh_explicit_function_nothrow_parse_state(binding);
    switch(binding.explicit_function_nothrow_kind) {
    case EFNK_NONE:
    case EFNK_INVALID:
      return false;
    case EFNK_ALWAYS_TRUE:
      out = true;
      return true;
    case EFNK_ALWAYS_FALSE:
      out = false;
      return true;
    case EFNK_EXPR:
      break;
    case EFNK_UNINITIALIZED:
      return false;
    }

    if(binding.explicit_function_nothrow_eval_cached) {
      out = binding.explicit_function_nothrow_eval_value;
      return true;
    }

    const CppAstNode * expr =
        binding.function_qualifier &&
                !binding.function_qualifier->children.empty() ?
            &binding.function_qualifier->children[0] :
            nullptr;
    if(expr == nullptr || !evaluate_trivial_noexcept_expression(*expr, out)) {
      return false;
    }
    binding.explicit_function_nothrow_eval_cached = true;
    binding.explicit_function_nothrow_eval_value = out;
    return true;
  }

  bool evaluate_explicit_function_nothrow_semantically(FunctionBinding & binding,
                                                       bool & out)
  {
    if(explicit_function_nothrow(binding, out)) {
      return true;
    }

    refresh_explicit_function_nothrow_parse_state(binding);
    if(binding.explicit_function_nothrow_kind != EFNK_EXPR) {
      return false;
    }
    if(binding.explicit_function_nothrow_eval_cached) {
      out = binding.explicit_function_nothrow_eval_value;
      return true;
    }

    Scope qualifier_scope = build_function_qualifier_scope(binding);
    const CppAstNode * expr =
        binding.function_qualifier &&
                !binding.function_qualifier->children.empty() ?
            &binding.function_qualifier->children[0] :
            nullptr;
    if(binding.explicit_function_nothrow_expr_text.empty() || expr == nullptr) {
      return false;
    }

    constant_eval::ConstexprValue value;
    if(!evaluate_constant_expression_value(qualifier_scope, *expr, value) ||
       !constant_eval::constexpr_value_truthy(value, out)) {
      return false;
    }

    binding.explicit_function_nothrow_eval_cached = true;
    binding.explicit_function_nothrow_eval_value = out;
    return true;
  }

  FunctionBinding * select_default_constructor_for_builtin_trait(Scope & scope,
                                                                 ClassInfo & info)
  {
    semantic_class_model::ensure_implicit_special_members(*this, info);
    vector<ExprInfo> source_args;
    vector<ExprInfo> args_out;
    try
    {
      const ConstructorSelectionOptions ctor_options =
          constructor_lifecycle_service::selection_options_for(
              constructor_lifecycle_service::no_user_defined_constructor_profile(
                  "builtin type trait default construction"));
      return select_constructor_from_exprs(scope,
                                           info,
                                           source_args,
                                           args_out,
                                           nullptr,
                                           ctor_options);
    }
    catch(const logic_error &)
    {
      return nullptr;
    }
  }

  string builtin_host_runtime_symbol(const string & builtin_name,
                                     const vector<TypePtr> & params) const
  {
    const auto floating_builtin_suffix =
        [&]() -> string
        {
          if(params.empty()) {
            return string();
          }
          TypePtr base = strip_top_level_cv(params[0]);
          if(!base || base->kind != Type::TK_FUNDAMENTAL) {
            return string();
          }
          switch(base->fundamental) {
          case FT_FLOAT:
            return "f";
          case FT_LONG_DOUBLE:
            return "l";
          case FT_DOUBLE:
            return string();
          default:
            return string();
          }
        };
    const auto is_align_val_type =
        [&](size_t index) -> bool
        {
          if(index >= params.size()) {
            return false;
          }
          TypePtr base = strip_top_level_cv(params[index]);
          return base &&
                 base->kind == Type::TK_NAMED &&
                 base->named_key == "enum class std::__1::align_val_t";
        };

    if(builtin_name == "__builtin_ceil") return "cppgm_builtin_ceil";
    if(builtin_name == "__builtin_ceilf") return "cppgm_builtin_ceilf";
    if(builtin_name == "__builtin_ceill") return "cppgm_builtin_ceill";
    if(builtin_name == "__builtin_fabs") return "cppgm_builtin_fabs";
    if(builtin_name == "__builtin_fabsf") return "cppgm_builtin_fabsf";
    if(builtin_name == "__builtin_fabsl") return "cppgm_builtin_fabsl";
    if(builtin_name == "__builtin_inf") return "cppgm_builtin_inf";
    if(builtin_name == "__builtin_inff") return "cppgm_builtin_inff";
    if(builtin_name == "__builtin_infl") return "cppgm_builtin_infl";
    if(builtin_name == "__builtin_is_constant_evaluated") {
      return "cppgm_builtin_is_constant_evaluated";
    }
    if(builtin_name == "__builtin_isfinite") {
      return string("cppgm_builtin_isfinite") + floating_builtin_suffix();
    }
    if(builtin_name == "__builtin_isinf") {
      return string("cppgm_builtin_isinf") + floating_builtin_suffix();
    }
    if(builtin_name == "__builtin_isnan") {
      return string("cppgm_builtin_isnan") + floating_builtin_suffix();
    }
    if(builtin_name == "__builtin_isnormal") {
      return string("cppgm_builtin_isnormal") + floating_builtin_suffix();
    }
    if(builtin_name == "__builtin_memchr") return "cppgm_builtin_memchr";
    if(builtin_name == "__builtin_memcmp") return "cppgm_builtin_memcmp";
    if(builtin_name == "__builtin_memcpy") return "cppgm_builtin_memcpy";
    if(builtin_name == "__builtin_memmove") return "cppgm_builtin_memmove";
    if(builtin_name == "__builtin_expect") return "cppgm_builtin_expect";
    if(builtin_name == "__builtin_strcmp") return "cppgm_builtin_strcmp";
    if(builtin_name == "__builtin_strlen") return "cppgm_builtin_strlen";
    if(builtin_name == "__builtin_unreachable") {
      return "cppgm_builtin_unreachable";
    }
    if(builtin_name == "operator new") {
      if(params.size() == 1) return "cppgm_builtin_operator_new";
      if(params.size() == 2 && is_align_val_type(1)) {
        return "cppgm_builtin_operator_new_aligned";
      }
    }
    if(builtin_name == "operator new[]") {
      if(params.size() == 1) return "cppgm_builtin_operator_new_array";
      if(params.size() == 2 && is_align_val_type(1)) {
        return "cppgm_builtin_operator_new_array_aligned";
      }
    }
    if(builtin_name == "operator delete") {
      if(params.size() == 1) return "cppgm_builtin_operator_delete";
      if(params.size() == 2 && !is_align_val_type(1)) {
        return "cppgm_builtin_operator_delete_sized";
      }
      if(params.size() == 2 && is_align_val_type(1)) {
        return "cppgm_builtin_operator_delete_aligned";
      }
      if(params.size() == 3) {
        return "cppgm_builtin_operator_delete_sized_aligned";
      }
    }
    if(builtin_name == "operator delete[]") {
      if(params.size() == 1) return "cppgm_builtin_operator_delete_array";
      if(params.size() == 2 && !is_align_val_type(1)) {
        return "cppgm_builtin_operator_delete_array_sized";
      }
      if(params.size() == 2 && is_align_val_type(1)) {
        return "cppgm_builtin_operator_delete_array_aligned";
      }
      if(params.size() == 3) {
        return "cppgm_builtin_operator_delete_array_sized_aligned";
      }
    }
    if(builtin_name == "__builtin_operator_new") {
      if(params.size() == 1) return "cppgm_builtin_operator_new";
      if(params.size() == 2 && is_align_val_type(1)) {
        return "cppgm_builtin_operator_new_aligned";
      }
    }
    if(builtin_name == "__builtin_operator_delete") {
      if(params.size() == 1) return "cppgm_builtin_operator_delete";
      if(params.size() == 2 && !is_align_val_type(1)) {
        return "cppgm_builtin_operator_delete_sized";
      }
      if(params.size() == 2 && is_align_val_type(1)) {
        return "cppgm_builtin_operator_delete_aligned";
      }
      if(params.size() == 3) {
        return "cppgm_builtin_operator_delete_sized_aligned";
      }
    }
    if(builtin_name.compare(0, 10, "__builtin_") == 0) {
      return builtin_name.substr(10);
    }
    return string();
  }

  void apply_builtin_host_runtime_symbol(FunctionBinding & binding,
                                         const vector<TypePtr> & params)
  {
    const string builtin_symbol =
        builtin_host_runtime_symbol(binding.display_name.empty() ?
                                        binding.name :
                                        binding.display_name,
                                    params);
    if(builtin_symbol.empty()) {
      return;
    }
    binding.symbol.object_symbol = builtin_symbol;
  }

  void register_builtin_function(Scope & scope,
                                 const string & name,
                                 const TypePtr & result_type,
                                 const vector<TypePtr> & params,
                                 bool explicit_nothrow)
  {

    vector<pair<string, TypePtr> > named_params;
    for(size_t i = 0; i < params.size(); ++i) {
      named_params.push_back(make_pair(string("arg") + to_string(i), params[i]));
    }
    const TypePtr function_type = make_function(result_type, params, false);
    register_function(scope,
                      name,
                      function_type,
                      named_params,
                      nullptr,
                      nullptr,
                      nullptr,
                      false);
    vector<FunctionBinding *> & slot = semantic_lookup::direct_function_set_slot(scope, name);
    FunctionBinding * binding = slot.empty() ? nullptr : slot.back();
    if(!binding) {
      throw logic_error("missing builtin function binding " + name);
    }
    if(explicit_nothrow) {
      binding->explicit_function_nothrow_kind = EFNK_ALWAYS_TRUE;
      binding->explicit_function_nothrow_cached_qualifier = binding->function_qualifier;
      binding->explicit_function_nothrow_eval_cached = true;
      binding->explicit_function_nothrow_eval_value = true;
    }
    apply_builtin_host_runtime_symbol(*binding, params);
  }

  FunctionBinding * register_function_entity(const FunctionRegistrationRequest & request)
  {
    const FunctionTemplateRegistrationIdentity & template_identity =
        request.template_identity;
    if(request.owner_class) {
      if(request.is_static_member) {
        return register_class_static_function(*request.owner_class,
                                              request.name,
                                              request.declared_type,
                                              request.params,
                                              request.default_arguments,
                                              request.body,
                                              request.semantic_flags,
                                              template_identity,
                                              request.declaration_node,
                                              request.parameter_syntax_node,
                                              request.function_qualifier,
                                              template_identity.prefer_overload_suffix);
      }
      return register_class_function(*request.owner_class,
                                     request.name,
                                     request.declared_type,
                                     request.params,
                                     request.default_arguments,
                                     request.body,
                                     request.ctor_initializer,
                                     request.semantic_flags,
                                     template_identity,
                                     request.declaration_node,
                                     request.parameter_syntax_node,
                                     template_identity.prefer_overload_suffix);
    }

    if(!request.scope) {
      throw logic_error("function registration request missing scope");
    }
    register_function(*request.scope,
                      request.name,
                      request.declared_type,
                      request.params,
                      request.default_arguments,
                      request.body,
                      request.declaration_node,
                      request.parameter_syntax_node,
                      request.is_c_linkage,
                      request.declaration_scope,
                      request.function_qualifier,
                      request.is_constexpr,
                      template_identity,
                      template_identity.prefer_overload_suffix,
                      request.lexical_access_class,
                      request.lexical_access_function);
    return find_exact_function(*request.scope, request.name, request.declared_type);
  }

  void build_function_template_parse_view(const FunctionTemplateDecl & decl,
                                          CppAstNode & specifiers,
                                          CppAstNode & declarator)
  {
    if(decl.is_lambda_call_operator_template) {
      specifiers = *decl.specifiers;
      declarator = *decl.declarator;
      return;
    }
    const bool method_like_template =
        decl.declaring_scope && decl.declaring_scope->class_info;
    if(method_like_template) {
      PreparedMethodParseContext prepared_method;
      semantic_class_model::prepare_method_parse_context(decl.specifiers,
                                                         *decl.declarator,
                                                         prepared_method,
                                                         true,
                                                         true);
      if(const CppAstNode * parse_specifiers = prepared_method.parse_specifiers_node()) {
        specifiers = *parse_specifiers;
      } else {
        specifiers = CppAstNode();
        specifiers.kind = CppAstKind::decl_specifier_seq;
      }
      declarator = prepared_method.parse_declarator_node();
      return;
    }
    if(!decl.friend_access_classes.empty() && decl.specifiers && decl.declarator) {
      for(size_t i = 0; i < decl.friend_access_classes.size(); ++i) {
        ClassInfo * owner = decl.friend_access_classes[i];
        if(!owner || !owner->member_scope) {
          continue;
        }

        CppAstNode resolved_specifiers;
        if(!prepare_namespace_scope_specifiers(*owner->member_scope,
                                               *decl.specifiers,
                                               nullptr,
                                               true,
                                               true,
                                               resolved_specifiers)) {
          continue;
        }

        PreparedMethodParseContext prepared_friend_method;
        semantic_class_model::prepare_method_parse_context(&resolved_specifiers,
                                                           *decl.declarator,
                                                           prepared_friend_method,
                                                           true,
                                                           true);
        specifiers = filtered_class_member_decl_specifiers(resolved_specifiers);
        declarator = prepared_friend_method.parse_declarator_node();
        return;
      }
    }
    if(decl.specifiers) {
      specifiers = *decl.specifiers;
    } else {
      specifiers = CppAstNode();
      specifiers.kind = CppAstKind::decl_specifier_seq;
    }
    declarator = *decl.declarator;
  }

  CppAstNode filter_function_declarator(const CppAstNode & declarator) const
  {
    return filtered_function_declarator(declarator);
  }

  void track_instantiated_class(ClassInfo * info)
  {
    callsemantic::TypeRegistryState state = type_registry_state();
    callsemantic::track_instantiated_class(state, info);
  }

  bool evaluate_sizeof_operand_for_consteval(Scope & scope,
                                             const CppAstNode & expr,
                                             size_t & size)
  {
    TypePtr type;
    try
    {
      const CppAstNode * operand = &expr;
      while(operand->kind == CppAstKind::parenthesized_expression &&
            operand->children.size() == 1) {
        operand = &operand->children[0];
      }
      ExprInfo analyzed =
          operand->kind == CppAstKind::call_expression ?
              analyze_call_expression(
                  scope, *operand, semantic_policy::without_body_instantiation()) :
              analyze_expression(scope, *operand);
      type = analyzed.type;
    }
    catch(const logic_error &)
    {
    }
    const QualifiedName * qualified =
        expr.kind == CppAstKind::id_expression ? cppast_qualified_name_syntax(expr) : nullptr;
    const ValueBinding * binding =
        expr.kind == CppAstKind::id_expression ?
            (qualified && (qualified->rooted || !qualified->qualifiers.empty()) ?
                 semantic_lookup::lookup_qualified_value_binding(*this, scope, *qualified) :
                 lookup_value(scope, expr.value)) :
            nullptr;
    if(!type && expr.kind == CppAstKind::id_expression && !binding) {
      type = lookup_type(scope, expr.value);
    }
    maybe_complete_sizeof_type(*this, type);
    if(!type || !type_is_complete(type)) {
      return false;
    }
    if(sizeof_depends_on_template_parameters(type) &&
       scope_has_template_placeholders(scope)) {
      return false;
    }
    TypePtr sizeof_type = remove_reference_type(type);
    if(!sizeof_type) {
      sizeof_type = type;
    }
    size = type_size(sizeof_type);
    return true;
  }

  bool lookup_offsetof_field(const TypePtr & object_type,
                             const string & member_name,
                             semantic_consteval::OffsetofFieldInfo & out)
  {
    ClassInfo * info = complete_class_type(strip_top_level_cv(object_type));
    if(!info || !info->complete) {
      return false;
    }

    MemberValueLookupResult found = lookup_member_value(*info, member_name);
    if(!found.binding || found.binding->kind != ValueBinding::VK_FIELD) {
      return false;
    }

    out.found = true;
    out.is_bit_field = found.binding->is_bit_field;
    out.offset = found.path_offset + found.binding->field_offset;
    return true;
  }

  bool parse_type_specifier_seq(Scope & scope,
                                const CppAstNode & node,
                                TypePtr & out,
                                bool reference_class_templates_only = false)
  {
    return parse_type_specifier_seq_ast(node,
                                        make_decl_hooks(scope, reference_class_templates_only),
                                        out);
  }

  std::string stable_template_parameter_witness_name(
      const TemplateParameterInfo & parameter,
      std::size_t index) const
  {
    switch(parameter.kind) {
    case TemplateParameterInfo::TP_NON_TYPE:
      return std::string("value-parameter-0-") + std::to_string(index);
    case TemplateParameterInfo::TP_TEMPLATE_TEMPLATE:
      return std::string("template-parameter-0-") + std::to_string(index);
    case TemplateParameterInfo::TP_TYPE:
    default:
      return std::string("type-parameter-0-") + std::to_string(index);
    }
  }

  bool type_argument_template_parameter_witness_name(
      const TypePtr & type,
      const std::vector<TemplateParameterInfo> & canonical_parameters,
      std::string & out) const
  {
    if(!type) {
      return false;
    }
    switch(type->kind) {
    case Type::TK_NAMED:
      if(type->named_semantic_kind != Type::NSK_TEMPLATE_PARAMETER) {
        return false;
      }
      for(std::size_t i = 0; i < canonical_parameters.size(); ++i) {
        const TemplateParameterInfo & parameter = canonical_parameters[i];
        if(parameter.kind != TemplateParameterInfo::TP_TYPE) {
          continue;
        }
        if(type->named_key == parameter.placeholder_key) {
          out = stable_template_parameter_witness_name(parameter, i);
          return true;
        }
      }
      return false;

    case Type::TK_CV:
      if(!type_argument_template_parameter_witness_name(type->inner,
                                                        canonical_parameters,
                                                        out)) {
        return false;
      }
      if(type->cv_const) {
        out += " const";
      }
      if(type->cv_volatile) {
        out += " volatile";
      }
      return true;

    case Type::TK_POINTER:
      if(!type_argument_template_parameter_witness_name(type->inner,
                                                        canonical_parameters,
                                                        out)) {
        return false;
      }
      out += " *";
      return true;

    case Type::TK_LVALUE_REFERENCE:
      if(!type_argument_template_parameter_witness_name(type->inner,
                                                        canonical_parameters,
                                                        out)) {
        return false;
      }
      out += " &";
      return true;

    case Type::TK_RVALUE_REFERENCE:
      if(!type_argument_template_parameter_witness_name(type->inner,
                                                        canonical_parameters,
                                                        out)) {
        return false;
      }
      out += " &&";
      return true;

    case Type::TK_ARRAY:
      if(!type_argument_template_parameter_witness_name(type->inner,
                                                        canonical_parameters,
                                                        out)) {
        return false;
      }
      if(type->has_bound) {
        out += "[" + std::to_string(type->bound) + "]";
      } else if(!type->bound_text.empty()) {
        out += "[" + type->bound_text + "]";
      } else {
        out += "[]";
      }
      return true;

    default:
      return false;
    }
  }

  void canonicalize_template_parameter_source_bindings(
      std::vector<witness::TemplateWitnessSourceBinding> & bindings,
      const std::vector<TemplateArgument> & arguments,
      const std::vector<TemplateParameterInfo> & canonical_parameters) const
  {
    const std::size_t count = std::min(bindings.size(), arguments.size());
    for(std::size_t i = 0; i < count; ++i) {
      if(arguments[i].kind != TemplateArgument::TA_TYPE) {
        continue;
      }
      std::string canonical;
      if(type_argument_template_parameter_witness_name(arguments[i].type,
                                                       canonical_parameters,
                                                       canonical)) {
        bindings[i].arg = canonical;
      }
    }
  }

  bool record_dependent_pattern_class_use_for_template_id_syntax(
      Scope & scope,
      const CppAstNode & node,
      const TemplateIdSyntax & syntax)
  {
    if(!witness::source_capture_enabled(template_witness_context()) ||
       syntax.name.name.empty()) {
      return false;
    }
    ClassTemplateDecl * class_template = lookup_class_template(scope, syntax.name);
    if(!class_template) {
      return false;
    }

    Scope * argument_scope = &scope;
    const std::vector<TemplateParameterInfo> * canonical_parameters = nullptr;
    if(scope.class_info &&
       scope.class_info->source_template == class_template &&
       scope.class_info->template_output_node) {
      for(size_t i = 0; i < class_template->partial_specializations.size(); ++i) {
        PartialClassTemplateSpecializationDecl & partial =
            class_template->partial_specializations[i];
        if(partial.class_node == scope.class_info->template_output_node &&
           partial.pattern_scope) {
          argument_scope = partial.pattern_scope;
          canonical_parameters = &partial.parameters;
          break;
        }
      }
    }

    std::vector<TemplateArgument> arguments;
    if(!resolve_template_arguments(*argument_scope,
                                   class_template->parameters,
                                   syntax.arguments,
                                   &syntax.argument_syntaxes,
                                   arguments,
                                   class_template->declaring_scope) ||
       !template_arguments_are_dependent(arguments)) {
      return false;
    }
    canonicalize_simple_dependent_argument_texts(arguments);
    const std::string key = template_args_identity_key(arguments);
    const template_api::specialization::ClassSpecializationSelection selection =
        template_api::specialization::select_class_specialization(
            *this,
            *class_template,
            *argument_scope,
            key,
            arguments,
            &syntax.arguments);
    if(selection.kind == template_api::MS_PRIMARY) {
      return false;
    }

    const std::string anchor_identifier =
        !unqualified_member_name(syntax.name.name).empty() ?
            unqualified_member_name(syntax.name.name) :
            syntax.name.name;
    std::string use_location =
        template_api::normalize_template_witness_source_location(
            template_api::template_witness_detail::
                source_location_for_location_id(template_witness_context(),
                                                syntax.source_location_id));
    if(use_location.empty()) {
      use_location =
          template_api::normalize_template_witness_source_location(
              source_location_for_name_in_subtree(node, anchor_identifier));
    }
    if(!source_location_points_at_identifier(use_location, anchor_identifier)) {
      return false;
    }

    const witness::TemplateWitnessSourceAnchor selected_decl_anchor =
        class_use_selected_decl_anchor(class_template, selection);
    if(!selected_decl_anchor.location.empty() &&
       use_location == selected_decl_anchor.location) {
      return false;
    }

    witness::ClassUseEmitRequest request;
    request.location = use_location;
    request.template_name =
        strip_trailing_top_level_template_arguments(
            qualified_name_syntax_text(syntax.name));
    request.selection = witness::SourceSelectionKind::ExplicitSpecialization;
    request.use_anchor_present = true;
    request.use_anchor_location = use_location;
    request.selected_decl_location = selected_decl_anchor.location;
    request.selected_decl_anchor = selected_decl_anchor;
    request.template_id_occurrence =
        witness::make_source_template_id_occurrence(use_location,
                                                    syntax.arguments);
    request.template_id_occurrence.in_template_body =
        scope_has_template_placeholders(scope) ||
        scope_is_inside_source_template_context(scope) ||
        witness::current_template_witness_entry_context().origin ==
            witness::TemplateWitnessOrigin::Closure;
    request.template_id_occurrence.current_specialization_use =
        request.template_id_occurrence.in_template_body &&
        !selected_decl_anchor.location.empty() &&
        source_location_is_later(selected_decl_anchor.location, use_location);
    request.template_id_occurrence.has_dependent_argument = true;
    for(std::size_t i = 0;
        i < request.template_id_occurrence.arguments.size();
        ++i) {
      bool dependent_argument = false;
      if(i < arguments.size()) {
        const TemplateArgument & argument = arguments[i];
        dependent_argument =
            (argument.kind == TemplateArgument::TA_TYPE &&
             type_depends_on_template_parameter(argument.type)) ||
            (argument.kind == TemplateArgument::TA_VALUE &&
             argument.dependent) ||
            ((argument.kind == TemplateArgument::TA_CLASS_TEMPLATE ||
              argument.kind == TemplateArgument::TA_ALIAS_TEMPLATE) &&
             (argument.dependent || argument.template_decl == nullptr));
      }
      request.template_id_occurrence.arguments[i].dependent =
          dependent_argument;
    }
    template_api::append_template_witness_source_bindings(
        *this,
        request.bindings,
        class_template->parameters,
        arguments,
        "explicit");
    if(canonical_parameters) {
      canonicalize_template_parameter_source_bindings(request.bindings,
                                                      arguments,
                                                      *canonical_parameters);
    }
    request.ownership = witness::SourceUseOwnership::SourceOwned;
    witness::emit_class_use(request);
    return true;
  }

  void record_deduced_class_use_for_resolved_alias_type(
      Scope & scope,
      const TypePtr & type,
      const std::string & use_location)
  {
    if(use_location.empty()) {
      return;
    }
    template_api::ClassTemplateUseInfo use;
    if(!template_api::class_template_use_info_for_type(*this,
                                                                 scope,
                                                                 type,
                                                                 use)) {
      return;
    }
    witness::ClassUseEmitRequest request;
    request.location = use_location;
    request.template_name = use.template_name;
    request.selection = source_selection_kind_for_match_kind(use.selection.kind);
    witness::set_selected_decl_anchor(
        request.selected_decl_location,
        request.selected_decl_anchor,
        class_use_selected_decl_anchor(use.origin, use.selection));
    template_api::append_template_witness_source_bindings(
        *this,
        request.bindings,
        *use.parameters,
        *use.arguments,
        "deduced");
    if(use.selection.parameters &&
       use.selection.parameters != use.parameters) {
      template_api::append_template_witness_source_bindings(
          *this,
          request.specialization_bindings,
          *use.selection.parameters,
          use.selection.arguments,
          "deduced");
    }
    request.ownership = witness::SourceUseOwnership::SourceOwned;
    witness::emit_class_use(request);
  }

  void record_class_use_for_resolved_type_node(
      Scope & scope,
      const CppAstNode & node,
      const TypePtr & type,
      const std::string & node_use_location,
      bool allow_concrete_dependent_argument_spelling = false)
  {
    if(!witness::source_capture_enabled(template_witness_context())) {
      return;
    }
    template_api::ClassTemplateUseInfo use;
    std::string type_text = trim_space(node_text(node));
    std::vector<std::string> explicit_arg_texts;
    if(!template_api::class_template_use_info_for_type(*this,
                                                                 scope,
                                                                 type,
                                                                 use)) {
      return;
    }
    const TemplateIdSyntax * type_template_syntax =
        template_id_syntax_for_anchor(node, use.origin->name);
    const bool has_template_id_syntax = type_template_syntax != nullptr;
    QualifiedName fallback_template_name;
    std::vector<std::string> fallback_arg_texts;
    const bool has_fallback_template_id =
        !has_template_id_syntax &&
        template_id_value_for_anchor(node,
                                     use.origin->name,
                                     fallback_template_name,
                                     fallback_arg_texts);
    const bool has_template_id =
        has_template_id_syntax || has_fallback_template_id;
    if(!has_template_id) {
      return;
    }
    explicit_arg_texts =
        has_template_id_syntax ?
            template_id_argument_texts_preserving_spacing(*type_template_syntax) :
            fallback_arg_texts;
    const bool arguments_mention_source_bindings =
        template_argument_texts_mention_source_bindings(*this,
                                                       scope,
                                                       explicit_arg_texts);
    const bool arguments_mention_local_aliases =
        template_argument_texts_mention_instantiated_class_local_type_aliases(
            scope,
            explicit_arg_texts);
    const bool arguments_mention_enclosing_parameters =
        template_argument_texts_mention_enclosing_source_template_parameters(
            scope,
            explicit_arg_texts);
    const bool arguments_mention_bound_scope_names =
        template_argument_texts_mention_template_bound_scope_names(
            scope,
            explicit_arg_texts);
    const bool source_syntax_mentions_current_class_member =
        has_template_id_syntax &&
        template_argument_syntaxes_mention_unqualified_current_class_member(
            scope,
            type_template_syntax->argument_syntaxes,
            use.origin);
    const bool source_syntax_mentions_any_current_class_member =
        has_template_id_syntax &&
        template_argument_syntaxes_mention_unqualified_current_class_member(
            scope,
            type_template_syntax->argument_syntaxes,
            nullptr,
            true);
    const bool arguments_mention_current_specialization =
        template_argument_texts_mention_current_specialization_names(
            scope,
            explicit_arg_texts) ||
        source_syntax_mentions_current_class_member;
    std::vector<std::string> binding_arg_texts = explicit_arg_texts;
    bool resolved_concrete_argument_spellings = false;
    if(allow_concrete_dependent_argument_spelling &&
       use.arguments->size() == explicit_arg_texts.size()) {
      binding_arg_texts.clear();
      for(std::size_t i = 0; i < use.arguments->size(); ++i) {
        binding_arg_texts.push_back(
            template_argument_text((*use.arguments)[i]));
      }
      resolved_concrete_argument_spellings = true;
    }
    if((arguments_mention_source_bindings &&
        !resolved_concrete_argument_spellings) ||
       (arguments_mention_local_aliases &&
        !resolved_concrete_argument_spellings) ||
       (arguments_mention_enclosing_parameters &&
        !resolved_concrete_argument_spellings) ||
       (arguments_mention_bound_scope_names &&
        !resolved_concrete_argument_spellings) ||
       (source_syntax_mentions_any_current_class_member &&
        !resolved_concrete_argument_spellings)) {
      return;
    }
    const std::string spelled_template_name =
        has_template_id_syntax && !type_template_syntax->name.name.empty() ?
            type_template_syntax->name.name :
            (!fallback_template_name.name.empty() ?
                 qualified_name_syntax_text(fallback_template_name) :
                 type_text);
    const std::string resolved_template_name = use.unqualified_name;
    if(strip_trailing_top_level_template_arguments(spelled_template_name) !=
       strip_trailing_top_level_template_arguments(resolved_template_name)) {
      return;
    }
    std::string use_location =
        template_api::normalize_template_witness_source_location(
            source_location_for_name_in_node(node,
                                             resolved_template_name));
    if(use_location.empty() &&
       source_location_points_at_identifier(node_use_location,
                                           resolved_template_name)) {
      use_location =
          template_api::normalize_template_witness_source_location(
              node_use_location);
    }
    if(use_location.empty() && has_template_id_syntax) {
      use_location =
          template_api::normalize_template_witness_source_location(
              template_api::template_witness_detail::
                  source_location_for_location_id(
                      template_witness_context(),
                      type_template_syntax->source_location_id));
    }
    if(use_location.empty() ||
       !source_location_points_at_identifier(use_location,
                                            resolved_template_name)) {
      return;
    }
    witness::ClassUseEmitRequest request;
    request.location = use_location;
    request.template_name = use.template_name;
    request.selection = source_selection_kind_for_match_kind(use.selection.kind);
    request.use_anchor_present = true;
    request.use_anchor_location = use_location;
    witness::set_selected_decl_anchor(
        request.selected_decl_location,
        request.selected_decl_anchor,
        class_use_selected_decl_anchor(use.origin, use.selection));
    request.template_id_occurrence.present = true;
    request.template_id_occurrence.source_spelled = true;
    request.template_id_occurrence.argument_list_spelled = true;
    request.template_id_occurrence.empty_argument_list =
        explicit_arg_texts.empty();
    request.template_id_occurrence.in_template_body =
        scope_has_template_placeholders(scope) ||
        scope_is_inside_source_template_context(scope) ||
        witness::current_template_witness_entry_context().origin ==
            witness::TemplateWitnessOrigin::Closure;
    request.template_id_occurrence.has_dependent_argument =
        ((arguments_mention_source_bindings ||
          arguments_mention_local_aliases ||
          arguments_mention_enclosing_parameters) &&
         !resolved_concrete_argument_spellings) ||
        arguments_mention_bound_scope_names;
    request.template_id_occurrence.has_current_specialization_argument =
        arguments_mention_current_specialization &&
        !resolved_concrete_argument_spellings;
    request.template_id_occurrence.name_anchor.location = use_location;
    request.template_id_occurrence.name_anchor.kind =
        semantic_source_use::SourceAnchorKind::Spelling;
    for(std::size_t i = 0; i < explicit_arg_texts.size(); ++i) {
      std::vector<std::string> arg_texts;
      arg_texts.push_back(explicit_arg_texts[i]);
      semantic_source_use::SourceTemplateArgumentOccurrence argument;
      argument.text = explicit_arg_texts[i];
      argument.source_spelled = true;
      argument.dependent =
          ((template_argument_texts_mention_source_bindings(*this,
                                                           scope,
                                                           arg_texts) ||
            template_argument_texts_mention_instantiated_class_local_type_aliases(
                scope,
                arg_texts) ||
            template_argument_texts_mention_enclosing_source_template_parameters(
                scope,
                arg_texts)) &&
           !resolved_concrete_argument_spellings) ||
          template_argument_texts_mention_template_bound_scope_names(
              scope,
              arg_texts);
      argument.current_specialization =
          (template_argument_texts_mention_current_specialization_names(
               scope,
               arg_texts) ||
          (has_template_id_syntax &&
            i < type_template_syntax->argument_syntaxes.size() &&
            template_argument_syntax_mentions_unqualified_current_class_member(
                scope,
                type_template_syntax->argument_syntaxes[i],
                use.origin))) &&
          !resolved_concrete_argument_spellings;
      if(argument.current_specialization &&
         has_template_id_syntax &&
         i < type_template_syntax->argument_syntaxes.size()) {
        std::string semantic_text;
        if(current_specialization_source_argument_semantic_text(
               *this,
               scope,
               type_template_syntax->argument_syntaxes[i],
               semantic_text) &&
           !callsemantic::template_argument_syntax_contains_template_id(
               type_template_syntax->argument_syntaxes[i])) {
          argument.semantic_text = semantic_text;
        }
      }
      if(has_template_id_syntax &&
         i < type_template_syntax->argument_syntaxes.size()) {
        collect_template_argument_value_references(
            scope,
            type_template_syntax->argument_syntaxes[i],
            argument.referenced_value_entities,
            argument.referenced_value_decl_locations);
      }
      if(i < use.arguments->size() &&
         (argument.referenced_value_entities.empty() ||
          argument.referenced_value_decl_locations.empty())) {
        collect_template_argument_value_references(
            scope,
            (*use.arguments)[i],
            argument.referenced_value_entities,
            argument.referenced_value_decl_locations);
      }
      request.template_id_occurrence.arguments.push_back(argument);
    }
    template_api::append_template_witness_source_bindings(
        *this,
        request.bindings,
        *use.parameters,
        *use.arguments,
        binding_arg_texts,
        "explicit",
        "defaulted");
    if(use.selection.parameters &&
       use.selection.parameters != use.parameters) {
      template_api::append_template_witness_source_bindings(
          *this,
          request.specialization_bindings,
          *use.selection.parameters,
          use.selection.arguments,
          "deduced");
    }
    request.ownership = witness::SourceUseOwnership::SourceOwned;
    const bool parent_class_use_records =
        witness::class_use_recording_enabled(request.origin);
    witness::emit_class_use(request);
    if(parent_class_use_records && has_template_id_syntax) {
      emit_nested_class_use_source_events_from_syntaxes(scope,
                                                        type_template_syntax->argument_syntaxes,
                                                        witness::SourceUseOwnership::SourceOwned,
                                                        std::string());
    } else if(parent_class_use_records) {
      emit_nested_class_use_source_events_from_location(scope,
                                                        request.location,
                                                        witness::SourceUseOwnership::SourceOwned,
                                                        request.template_name);
    }
  }

  bool parse_decl_spec(const CppAstNode & node,
                       Scope & scope,
                       bool & is_typedef,
                       TypePtr & out,
                       bool reference_class_templates_only = false)
  {
    const std::string node_use_location = prefer_later_source_location(
        source_location_for_node(node),
        preferred_source_location_for_node(node));
    const ScopedTemplateUseLocation use_location_guard(
        !node_use_location.empty() ?
            node_use_location :
            template_public_use_location_or(node_use_location));
    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "parse-decl-spec loc=" << source_location_for_node(node)
            << " scope=" << scope_qualified_name(scope, "<here>")
            << " ref-only=" << (reference_class_templates_only ? "yes" : "no")
            << " text=" << node_text(node);
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
    const bool parsed =
        parse_decl_spec_ast(node,
                            make_decl_hooks(scope, reference_class_templates_only),
                            is_typedef,
                            out);
    if(parsed) {
      record_class_use_for_resolved_type_node(scope,
                                              node,
                                              out,
                                              node_use_location);
    }
    return parsed;
  }

  bool prepare_namespace_scope_specifiers(Scope & scope,
                                          const CppAstNode & specifiers,
                                          const CppAstNode * declarators,
                                          bool collect_embedded_types,
                                          bool collect_named_forward_declarations,
                                          CppAstNode & out)
  {
    out = specifiers;
    const auto visible_named_type = [this, &scope](const string & lookup_name) -> TypePtr
    {
      for(Scope * current = &scope; current; current = current->parent) {
        map<string, TypePtr>::const_iterator found = current->named_types.find(lookup_name);
        if(found != current->named_types.end()) {
          return found->second;
        }
        if(current->class_info) {
          MemberTypeLookupResult member =
              lookup_member_type(*this, *current->class_info, lookup_name, true, &scope);
          if(member.type) {
            return member.type;
          }
        }
      }
      return TypePtr();
    };

    for(size_t i = 0; i < out.children.size(); ++i) {
      CppAstNode & child = out.children[i];
      if(child.kind != CppAstKind::class_specifier &&
         child.kind != CppAstKind::class_forward_declaration &&
         child.kind != CppAstKind::enum_specifier) {
        continue;
      }

      const bool synthesized_embedded_name = child.value.empty();
      string effective_name = child.value;
      if(effective_name.empty()) {
        if(!declarators || declarators->children.size() != 1) {
          return false;
        }
        const CppAstNode & init_decl = declarators->children[0];
        if(init_decl.kind != CppAstKind::init_declarator || init_decl.children.empty() ||
           !declarator_is_plain_identifier(init_decl.children[0], effective_name)) {
          return false;
        }
      }

      const bool should_collect =
          child.kind == CppAstKind::enum_specifier ||
          child.kind == CppAstKind::class_specifier ||
          child.value.empty() ||
          (collect_named_forward_declarations &&
           child.kind == CppAstKind::class_forward_declaration &&
           !visible_named_type(effective_name));

      if(synthesized_embedded_name) {
        TypePtr existing = direct_named_type(scope, effective_name);
        ClassInfo * existing_info = class_info_for_type(existing);
        if(existing_info && existing_info->complete) {
          child.value = effective_name;
          continue;
        }
      }

      if(collect_embedded_types && should_collect) {
        CppAstNode named_child = child;
        named_child.value = effective_name;
        const CppAstNode * durable_named_child =
            own_synthetic_ast(std::move(named_child));
        if(child.kind == CppAstKind::enum_specifier) {
          collect_enum_declaration(scope, *durable_named_child);
        } else {
          collect_class_declaration(scope, *durable_named_child);
          if(synthesized_embedded_name) {
            if(ClassInfo * info = lookup_declared_class_info(scope, effective_name)) {
              info->source_is_unnamed_class = true;
              info->source_unnamed_class_node =
                  info->class_node ? info->class_node : durable_named_child;
            }
          }
        }
      }

      child.value = effective_name;
    }
    return true;
  }

  bool prepare_namespace_scope_declaration_specifiers(
      Scope & scope,
      const CppAstNode & specifiers,
      const CppAstNode * declarators,
      bool collect_embedded_types,
      bool collect_named_forward_declarations,
      PreparedDeclarationSpecifiers & out)
  {
    out = PreparedDeclarationSpecifiers();
    if(!prepare_namespace_scope_specifiers(scope,
                                           specifiers,
                                           declarators,
                                           collect_embedded_types,
                                           collect_named_forward_declarations,
                                           out.resolved_specifiers)) {
      return false;
    }
    out.has_auto = decl_spec_contains_auto(out.resolved_specifiers);
    const bool declaration_is_typedef =
        decl_spec_contains_token(out.resolved_specifiers, KW_TYPEDEF);
    out.parsed_decl_spec =
        out.has_auto ||
        parse_decl_spec(out.resolved_specifiers,
                        scope,
                        out.declaration_is_typedef,
                        out.base,
                        declaration_is_typedef);
    return true;
  }

  bool parse_parameter_clause(Scope & scope,
                              const CppAstNode & node,
                              vector<pair<string, TypePtr> > & params,
                              vector<const CppAstNode *> * default_args_out = nullptr,
                              bool reference_class_templates_only = false)
  {
    const std::string earliest_child_use_location =
        earliest_child_source_location_for_node(node);
    const std::string parameter_use_location =
        !earliest_child_use_location.empty() ?
            earliest_child_use_location :
            prefer_later_source_location(
                template_public_use_location_or(std::string()),
                preferred_source_location_for_node(node));
    const ScopedTemplateUseLocation use_location_guard(parameter_use_location);

    if(parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "parse-parameter-clause loc=" << source_location_for_node(node)
            << " scope=" << scope_qualified_name(scope, "<here>")
            << " ref-only=" << (reference_class_templates_only ? "yes" : "no")
            << " params=" << node.children.size()
            << " text=" << node_text(node);
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
    const bool parsed =
        parse_parameter_clause_ast(node,
                                   make_decl_hooks(scope, reference_class_templates_only),
                                   params,
                                   default_args_out);
    if(!parsed) {
      return false;
    }
    const bool record_reference_parameter_template_uses =
        !reference_class_templates_only ||
        template_api::class_is_explicit_specialization(scope.class_info);
    if(record_reference_parameter_template_uses &&
       witness::source_capture_enabled(template_witness_context())) {
      std::set<std::string> recorded_parameter_template_uses;
      const auto maybe_record_parameter_template_use =
          [&](const CppAstNode & current,
              const std::string & base_location) -> void
      {
        std::string template_text;
        if(!current.value.empty()) {
          template_text = template_lookup_fragment_text(current.value);
        }
        if(template_text.empty()) {
          template_text = template_lookup_fragment_text(node_text(current));
        }
        if(template_text.empty()) {
          return;
        }
        const std::string identifier =
            template_lookup_fragment_identifier(template_text);
        if(identifier.empty()) {
          return;
        }
        const std::vector<WitnessUseLocationSearch> location_searches = {
          WitnessUseLocationSearch(WITNESS_USE_LOCATION_NAME_IN_NODE,
                                   &current),
          WitnessUseLocationSearch(WITNESS_USE_LOCATION_NAME_IN_SUBTREE,
                                   &current)
        };
        std::string exact_location =
            select_witness_use_location(location_searches, identifier);
        if(!source_location_identifier_followed_by(exact_location,
                                                   identifier,
                                                   '<')) {
          exact_location.clear();
        }
        if(exact_location.empty() ||
           !recorded_parameter_template_uses.insert(exact_location).second) {
          return;
        }
        const TemplateIdSyntax * source_syntax =
            template_id_syntax_for_anchor_at_or_after_location(
                template_witness_context(),
                current,
                identifier,
                exact_location);
        witness::ClassUseSourceDecision decision;
        if(!build_class_use_source_decision_from_template_text(
               scope,
               template_text,
               exact_location,
               decision,
               source_syntax)) {
          return;
        }
        if(source_location_points_at_identifier(exact_location, identifier)) {
          witness::set_use_anchor(decision.location,
                                  decision.use_anchor,
                                  exact_location);
        }
        witness::emit_class_use_decision(
            decision,
            witness::SourceUseOwnership::SourceOwned,
            witness::SourceUseRole::TypeUse);
      };
      std::function<void(const CppAstNode &, const std::string &)>
          collect_parameter_template_uses =
              [&](const CppAstNode & current,
                  const std::string & base_location) -> void
      {
        maybe_record_parameter_template_use(current, base_location);
        for(size_t i = 0; i < current.children.size(); ++i) {
          collect_parameter_template_uses(current.children[i], base_location);
        }
      };
      for(size_t i = 0; i < node.children.size(); ++i) {
        const CppAstNode & child = node.children[i];
        if(child.kind != CppAstKind::parameter_declaration) {
          continue;
        }
        const std::string parameter_location =
            template_api::normalize_template_witness_source_location(
                prefer_later_source_location(
                    source_location_for_node(child),
                    earliest_child_source_location_for_node(child)));
        if(parameter_location.empty()) {
          continue;
        }
        const CppAstNode * specifiers =
            find_child(child, CppAstKind::decl_specifier_seq);
        if(specifiers) {
          collect_parameter_template_uses(*specifiers, parameter_location);
        }
      }
    }
    for(size_t i = 0; i < params.size(); ++i) {
      if(params[i].second) {
        continue;
      }
      ostringstream out;
      out << "parse_parameter_clause: null parameter type"
          << " [index " << i << "]"
          << " [loc " << source_location_for_node(node) << "]"
          << " [scope " << scope_qualified_name(scope, "<here>") << "]"
          << " [text " << node_text(node) << "]";
      throw logic_error(out.str());
    }
    return true;
  }

  bool parse_declarator(Scope & scope,
                        const CppAstNode & node,
                        const TypePtr & base,
                        string & name,
                        TypePtr & out,
                        bool reference_class_templates_only = false)
  {
    return parse_declarator_ast(node,
                                make_decl_hooks(scope, reference_class_templates_only),
                                base,
                                name,
                                out);
  }

  bool parse_type_id(Scope & scope,
                     const CppAstNode & node,
                     TypePtr & out,
                     bool reference_class_templates_only = false,
                     bool record_class_template_use = true)
  {
    const std::string node_use_location = prefer_later_source_location(
        source_location_for_node(node),
        preferred_source_location_for_node(node));
    ScopedTemplateUseLocation use_location_guard(preferred_source_location_for_node(node));
    const bool parsed =
        parse_type_id_ast(node,
                          make_decl_hooks(scope, reference_class_templates_only),
                          out);
    if(parsed && record_class_template_use) {
      record_class_use_for_resolved_type_node(scope,
                                              node,
                                              out,
                                              node_use_location);
    }
    return parsed;
  }

  bool parse_abstract_declarator(Scope & scope,
                                 const CppAstNode & node,
                                 const TypePtr & base,
                                 TypePtr & out,
                                 bool reference_class_templates_only = false)
  {
    return parse_abstract_declarator_ast(node,
                                         make_decl_hooks(scope, reference_class_templates_only),
                                         base,
                                         out);
  }

  bool decl_spec_contains_auto(const CppAstNode & node) const
  {
    return decl_spec_contains_token(node, KW_AUTO);
  }

  bool declarator_is_plain_identifier(const CppAstNode & node,
                                      string & name) const
  {
    if(node.kind != CppAstKind::declarator || node.children.size() != 1 ||
       node.children[0].kind != CppAstKind::identifier) {
      return false;
    }
    name = node.children[0].value;
    return !name.empty();
  }

  string next_synthetic_local_name(const string & prefix)
  {
    ostringstream out;
    out << "__" << prefix << ++synthetic_local_counter;
    return out.str();
  }

  string next_anonymous_enum_name()
  {
    ostringstream out;
    out << "__anonymous_enum" << ++anonymous_enum_counter;
    return out.str();
  }

  bool prepare_block_scope_specifiers(Scope & scope,
                                      const CppAstNode & specifiers,
                                      const CppAstNode * declarators,
                                      CppAstNode & out)
  {
    out = specifiers;
    const bool has_declarators = declarators && !declarators->children.empty();
    for(size_t i = 0; i < out.children.size(); ++i) {
      CppAstNode & child = out.children[i];
      if(child.kind != CppAstKind::class_specifier &&
         child.kind != CppAstKind::class_forward_declaration &&
         child.kind != CppAstKind::enum_specifier) {
        continue;
      }

      // In declarations like `struct S x;`, the parser uses a
      // class-forward-declaration node for the elaborated type spelling.
      // That should resolve the existing `S`, not inject a new block-scope
      // declaration that shadows it.
      if(child.kind == CppAstKind::class_forward_declaration &&
         !child.value.empty() &&
         has_declarators) {
        continue;
      }

      const bool synthesized_embedded_name = child.value.empty();
      string effective_name = child.value;
      if(effective_name.empty()) {
        effective_name = next_synthetic_local_name("local_type");
      }

      if(synthesized_embedded_name) {
        TypePtr existing = direct_named_type(scope, effective_name);
        ClassInfo * existing_info = class_info_for_type(existing);
        if(existing_info && existing_info->complete) {
          child.value = effective_name;
          continue;
        }
      }

      CppAstNode named_child = child;
      named_child.value = effective_name;
      const CppAstNode * durable_named_child =
          own_synthetic_ast(std::move(named_child));
      if(child.kind == CppAstKind::enum_specifier) {
        collect_enum_declaration(scope, *durable_named_child);
      } else {
        collect_class_declaration(scope, *durable_named_child);
        if(synthesized_embedded_name) {
          if(ClassInfo * info = lookup_declared_class_info(scope, effective_name)) {
            info->source_is_unnamed_class = true;
            info->source_unnamed_class_node =
                info->class_node ? info->class_node : durable_named_child;
          }
        }
      }

      child.value = effective_name;
    }
    return true;
  }

  bool prepare_block_scope_declaration_specifiers(Scope & scope,
                                                  const CppAstNode & specifiers,
                                                  const CppAstNode * declarators,
                                                  PreparedDeclarationSpecifiers & out)
  {
    out = PreparedDeclarationSpecifiers();
    if(!prepare_block_scope_specifiers(scope,
                                       specifiers,
                                       declarators,
                                       out.resolved_specifiers)) {
      return false;
    }
    out.has_auto = decl_spec_contains_auto(out.resolved_specifiers);
    const bool declaration_is_typedef =
        decl_spec_contains_token(out.resolved_specifiers, KW_TYPEDEF);
    out.parsed_decl_spec =
        out.has_auto ||
        parse_decl_spec(out.resolved_specifiers,
                        scope,
                        out.declaration_is_typedef,
                        out.base,
                        declaration_is_typedef);
    return true;
  }

  TypePtr apply_auto_cv_qualifiers(const CppAstNode & specifiers,
                                   const TypePtr & deduced) const
  {
    return apply_cv(deduced,
                    decl_spec_contains_token(specifiers, KW_CONST),
                    decl_spec_contains_token(specifiers, KW_VOLATILE));
  }

  TypePtr apply_constexpr_object_qualifier(const CppAstNode & specifiers,
                                           const TypePtr & type,
                                           bool is_typedef) const
  {
    if(is_typedef ||
       !type ||
       !decl_spec_contains_token(specifiers, KW_CONSTEXPR)) {
      return type;
    }
    TypePtr stripped = strip_top_level_cv(type);
    if(!stripped || stripped->kind == Type::TK_FUNCTION) {
      return type;
    }
    return apply_cv(type, true, false);
  }

  bool deduce_auto_type_from_initializer(Scope & scope,
                                         const CppAstNode & initializer,
                                         TypePtr & out)
  {
    if(initializer.kind != CppAstKind::initializer || initializer.children.size() != 1) {
      return false;
    }

    const CppAstNode & payload = initializer.children[0];
    if(payload.kind == CppAstKind::braced_init_list) {
      if(payload.children.empty()) {
        return false;
      }
      if(payload.children.size() == 1) {
        ExprInfo expr = analyze_expression(scope, payload.children[0]);
        out = value_conversion_type(expr);
        return static_cast<bool>(out);
      }
      ExprInfo expr = analyze_braced_init_list_expression(scope, payload);
      out = expr.type;
      return static_cast<bool>(out);
    }

    ExprInfo expr = analyze_expression(scope, payload);
    out = value_conversion_type(expr);
    return static_cast<bool>(out);
  }

  bool analyze_auto_initializer_expression(Scope & scope,
                                           const CppAstNode & initializer,
                                           ExprInfo & out)
  {
    if(initializer.kind != CppAstKind::initializer || initializer.children.size() != 1) {
      return false;
    }

    const CppAstNode & payload = initializer.children[0];
    if(payload.kind == CppAstKind::braced_init_list) {
      if(payload.children.empty()) {
        return false;
      }
      if(payload.children.size() == 1) {
        out = analyze_expression(scope, payload.children[0]);
        return static_cast<bool>(out.type);
      }
      out = analyze_braced_init_list_expression(scope, payload);
      return static_cast<bool>(out.type);
    }

    out = analyze_expression(scope, payload);
    return static_cast<bool>(out.type);
  }

  bool deduce_auto_placeholder_from_type_pattern(const TypePtr & pattern,
                                                 const TypePtr & argument,
                                                 const string & placeholder_key,
                                                 TypePtr & out) const
  {
    if(!pattern || !argument) {
      return false;
    }

    TypePtr stripped_pattern = strip_top_level_cv(pattern);
    if(!stripped_pattern) {
      return false;
    }
    if(stripped_pattern->kind == Type::TK_NAMED &&
       stripped_pattern->named_key == placeholder_key) {
      out = argument;
      return true;
    }

    TypePtr stripped_argument = strip_top_level_cv(argument);
    if(!stripped_argument) {
      return false;
    }

    switch(stripped_pattern->kind) {
    case Type::TK_POINTER:
      return stripped_argument->kind == Type::TK_POINTER &&
             deduce_auto_placeholder_from_type_pattern(stripped_pattern->inner,
                                                       stripped_argument->inner,
                                                       placeholder_key,
                                                       out);

    case Type::TK_LVALUE_REFERENCE:
      return stripped_argument->kind == Type::TK_LVALUE_REFERENCE &&
             deduce_auto_placeholder_from_type_pattern(stripped_pattern->inner,
                                                       stripped_argument->inner,
                                                       placeholder_key,
                                                       out);

    case Type::TK_RVALUE_REFERENCE:
      return stripped_argument->kind == Type::TK_RVALUE_REFERENCE &&
             deduce_auto_placeholder_from_type_pattern(stripped_pattern->inner,
                                                       stripped_argument->inner,
                                                       placeholder_key,
                                                       out);

    case Type::TK_ARRAY:
      return stripped_argument->kind == Type::TK_ARRAY &&
             stripped_pattern->has_bound == stripped_argument->has_bound &&
             (!stripped_pattern->has_bound || stripped_pattern->bound == stripped_argument->bound) &&
             deduce_auto_placeholder_from_type_pattern(stripped_pattern->inner,
                                                       stripped_argument->inner,
                                                       placeholder_key,
                                                       out);

    case Type::TK_FUNCTION:
      if(stripped_argument->kind != Type::TK_FUNCTION ||
         stripped_pattern->params.size() != stripped_argument->params.size() ||
         stripped_pattern->variadic != stripped_argument->variadic ||
         stripped_pattern->prototype_relaxed != stripped_argument->prototype_relaxed) {
        return false;
      }
      if(!deduce_auto_placeholder_from_type_pattern(stripped_pattern->inner,
                                                    stripped_argument->inner,
                                                    placeholder_key,
                                                    out)) {
        return false;
      }
      for(size_t i = 0; i < stripped_pattern->params.size(); ++i) {
        TypePtr ignored;
        if(!deduce_auto_placeholder_from_type_pattern(stripped_pattern->params[i],
                                                      stripped_argument->params[i],
                                                      placeholder_key,
                                                      ignored)) {
          return false;
        }
      }
      return true;

    default:
      return false;
    }
  }

  bool deduce_auto_declarator_base_from_expr(Scope & scope,
                                             const CppAstNode & declarator,
                                             const ExprInfo & expr,
                                             string & name,
                                             TypePtr & out)
  {
    static const string auto_placeholder_key = "__cppgm_auto_placeholder";
    const TypePtr placeholder =
        make_named("auto", auto_placeholder_key, false);
    TypePtr pattern;
    if(!parse_declarator(scope, declarator, placeholder, name, pattern)) {
      return false;
    }

    TypePtr stripped_pattern = strip_top_level_cv(pattern);
    if(!stripped_pattern) {
      return false;
    }

    TypePtr argument_type;
    if(stripped_pattern->kind == Type::TK_LVALUE_REFERENCE) {
      if(expr.category != VC_LVALUE) {
        return false;
      }
      argument_type = remove_reference_type(expr.type);
      return deduce_auto_placeholder_from_type_pattern(stripped_pattern->inner,
                                                       argument_type,
                                                       auto_placeholder_key,
                                                       out);
    }

    if(stripped_pattern->kind == Type::TK_RVALUE_REFERENCE) {
      TypePtr base_type = remove_reference_type(expr.type);
      if(!base_type) {
        return false;
      }
      argument_type = expr.category == VC_LVALUE ?
          make_lvalue_reference_raw(base_type) :
          base_type;
      return deduce_auto_placeholder_from_type_pattern(stripped_pattern->inner,
                                                       argument_type,
                                                       auto_placeholder_key,
                                                       out);
    }

    argument_type = value_conversion_type(expr);
    return deduce_auto_placeholder_from_type_pattern(pattern,
                                                     argument_type,
                                                     auto_placeholder_key,
                                                     out);
  }

  bool parse_auto_declaration_type_from_expr(Scope & scope,
                                             const CppAstNode & specifiers,
                                             const CppAstNode & declarator,
                                             const ExprInfo & expr,
                                             string & name,
                                             TypePtr & out,
                                             bool reference_class_templates_only = false)
  {
    TypePtr deduced_base;
    if(!deduce_auto_declarator_base_from_expr(scope,
                                              declarator,
                                              expr,
                                              name,
                                              deduced_base)) {
      return false;
    }
    deduced_base = apply_auto_cv_qualifiers(specifiers, deduced_base);
    return parse_declarator(scope,
                            declarator,
                            deduced_base,
                            name,
                            out,
                            reference_class_templates_only);
  }

  enum AutoFunctionReturnPattern
  {
    AFR_VALUE,
    AFR_LVALUE_REFERENCE,
    AFR_RVALUE_REFERENCE
  };

  bool determine_auto_function_return_pattern(Scope & scope,
                                              const CppAstNode & declarator,
                                              AutoFunctionReturnPattern & out,
                                              bool reference_class_templates_only = false)
  {
    string ignored_name;
    TypePtr parsed_type;
    if(!parse_declarator(scope,
                         declarator,
                         make_fundamental(FT_INT),
                         ignored_name,
                         parsed_type,
                         reference_class_templates_only)) {
      return false;
    }

    TypePtr function_type = strip_top_level_cv(parsed_type);
    if(!function_type || function_type->kind != Type::TK_FUNCTION || !function_type->inner) {
      return false;
    }

    if(function_type->inner->kind == Type::TK_LVALUE_REFERENCE) {
      out = AFR_LVALUE_REFERENCE;
      return true;
    }
    if(function_type->inner->kind == Type::TK_RVALUE_REFERENCE) {
      out = AFR_RVALUE_REFERENCE;
      return true;
    }
    out = AFR_VALUE;
    return true;
  }

  bool deduce_auto_function_return_base_from_expr(AutoFunctionReturnPattern pattern,
                                                  const ExprInfo & expr,
                                                  TypePtr & out) const
  {
    switch(pattern) {
    case AFR_VALUE:
      out = value_conversion_type(expr);
      return static_cast<bool>(out);

    case AFR_LVALUE_REFERENCE:
      if(expr.category != VC_LVALUE || !expr.type) {
        return false;
      }
      out = expr.type;
      return true;

    case AFR_RVALUE_REFERENCE:
      if(!expr.type) {
        return false;
      }
      if(expr.category == VC_LVALUE) {
        out = make_lvalue_reference_raw(expr.type);
      } else {
        out = expr.type;
      }
      return true;
    }

    return false;
  }

  bool deduce_auto_function_definition_base(Scope & scope,
                                            const CppAstNode & specifiers,
                                            const CppAstNode & declarator,
                                            const CppAstNode & body,
                                            bool is_const_method,
                                            bool is_volatile_method,
                                            TypePtr & out,
                                            bool reference_class_templates_only = false)
  {
    AutoFunctionReturnPattern pattern = AFR_VALUE;
    if(!determine_auto_function_return_pattern(scope,
                                               declarator,
                                               pattern,
                                               reference_class_templates_only)) {
      return false;
    }

    vector<pair<string, TypePtr> > params;
    const CppAstNode * parameter_clause =
        find_child_kind(declarator, CppAstKind::parameter_clause);
    if(parameter_clause &&
       !parse_parameter_clause(scope,
                               *parameter_clause,
                               params,
                               nullptr,
                               reference_class_templates_only)) {
      return false;
    }

    Scope function_scope(&scope, "<auto-return>", false);
    function_scope.class_info = scope.class_info;

    FunctionBinding synthetic_function;
    synthetic_function.name = "<auto-return>";
    synthetic_function.owner_class = scope.class_info;
    if(scope.class_info) {
      vector<TypePtr> explicit_param_types;
      for(size_t i = 0; i < params.size(); ++i) {
        explicit_param_types.push_back(params[i].second);
      }
      TypePtr placeholder_declared =
          make_function(make_fundamental(FT_VOID), explicit_param_types, false);
      synthetic_function.type =
          method_function_type(scope.class_info->type,
                               is_const_method,
                               is_volatile_method,
                               placeholder_declared);
      if(!synthetic_function.type || synthetic_function.type->params.empty()) {
        return false;
      }
      function_scope.values["this"] =
          ValueBinding(ValueBinding::VK_PARAMETER, "this", synthetic_function.type->params[0]);
    }
    function_scope.function = &synthetic_function;

    for(size_t i = 0; i < params.size(); ++i) {
      if(params[i].first.empty()) {
        continue;
      }
      function_scope.values[params[i].first] =
          ValueBinding(ValueBinding::VK_PARAMETER, params[i].first, params[i].second);
    }

    vector<ExprInfo> return_exprs;
    bool saw_void_return = false;
    semantic_statement::collect_return_expressions(
        *this, function_scope, body, return_exprs, saw_void_return);

    if(return_exprs.empty()) {
      if(saw_void_return || pattern == AFR_VALUE) {
        out = apply_auto_cv_qualifiers(specifiers, make_fundamental(FT_VOID));
        return static_cast<bool>(out);
      }
      return false;
    }
    if(saw_void_return) {
      return false;
    }

    TypePtr deduced_base;
    for(size_t i = 0; i < return_exprs.size(); ++i) {
      TypePtr candidate;
      if(!deduce_auto_function_return_base_from_expr(pattern, return_exprs[i], candidate)) {
        return false;
      }
      if(!deduced_base) {
        deduced_base = candidate;
        continue;
      }
      if(!type_equals(deduced_base, candidate)) {
        return false;
      }
    }

    if(!deduced_base) {
      return false;
    }
    out = apply_auto_cv_qualifiers(specifiers, deduced_base);
    return static_cast<bool>(out);
  }

  bool parse_trailing_return_base(Scope & scope,
                                  const CppAstNode & specifiers,
                                  const CppAstNode & declarator,
                                  bool & is_typedef,
                                  TypePtr & out,
                                  bool reference_class_templates_only = false)
  {
    const CppAstNode * trailing =
        find_child_kind(declarator, CppAstKind::trailing_return_type);
    if(!trailing) {
      return parse_decl_spec(specifiers, scope, is_typedef, out,
                             reference_class_templates_only);
    }
    if(!decl_spec_contains_auto(specifiers)) {
      return false;
    }

    is_typedef = false;
    Scope return_scope(&scope, "<trailing-return>", false);
    const CppAstNode * parameter_clause =
        find_child_kind(declarator, CppAstKind::parameter_clause);
    if(parameter_clause) {
      vector<pair<string, TypePtr> > params;
      if(!parse_parameter_clause(scope, *parameter_clause, params, nullptr,
                                reference_class_templates_only)) {
        return false;
      }
      for(size_t i = 0; i < params.size(); ++i) {
        if(params[i].first.empty()) {
          continue;
        }
        return_scope.values[params[i].first] =
            ValueBinding(ValueBinding::VK_PARAMETER, params[i].first, params[i].second);
      }
    }

    const CppAstNode * trailing_type_id =
        find_child_kind(*trailing, CppAstKind::type_id);
    if(!trailing_type_id ||
       !parse_type_id(return_scope,
                      *trailing_type_id,
                      out,
                      reference_class_templates_only)) {
      return false;
    }
    out = apply_auto_cv_qualifiers(specifiers, out);
    return static_cast<bool>(out);
  }

  bool parse_function_definition_base(Scope & scope,
                                      const CppAstNode & specifiers,
                                      const CppAstNode & declarator,
                                      const CppAstNode & body,
                                      bool is_const_method,
                                      bool is_volatile_method,
                                      bool & is_typedef,
                                      TypePtr & out,
                                      bool reference_class_templates_only = false)
  {
    if(parse_trailing_return_base(scope,
                                  specifiers,
                                  declarator,
                                  is_typedef,
                                  out,
                                  reference_class_templates_only)) {
      return true;
    }

    if(!decl_spec_contains_auto(specifiers) ||
       find_child_kind(declarator, CppAstKind::trailing_return_type)) {
      return false;
    }

    is_typedef = false;
    const CppAstNode * effective_body = &body;
    if(body.kind == CppAstKind::lazy_function_body) {
      effective_body = materialize_lazy_function_body(body);
    }
    return deduce_auto_function_definition_base(scope,
                                                specifiers,
                                                declarator,
                                                *effective_body,
                                                is_const_method,
                                                is_volatile_method,
                                                out,
                                                reference_class_templates_only);
  }

  bool parse_variable_declaration_type(Scope & scope,
                                       const CppAstNode & specifiers,
                                       const CppAstNode & declarator,
                                       const CppAstNode * initializer,
                                       bool allow_auto,
                                       std::string & name,
                                       TypePtr & out,
                                       bool & is_typedef,
                                       bool reference_class_templates_only = false)
  {
    const std::string use_location =
        prefer_later_source_location(preferred_source_location_for_node(specifiers),
                                     prefer_later_source_location(
                                         preferred_source_location_for_node(declarator),
                                         initializer ?
                                             preferred_source_location_for_node(*initializer) :
                                             std::string()));
    ScopedTemplateUseLocation use_location_guard(use_location);
    is_typedef = false;
    const bool declaration_is_typedef =
        decl_spec_contains_token(specifiers, KW_TYPEDEF);
    const bool parse_type_by_reference =
        reference_class_templates_only || declaration_is_typedef;
    if(find_child_kind(declarator, CppAstKind::parameter_clause)) {
      TypePtr base;
      if(!parse_trailing_return_base(scope, specifiers, declarator, is_typedef, base,
                                     parse_type_by_reference)) {
        return false;
      }
      const CppAstNode filtered_declarator = filtered_function_declarator(declarator);
      if(!parse_declarator(scope, filtered_declarator, base, name, out,
                           parse_type_by_reference)) {
        return false;
      }
      out = apply_constexpr_object_qualifier(specifiers, out, is_typedef);
      return true;
    }

    if(allow_auto && decl_spec_contains_auto(specifiers)) {
      if(!initializer) {
        return false;
      }
      ExprInfo initializer_expr;
      if(!analyze_auto_initializer_expression(scope, *initializer, initializer_expr)) {
        return false;
      }
      TypePtr deduced_base;
      if(!deduce_auto_declarator_base_from_expr(scope,
                                                declarator,
                                                initializer_expr,
                                                name,
                                                deduced_base)) {
        return false;
      }
      deduced_base = apply_auto_cv_qualifiers(specifiers, deduced_base);
      if(!parse_declarator(scope,
                           declarator,
                           deduced_base,
                           name,
                           out,
                           reference_class_templates_only)) {
        return false;
      }
      out = apply_constexpr_object_qualifier(specifiers, out, is_typedef);
      out = apply_initializer_array_bound(out, initializer);
      return true;
    }

    TypePtr base;
    if(!parse_decl_spec(specifiers, scope, is_typedef, base,
                        parse_type_by_reference)) {
      return false;
    }
    if(!parse_declarator(scope, declarator, base, name, out,
                         parse_type_by_reference)) {
      return false;
    }
    if(!is_typedef && out) {
      const std::string type_start_location =
          prefer_earlier_source_location(source_location_for_node(specifiers),
                                         earliest_child_source_location_for_node(specifiers));
      std::string declarator_name_location =
          source_location_for_name_in_node(declarator, name);
      if(declarator_name_location.empty()) {
        declarator_name_location = preferred_source_location_for_node(declarator);
      }
      std::string alias_member_name;
      const std::string qualified_alias_start_location =
          source_location_for_last_qualified_member_start_before(
              type_start_location,
              declarator_name_location,
              alias_member_name);
      if(!qualified_alias_start_location.empty()) {
        template_api::ClassTemplateUseInfo use;
        const bool direct_class_template_spelling =
            !alias_member_name.empty() &&
            template_api::class_template_use_info_for_type(*this,
                                                           scope,
                                                           out,
                                                           use) &&
            alias_member_name == use.unqualified_name;
        if(!direct_class_template_spelling) {
          record_deduced_class_use_for_resolved_alias_type(
              scope,
              out,
              qualified_alias_start_location);
        }
      } else if(specifiers.children.size() == 1 &&
                specifiers.children[0].kind == CppAstKind::decl_specifier &&
                is_identifier_text(specifiers.children[0].value)) {
        const std::string specifier_location =
            source_location_for_name_in_node(specifiers,
                                             specifiers.children[0].value);
        if(!specifier_location.empty()) {
          record_deduced_class_use_for_resolved_alias_type(
              scope,
              out,
              specifier_location);
        }
      }
    }
    out = apply_constexpr_object_qualifier(specifiers, out, is_typedef);
    out = apply_initializer_array_bound(out, initializer);
    return true;
  }

  bool parse_constexpr_local_declaration(
      Scope & scope,
      const CppAstNode & node,
      vector<constant_eval::LocalDeclaration> & out,
      string & error)
  {
    out.clear();
    error.clear();

    const CppAstNode * specifiers = find_child_kind(node, CppAstKind::decl_specifier_seq);
    const CppAstNode * declarators = find_child_kind(node, CppAstKind::init_declarator_list);
    if(!specifiers) {
      error = "simple-declaration missing decl-specifier-seq";
      return false;
    }
    if(!declarators) {
      return true;
    }

    PreparedDeclarationSpecifiers prepared_specifiers;
    if(!prepare_block_scope_declaration_specifiers(scope,
                                                   *specifiers,
                                                   declarators,
                                                   prepared_specifiers)) {
      error = "unsupported local embedded class-specifier";
      return false;
    }

    for(size_t i = 0; i < declarators->children.size(); ++i) {
      const CppAstNode & init_decl = declarators->children[i];
      if(init_decl.kind != CppAstKind::init_declarator || init_decl.children.empty()) {
        error = "unsupported local init-declarator";
        return false;
      }

      const CppAstNode * initializer = init_decl.children.size() > 1 ? &init_decl.children[1] : nullptr;
      string name;
      TypePtr type;
      bool is_typedef = false;
      if(!parse_variable_declaration_type(scope,
                                          prepared_specifiers.resolved_specifiers,
                                          init_decl.children[0],
                                          initializer,
                                          true,
                                          name,
                                          type,
                                          is_typedef)) {
        error = "unsupported local declarator in constexpr evaluation";
        return false;
      }
      if(is_typedef) {
        if(!name.empty()) {
          scope.named_types[name] = type;
          template_api::bump_scope_template_binding_fingerprint_epoch(scope);
        }
        continue;
      }
      if(!type || strip_top_level_cv(type)->kind == Type::TK_FUNCTION) {
        error = "local function declaration unsupported in constexpr evaluation";
        return false;
      }

      constant_eval::LocalDeclaration decl;
      decl.name = name;
      decl.type = type;
      decl.initializer = initializer;
      out.push_back(decl);
    }

    return true;
  }

  bool variable_declaration_is_definition(const CppAstNode & specifiers,
                                          const CppAstNode * initializer,
                                          const TypePtr & type,
                                          bool is_c_linkage = false,
                                          bool linkage_has_braces = false) const
  {
    TypePtr base = strip_top_level_cv(type);
    if(base && base->kind == Type::TK_FUNCTION) {
      return false;
    }
    if(is_c_linkage && !linkage_has_braces && initializer == nullptr &&
       !decl_spec_contains_token(specifiers, KW_CONSTEXPR)) {
      return false;
    }
    return !decl_spec_contains_token(specifiers, KW_EXTERN) ||
           decl_spec_contains_token(specifiers, KW_CONSTEXPR) ||
           initializer != nullptr;
  }

  FunctionBinding * find_exact_function_binding(
      map<string, vector<FunctionBinding *> > & functions,
      const string & name,
      const TypePtr & type,
      const FunctionTemplateRegistrationIdentity & template_identity =
          FunctionTemplateRegistrationIdentity(),
      RefQualifier ref_qualifier = RQ_NONE)
  {
    return callsemantic::find_exact_function_binding(functions,
                                                     name,
                                                     type,
                                                     template_identity.decl,
                                                     template_identity.key,
                                                     ref_qualifier);
  }

  FunctionBinding * find_defined_function_binding(
      map<string, vector<FunctionBinding *> > & functions,
      const string & name,
      const TypePtr & type,
      const FunctionTemplateRegistrationIdentity & template_identity =
          FunctionTemplateRegistrationIdentity(),
      RefQualifier ref_qualifier = RQ_NONE)
  {
    return callsemantic::find_defined_function_binding(functions,
                                                       name,
                                                       type,
                                                       template_identity.decl,
                                                       template_identity.key,
                                                       ref_qualifier);
  }

  FunctionBinding * find_matching_function(Scope & scope,
                                           const string & name,
                                           const TypePtr & type,
                                           const FunctionTemplateRegistrationIdentity
                                               & template_identity =
                                               FunctionTemplateRegistrationIdentity())
  {
    return find_exact_function_binding(scope.function_sets,
                                       name,
                                       type,
                                       template_identity,
                                       RQ_NONE);
  }

  FunctionBinding * find_matching_class_function(ClassInfo & info,
                                                 const string & name,
                                                 const TypePtr & type,
                                                 const FunctionTemplateRegistrationIdentity
                                                     & template_identity =
                                                     FunctionTemplateRegistrationIdentity(),
                                                 RefQualifier ref_qualifier = RQ_NONE)
  {
    return find_exact_function_binding(info.methods,
                                       name,
                                       type,
                                       template_identity,
                                       ref_qualifier);
  }

  FunctionBinding * find_exact_function(Scope & scope,
                                        const string & name,
                                        const TypePtr & type)
  {
    return find_matching_function(scope, name, type);
  }

  FunctionBinding * find_function_by_symbol(const symbol_linkage::SymbolIdentity & symbol,
                                            const string & name,
                                            const TypePtr & type)
  {
    callsemantic::FunctionRegistryState state = function_registry_state();
    return callsemantic::find_function_by_symbol(
        state,
        function_registry_callbacks(),
        symbol,
        name,
        type);
  }

  void index_function_binding(FunctionBinding * binding)
  {
    callsemantic::FunctionRegistryState state = function_registry_state();
    callsemantic::index_function_binding(state, binding);
  }

  void erase_indexed_function_binding(FunctionBinding * binding)
  {
    callsemantic::FunctionRegistryState state = function_registry_state();
    callsemantic::erase_indexed_function_binding(state, binding);
  }

  void release_function_symbol_reservation(
      const FunctionBinding * binding,
      const FunctionBinding * retained_binding = nullptr)
  {
    callsemantic::FunctionRegistryState state = function_registry_state();
    callsemantic::release_function_symbol_reservation(state,
                                                      binding,
                                                      retained_binding);
  }

  void discard_function_binding(FunctionBinding * binding)
  {
    callsemantic::FunctionRegistryState state = function_registry_state();
    callsemantic::discard_function_binding(state, binding);
  }

  void discard_class_function_bindings(ClassInfo & info)
  {
    callsemantic::FunctionRegistryState state = function_registry_state();
    callsemantic::discard_class_function_bindings(state, info);
  }

  FunctionBinding * first_function_by_internal_symbol(const string & internal_symbol) const
  {
    callsemantic::FunctionRegistryState state =
        const_cast<Analyzer *>(this)->function_registry_state();
    return callsemantic::first_function_by_internal_symbol(state,
                                                           internal_symbol);
  }

  FunctionBinding * find_exact_class_function(ClassInfo & info,
                                              const string & name,
                                              const TypePtr & type,
                                              RefQualifier ref_qualifier = RQ_NONE)
  {
    return find_matching_class_function(
        info, name, type, FunctionTemplateRegistrationIdentity(), ref_qualifier);
  }

  FunctionBinding * find_defined_function(Scope & scope,
                                          const string & name,
                                          const TypePtr & type,
                                          const FunctionTemplateRegistrationIdentity &
                                              template_identity =
                                                  FunctionTemplateRegistrationIdentity())
  {
    return find_defined_function_binding(scope.function_sets,
                                         name,
                                         type,
                                         template_identity,
                                         RQ_NONE);
  }

  FunctionBinding * find_defined_class_function(ClassInfo & info,
                                                const string & name,
                                                const TypePtr & type,
                                                const FunctionTemplateRegistrationIdentity &
                                                    template_identity =
                                                        FunctionTemplateRegistrationIdentity(),
                                                RefQualifier ref_qualifier = RQ_NONE)
  {
    return find_defined_function_binding(info.methods,
                                         name,
                                         type,
                                         template_identity,
                                         ref_qualifier);
  }

  bool types_equivalent_for_member_binding(const TypePtr & lhs, const TypePtr & rhs) const
  {
    return callsemantic::types_equivalent_for_member_binding(lhs, rhs);
  }

  bool append_dependent_type_resolution_cache_key(std::string & out,
                                                  const TypePtr & type,
                                                  std::set<const Type *> & active) const
  {
    if(!type) {
      out += "<null>";
      return true;
    }

    const Type * raw = type.get();
    if(!active.insert(raw).second) {
      return false;
    }

    switch(type->kind) {
    case Type::TK_FUNDAMENTAL:
      out += "F:";
      out += to_string(static_cast<int>(type->fundamental));
      break;

    case Type::TK_NAMED:
      out += "N:";
      out += type->named_key;
      out += ":";
      out += type->named_display;
      out += ":";
      out += (type->named_complete ? "1" : "0");
      break;

    case Type::TK_CV:
      out += "CV:";
      out += (type->cv_const ? "1" : "0");
      out += (type->cv_volatile ? "1" : "0");
      out += ":";
      if(!append_dependent_type_resolution_cache_key(out, type->inner, active)) {
        active.erase(raw);
        return false;
      }
      break;

    case Type::TK_ATOMIC:
      out += "AT:";
      if(!append_dependent_type_resolution_cache_key(out, type->inner, active)) {
        active.erase(raw);
        return false;
      }
      break;

    case Type::TK_POINTER:
      out += "P:";
      if(!append_dependent_type_resolution_cache_key(out, type->inner, active)) {
        active.erase(raw);
        return false;
      }
      break;

    case Type::TK_MEMBER_POINTER:
      out += "MP:";
      if(!append_dependent_type_resolution_cache_key(out, type->owner, active)) {
        active.erase(raw);
        return false;
      }
      out += ":";
      if(!append_dependent_type_resolution_cache_key(out, type->inner, active)) {
        active.erase(raw);
        return false;
      }
      break;

    case Type::TK_BLOCK_POINTER:
      out += "BP:";
      if(!append_dependent_type_resolution_cache_key(out, type->inner, active)) {
        active.erase(raw);
        return false;
      }
      break;

    case Type::TK_LVALUE_REFERENCE:
      out += "LR:";
      if(!append_dependent_type_resolution_cache_key(out, type->inner, active)) {
        active.erase(raw);
        return false;
      }
      break;

    case Type::TK_RVALUE_REFERENCE:
      out += "RR:";
      if(!append_dependent_type_resolution_cache_key(out, type->inner, active)) {
        active.erase(raw);
        return false;
      }
      break;

    case Type::TK_ARRAY:
      out += "AR:";
      out += (type->has_bound ? "1" : "0");
      out += ":";
      out += to_string(type->bound);
      out += ":";
      out += type->bound_text;
      out += ":";
      if(!append_dependent_type_resolution_cache_key(out, type->inner, active)) {
        active.erase(raw);
        return false;
      }
      break;

    case Type::TK_FUNCTION:
      out += "FN:";
      if(!append_dependent_type_resolution_cache_key(out, type->inner, active)) {
        active.erase(raw);
        return false;
      }
      out += ":";
      out += (type->variadic ? "1" : "0");
      out += ":";
      out += (type->prototype_relaxed ? "1" : "0");
      out += (type->function_const ? "1" : "0");
      out += ":";
      out += (type->function_volatile ? "1" : "0");
      for(size_t i = 0; i < type->params.size(); ++i) {
        out += "|";
        if(!append_dependent_type_resolution_cache_key(out, type->params[i], active)) {
          active.erase(raw);
          return false;
        }
      }
      break;

    default:
      out += describe_type(type);
      break;
    }

    active.erase(raw);
    return true;
  }

  bool try_dependent_type_resolution_cache_key(const TypePtr & type, std::string & out) const
  {
    out.clear();
    std::set<const Type *> active;
    return append_dependent_type_resolution_cache_key(out, type, active);
  }

  string member_binding_type_identity(const TypePtr & type) const
  {
    if(!type) {
      return "<null>";
    }

    std::string key_text;
    ostringstream out;
    out << describe_type(type) << " {key=";
    if(try_dependent_type_resolution_cache_key(type, key_text)) {
      out << key_text;
    } else {
      out << "<cyclic>";
    }
    if(type->kind == Type::TK_NAMED) {
      out << " display=" << type->named_display;
    }
    out << "}";
    return out.str();
  }

  string dependent_type_resolution_cache_key(const TypePtr & type) const
  {
    std::string out;
    if(try_dependent_type_resolution_cache_key(type, out)) {
      return out;
    }
    return std::string();
  }

  string function_template_entity_key(const Scope & scope, const string & name) const
  {
    const string base_name = unqualified_member_name(canonical_function_lookup_name(name));
    const string scope_name = inline_namespace_collapsed_scope_name(&scope);
    return scope_name.empty() ? base_name : scope_name + "::" + base_name;
  }

  void append_unique_friend_template_access(FunctionTemplateDecl & decl,
                                            ClassInfo & info)
  {
    if(find(decl.friend_access_classes.begin(),
            decl.friend_access_classes.end(),
            &info) == decl.friend_access_classes.end()) {
      decl.friend_access_classes.push_back(&info);
    }
  }

  void append_unique_friend_function_template(ClassInfo & info,
                                              FunctionTemplateDecl & decl)
  {
    if(find(info.friend_function_templates.begin(),
            info.friend_function_templates.end(),
            &decl) == info.friend_function_templates.end()) {
      info.friend_function_templates.push_back(&decl);
    }
  }

  bool function_template_entities_match(
      const FunctionTemplateDecl & existing,
      Scope & candidate_entity_scope,
      Scope & candidate_template_scope,
      const string & candidate_name,
      const vector<TemplateParameterInfo> & candidate_parameters,
      const TypePtr & candidate_type,
      bool candidate_is_special_member_template,
      bool candidate_is_static_member,
      bool candidate_is_const_method,
      bool candidate_is_volatile_method,
      RefQualifier candidate_ref_qualifier) const override
  {
    auto function_parameter_lists_match =
        [&](const TypePtr & lhs,
            const vector<TemplateParameterInfo> & lhs_parameters,
            const TypePtr & rhs,
            const vector<TemplateParameterInfo> & rhs_parameters) -> bool
    {
      TypePtr lhs_function = strip_top_level_cv(lhs);
      TypePtr rhs_function = strip_top_level_cv(rhs);
      if(!lhs_function || !rhs_function ||
         lhs_function->kind != Type::TK_FUNCTION ||
         rhs_function->kind != Type::TK_FUNCTION) {
        return false;
      }
      if(lhs_function->variadic != rhs_function->variadic ||
         lhs_function->prototype_relaxed != rhs_function->prototype_relaxed ||
         lhs_function->function_const != rhs_function->function_const ||
         lhs_function->function_volatile != rhs_function->function_volatile ||
         lhs_function->params.size() != rhs_function->params.size()) {
        return false;
      }
      for(size_t i = 0; i < lhs_function->params.size(); ++i) {
        if(!out_of_class_special_member_template_param_types_match(
               lhs_function->params[i],
               lhs_parameters,
               rhs_function->params[i],
               rhs_parameters)) {
          return false;
        }
      }
      return true;
    };

    if(!existing.declaring_scope || !existing.type_pattern) {
      return false;
    }
    const string existing_key =
        function_template_entity_key(*existing.declaring_scope, existing.name);
    const string candidate_key =
        function_template_entity_key(candidate_entity_scope, candidate_name);
    if(existing_key != candidate_key) {
      return false;
    }
    Scope & existing_template_scope =
        existing.pattern_scope ? *existing.pattern_scope : candidate_template_scope;
    const bool parameters_match =
        out_of_class_special_member_template_parameters_match(existing_template_scope,
                                                              existing.parameters,
                                                              candidate_template_scope,
                                                              candidate_parameters);
    if(!parameters_match) {
      return false;
    }
    if(existing.declaring_scope->class_info &&
       (existing.is_static_member != candidate_is_static_member ||
        existing.is_const_method != candidate_is_const_method ||
        existing.is_volatile_method != candidate_is_volatile_method ||
        existing.ref_qualifier != candidate_ref_qualifier)) {
      return false;
    }
    const bool allow_special_member_match =
        candidate_is_special_member_template ||
        existing.is_constructor ||
        existing.is_destructor;
    if(!allow_special_member_match) {
      // Function template redeclarations are identified by the template head and
      // function parameter list. Dependent return-type alias spellings such as
      // `_Require<...>` vs `enable_if<...>::type` can differ while still naming
      // the same entity.
      return function_parameter_lists_match(existing.type_pattern,
                                            existing.parameters,
                                            candidate_type,
                                            candidate_parameters);
    }
    if(out_of_class_special_member_template_param_types_match(existing.type_pattern,
                                                              existing.parameters,
                                                              candidate_type,
                                                              candidate_parameters)) {
      return true;
    }
    if(!existing.declaring_scope->class_info) {
      return false;
    }
    return function_parameter_lists_match(existing.type_pattern,
                                          existing.parameters,
                                          candidate_type,
                                          candidate_parameters);
  }

  void inherit_pending_friend_function_template_access(
      FunctionTemplateDecl & decl) override
  {
    if(!decl.declaring_scope || !decl.type_pattern) {
      return;
    }
    map<string, vector<FunctionTemplateDecl *> >::const_iterator found =
        pending_friend_function_templates.find(
            function_template_entity_key(*decl.declaring_scope, decl.name));
    if(found == pending_friend_function_templates.end()) {
      return;
    }

    Scope & decl_template_scope = decl.pattern_scope ? *decl.pattern_scope : *decl.declaring_scope;
    for(size_t i = 0; i < found->second.size(); ++i) {
      FunctionTemplateDecl * pending = found->second[i];
      if(!pending || pending == &decl ||
         !function_template_entities_match(*pending,
                                           *decl.declaring_scope,
                                           decl_template_scope,
                                           decl.name,
                                           decl.parameters,
                                           decl.type_pattern,
                                           false,
                                           decl.is_static_member,
                                           decl.is_const_method,
                                           decl.is_volatile_method,
                                           decl.ref_qualifier)) {
        continue;
      }
      for(size_t j = 0; j < pending->friend_access_classes.size(); ++j) {
        if(pending->friend_access_classes[j]) {
          append_unique_friend_template_access(decl,
                                               *pending->friend_access_classes[j]);
        }
      }
    }
  }

  void attach_friend_function_template_access_to_existing(
      Scope & entity_scope,
      Scope & template_scope,
      const string & name,
      const vector<TemplateParameterInfo> & template_parameters,
      const TypePtr & type_pattern,
      bool is_static_member,
      bool is_const_method,
      bool is_volatile_method,
      RefQualifier ref_qualifier,
      ClassInfo & info)
  {
    vector<FunctionTemplateDecl *> existing_templates =
        lookup_direct_function_templates(entity_scope, name);
    for(size_t i = 0; i < existing_templates.size(); ++i) {
      FunctionTemplateDecl * candidate = existing_templates[i];
      if(candidate &&
         function_template_entities_match(*candidate,
                                          entity_scope,
                                          template_scope,
                                          name,
                                          template_parameters,
                                          type_pattern,
                                          false,
                                          is_static_member,
                                          is_const_method,
                                          is_volatile_method,
                                          ref_qualifier)) {
        append_unique_friend_template_access(*candidate, info);
        append_unique_friend_function_template(info, *candidate);
      }
    }
  }

  void record_friend_function_template_declaration(
      ClassInfo & info,
      Scope & entity_scope,
      Scope & template_scope,
      const string & name,
      const vector<TemplateParameterInfo> & template_parameters,
      const TypePtr & type_pattern,
      const vector<pair<string, TypePtr> > & params_pattern,
      const CppAstNode & result_type_pattern,
      const vector<const CppAstNode *> & default_arguments_pattern,
      const vector<const CppAstNode *> & parameter_declarations_pattern,
      bool is_static_member,
      bool is_const_method,
      bool is_volatile_method,
      RefQualifier ref_qualifier,
      const CppAstNode * specifiers,
      const CppAstNode * declarator,
      const CppAstNode * body,
      const CppAstNode * declaration_node) override
  {
    const vector<const CppAstNode *> normalized_default_arguments_pattern =
        normalize_default_arguments(params_pattern, default_arguments_pattern);
    const string key = function_template_entity_key(entity_scope, name);
    vector<FunctionTemplateDecl *> & slot = pending_friend_function_templates[key];
    for(size_t i = 0; i < slot.size(); ++i) {
      FunctionTemplateDecl * existing = slot[i];
      if(existing &&
         function_template_entities_match(*existing,
                                          entity_scope,
                                          template_scope,
                                          name,
                                          template_parameters,
                                          type_pattern,
                                          false,
                                          is_static_member,
                                          is_const_method,
                                          is_volatile_method,
                                          ref_qualifier)) {
        append_unique_friend_template_access(*existing, info);
        append_unique_friend_function_template(info, *existing);
        if(existing->result_type_pattern.kind == CppAstKind::invalid &&
           result_type_pattern.kind != CppAstKind::invalid) {
          existing->result_type_pattern = result_type_pattern;
        }
        attach_friend_function_template_access_to_existing(entity_scope,
                                                           template_scope,
                                                           name,
                                                           template_parameters,
                                                           type_pattern,
                                                           is_static_member,
                                                           is_const_method,
                                                           is_volatile_method,
                                                           ref_qualifier,
                                                           info);
        return;
      }
    }

    unique_ptr<FunctionTemplateDecl> decl(new FunctionTemplateDecl());
    decl->declaring_scope = &entity_scope;
    decl->pattern_scope = &template_scope;
    decl->name = name;
    decl->declaration_node = declaration_node;
    decl->definition_node = body ? declaration_node : nullptr;
    decl->body = body;
    decl->specifiers = specifiers;
    decl->definition_specifiers = body ? specifiers : nullptr;
    decl->declarator = declarator;
    decl->definition_declarator = body ? declarator : nullptr;
    decl->inner = declaration_node;
    decl->definition_inner = body ? declaration_node : nullptr;
    decl->parameters = template_parameters;
    decl->type_pattern = type_pattern;
    decl->params_pattern = params_pattern;
    decl->result_type_pattern = result_type_pattern;
    decl->parameter_declarations_pattern = parameter_declarations_pattern;
    initialize_function_template_parameter_aliases(*decl);
    decl->default_arguments_pattern = normalized_default_arguments_pattern;
    decl->is_constexpr = specifiers && decl_spec_contains_token(*specifiers, KW_CONSTEXPR);
    append_unique_friend_template_access(*decl, info);
    append_unique_friend_function_template(info, *decl);
    snapshot_function_template_debug_info(*this, *decl);
    slot.push_back(decl.get());
    function_templates.push_back(std::move(decl));

    attach_friend_function_template_access_to_existing(entity_scope,
                                                       template_scope,
                                                       name,
                                                       template_parameters,
                                                       type_pattern,
                                                       is_static_member,
                                                       is_const_method,
                                                       is_volatile_method,
                                                       ref_qualifier,
                                                       info);
  }

  FunctionBinding * find_equivalent_class_function(ClassInfo & info,
                                                   const string & name,
                                                   const TypePtr & type,
                                                   RefQualifier ref_qualifier = RQ_NONE)
  {
    FunctionBinding * exact =
        find_exact_class_function(info, name, type, ref_qualifier);
    if(exact && exact->has_definition) {
      return exact;
    }

    TypePtr resolved_target = type;
    TypePtr normalized_target;
    if(info.member_scope &&
       resolve_instantiated_dependent_type(*info.member_scope, type, normalized_target) &&
       normalized_target) {
      resolved_target = normalized_target;
    }
    const bool trace_binding = parser_trace::enabled("class.collect");
    if(trace_binding) {
      std::ostringstream trace;
      trace << "member-binding-search class=" << info.qualified_name
            << " name=" << name
            << " target.raw=" << member_binding_type_identity(type)
            << " target.resolved=" << member_binding_type_identity(resolved_target);
      parser_trace::note("class.collect", std::string(), trace.str());
    }

    typename map<string, vector<FunctionBinding *> >::iterator found =
        info.methods.find(name);
    if(found == info.methods.end()) {
      if(trace_binding) {
        std::ostringstream trace;
        trace << "member-binding-no-set class=" << info.qualified_name
              << " name=" << name;
        parser_trace::note("class.collect", std::string(), trace.str());
      }
      return nullptr;
    }

    FunctionBinding * normalized_match = nullptr;
    FunctionBinding * fallback_match = exact;
    for(size_t i = 0; i < found->second.size(); ++i) {
      FunctionBinding * candidate = found->second[i];
      if(candidate->ref_qualifier != ref_qualifier) {
        continue;
      }

      TypePtr resolved_candidate = candidate->type;
      TypePtr normalized_candidate;
      if(info.member_scope &&
         resolve_instantiated_dependent_type(*info.member_scope,
                                             candidate->type,
                                             normalized_candidate) &&
         normalized_candidate) {
        resolved_candidate = normalized_candidate;
      }

      const bool equivalent =
          types_equivalent_for_member_binding(resolved_candidate, resolved_target);
      if(trace_binding) {
        std::ostringstream trace;
        trace << "member-binding-candidate class=" << info.qualified_name
              << " name=" << name
              << " binding=" << static_cast<void *>(candidate)
              << " candidate.raw=" << member_binding_type_identity(candidate->type)
              << " candidate.resolved=" << member_binding_type_identity(resolved_candidate)
              << " candidate.decl="
              << (candidate->declaration_node ?
                      source_location_for_node(*candidate->declaration_node) :
                      string("<none>"))
              << " candidate.def="
              << (candidate->definition_node ?
                      source_location_for_node(*candidate->definition_node) :
                      string("<none>"))
              << " candidate.source-template="
              << function_binding_source_template_primary_location(*this, candidate)
              << " candidate.has-definition="
              << (candidate->has_definition ? "yes" : "no")
              << " equivalent=" << (equivalent ? "yes" : "no");
        parser_trace::note("class.collect", std::string(), trace.str());
      }
      if(!equivalent) {
        continue;
      }
      if(candidate->has_definition) {
        if(normalized_match && normalized_match != candidate) {
          if(trace_binding) {
            std::ostringstream trace;
            trace << "member-binding-ambiguous class=" << info.qualified_name
                  << " name=" << name;
            parser_trace::note("class.collect", std::string(), trace.str());
          }
          return nullptr;
        }
        normalized_match = candidate;
        continue;
      }
      if(fallback_match && fallback_match != candidate) {
        if(trace_binding) {
          std::ostringstream trace;
          trace << "member-binding-ambiguous class=" << info.qualified_name
                << " name=" << name;
          parser_trace::note("class.collect", std::string(), trace.str());
        }
        if(!exact) {
          return nullptr;
        }
        continue;
      }
      fallback_match = candidate;
    }
    if(trace_binding) {
      std::ostringstream trace;
      trace << "member-binding-result class=" << info.qualified_name
            << " name=" << name
            << " matched="
            << ((normalized_match || fallback_match) ? "yes" : "no");
      parser_trace::note("class.collect", std::string(), trace.str());
    }
    return normalized_match ? normalized_match : fallback_match;
  }

  string describe_exported_symbol_owner(const string & kind,
                                        const string & qualified_name,
                                        const TypePtr & type) const
  {
    ostringstream out;
    out << kind << " " << qualified_name;
    if(type) {
      out << " : " << describe_type(type);
    }
    return out.str();
  }

  void claim_exported_symbol(const symbol_linkage::SymbolIdentity & symbol,
                             const string & owner_description)
  {
    if(!symbol_linkage::has_exported_object_symbol(symbol) ||
       symbol.linkage == symbol_linkage::SL_WEAK ||
       symbol.object_symbol.compare(0, 2, "_Z") == 0) {
      return;
    }
    map<string, string>::const_iterator found =
        exported_symbol_owners.find(symbol.object_symbol);
    if(found != exported_symbol_owners.end()) {
      map<string, string>::const_iterator internal =
          exported_symbol_internals.find(symbol.object_symbol);
      if(internal != exported_symbol_internals.end() &&
         internal->second == symbol.internal_symbol) {
        return;
      }
      if(found->second == owner_description) {
        return;
      }
      throw logic_error("duplicate exported symbol " + symbol.object_symbol +
                        " [first " + found->second + "] [second " + owner_description + "]");
    }
    exported_symbol_owners[symbol.object_symbol] = owner_description;
    exported_symbol_internals[symbol.object_symbol] = symbol.internal_symbol;
  }

  string allocate_internal_symbol(const string & base_symbol)
  {
    string symbol = base_symbol;
    size_t overload_index = 1;
    while(used_internal_symbols.count(symbol) != 0) {
      ostringstream suffix;
      suffix << base_symbol << "__ov" << (++overload_index);
      symbol = suffix.str();
    }
    used_internal_symbols.insert(symbol);
    return symbol;
  }

  FunctionBinding * find_existing_c_linkage_function_symbol(const string & object_symbol,
                                                            const TypePtr & type) const
  {
    for(size_t i = 0; i < functions.size(); ++i) {
      const FunctionBinding & binding = *functions[i];
      if(!binding.is_c_linkage ||
         binding.symbol.object_symbol != object_symbol ||
         !type_equals(binding.type, type)) {
        continue;
      }
      return functions[i].get();
    }
    return nullptr;
  }

  bool declaration_specifiers_contain_token(const CppAstNode * declaration_node,
                                            ETokenType token) const
  {
    if(!declaration_node) {
      return false;
    }
    const CppAstNode * specifiers =
        declaration_node->kind == CppAstKind::decl_specifier_seq ||
                declaration_node->kind == CppAstKind::member_specifiers ?
            declaration_node :
            find_child_kind(*declaration_node, CppAstKind::decl_specifier_seq);
    if(!specifiers) {
      specifiers = find_child_kind(*declaration_node, CppAstKind::member_specifiers);
    }
    return specifiers && decl_spec_contains_token(*specifiers, token);
  }

  bool declaration_marks_inline(const CppAstNode * declaration_node) const
  {
    return declaration_specifiers_contain_token(declaration_node, KW_INLINE) ||
           declaration_specifiers_contain_token(declaration_node, KW_CONSTEXPR);
  }

  bool declaration_marks_inline_specifier(const CppAstNode * declaration_node) const
  {
    return declaration_specifiers_contain_token(declaration_node, KW_INLINE);
  }

  bool declaration_marks_static(const CppAstNode * declaration_node) const
  {
    return declaration_specifiers_contain_token(declaration_node, KW_STATIC);
  }

  bool declaration_marks_extern(const CppAstNode * declaration_node) const
  {
    return declaration_specifiers_contain_token(declaration_node, KW_EXTERN);
  }

  bool declaration_marks_friend(const CppAstNode * declaration_node) const
  {
    return declaration_specifiers_contain_token(declaration_node, KW_FRIEND);
  }

  bool declaration_marks_exclude_from_explicit_instantiation(
      const CppAstNode * declaration_node) const override
  {
    if(!declaration_node) {
      return false;
    }
    if(declaration_node->has_exclude_from_explicit_instantiation) {
      return true;
    }
    const CppAstNode * specifiers =
        declaration_node->kind == CppAstKind::decl_specifier_seq ||
                declaration_node->kind == CppAstKind::member_specifiers ?
            declaration_node :
            find_child_kind(*declaration_node, CppAstKind::decl_specifier_seq);
    if(specifiers && specifiers->has_exclude_from_explicit_instantiation) {
      return true;
    }
    const CppAstNode * member_specifiers =
        declaration_node->kind == CppAstKind::member_specifiers ?
            declaration_node :
            find_child_kind(*declaration_node, CppAstKind::member_specifiers);
    if(member_specifiers &&
       member_specifiers->has_exclude_from_explicit_instantiation) {
      return true;
    }
    return node_text(*declaration_node).find("exclude_from_explicit_instantiation") !=
           std::string::npos;
  }

  void invalidate_out_of_class_definition_caches(ClassTemplateDecl & decl)
  {
    callsemantic::invalidate_out_of_class_definition_caches(decl);
  }

  PartialClassTemplateSpecializationDecl * find_partial_specialization_decl(
      ClassTemplateDecl & decl,
      const ClassInfo * owner)
  {
    return callsemantic::find_partial_specialization_decl(decl, owner);
  }

  bool class_scope_defines_inline_like_function(Scope & scope,
                                                const CppAstNode * declaration_node,
                                                const CppAstNode * body,
                                                bool is_defaulted,
                                                bool in_class_member_definition_context) const
  {
    return scope.class_info &&
           in_class_member_definition_context &&
           (body != nullptr || is_defaulted) &&
           !declaration_marks_friend(declaration_node);
  }

  bool class_scope_has_template_identity(Scope & scope) const
  {
    return template_api::scope_has_template_owner_identity(&scope);
  }

  bool owner_class_has_template_identity(const ClassInfo * owner_class) const
  {
    return template_api::class_owner_scope_has_template_identity(owner_class);
  }

  bool scope_has_internal_namespace_linkage(Scope & scope) const
  {
    for(Scope * current = &scope; current; current = current->parent) {
      if(current->namespace_scope && current->name == "<unnamed>") {
        return true;
      }
    }
    return false;
  }

  bool friend_scope_defines_inline_like_function(const ClassInfo * lexical_access_class,
                                                 const CppAstNode * declaration_node,
                                                 const CppAstNode * body,
                                                 bool is_defaulted) const
  {
    return lexical_access_class &&
           declaration_marks_friend(declaration_node) &&
           (body != nullptr || is_defaulted);
  }

  FunctionTemplateRegistrationIdentity function_template_registration_identity(
      const FunctionBinding & binding) const
  {
    return template_api::function_binding_registration_identity(binding);
  }

  string function_object_symbol_key(const string & qualified_name,
                                    const TypePtr & type,
                                    bool is_const_method,
                                    bool is_volatile_method,
                                    RefQualifier ref_qualifier,
                                    bool is_constructor,
                                    bool is_destructor,
                                    const FunctionTemplateRegistrationIdentity & template_identity,
                                    const ClassInfo * owner_class) const
  {
    ostringstream out;
    out << qualified_name;
    out << "|type=" << describe_type(type);
    if(is_const_method) {
      out << "|const";
    }
    if(is_volatile_method) {
      out << "|volatile";
    }
    if(ref_qualifier != RQ_NONE) {
      out << "|refq=" << static_cast<int>(ref_qualifier);
    }
    if(is_constructor) {
      out << "|ctor";
    }
    if(is_destructor) {
      out << "|dtor";
    }
    if(template_identity.decl) {
      out << "|template=" << template_identity.decl->name;
    }
    if(owner_class) {
      const string & owner_instantiation =
          template_api::class_template_instantiation_key(*owner_class);
      if(!owner_instantiation.empty()) {
        out << "|owner-inst=" << owner_instantiation;
      }
    }
    if(!template_identity.key.empty()) {
      out << "|inst=" << template_identity.key;
    }
    return out.str();
  }

  symbol_linkage::FunctionRefQualifier symbol_linkage_ref_qualifier(
      RefQualifier ref_qualifier) const
  {
    switch(ref_qualifier) {
    case RQ_NONE:
      return symbol_linkage::FRQ_NONE;
    case RQ_LVALUE:
      return symbol_linkage::FRQ_LVALUE;
    case RQ_RVALUE:
      return symbol_linkage::FRQ_RVALUE;
    }
    return symbol_linkage::FRQ_NONE;
  }

  symbol_linkage::SymbolLinkage function_symbol_linkage(Scope & scope,
                                                        const CppAstNode * declaration_node,
                                                        const CppAstNode * body,
                                                        bool is_c_linkage,
                                                        const CppAstNode * function_qualifier,
                                                        const FunctionTemplateRegistrationIdentity
                                                            & template_identity =
                                                            FunctionTemplateRegistrationIdentity(),
                                                        bool is_defaulted = false,
                                                        const ClassInfo * lexical_access_class =
                                                            nullptr,
                                                        bool in_class_member_definition_context =
                                                            false)
      const override
  {
    if(scope_has_internal_namespace_linkage(scope) ||
       (!scope.class_info && declaration_marks_static(declaration_node))) {
      return symbol_linkage::SL_INTERNAL;
    }
    if((template_identity.decl &&
        (template_identity.has_arguments() || !template_identity.key.empty())) ||
       class_scope_has_template_identity(scope) ||
       class_scope_defines_inline_like_function(scope,
                                                declaration_node,
                                                body,
                                                is_defaulted,
                                                in_class_member_definition_context) ||
       friend_scope_defines_inline_like_function(lexical_access_class,
                                                declaration_node,
                                                body,
                                                is_defaulted) ||
       declaration_marks_inline(declaration_node)) {
      return symbol_linkage::SL_WEAK;
    }
    if(is_c_linkage) {
      return symbol_linkage::SL_EXTERNAL;
    }
    return symbol_linkage::SL_EXTERNAL;
  }

  symbol_linkage::SymbolLinkage variable_symbol_linkage(Scope & scope,
                                                        const CppAstNode * declaration_node,
                                                        bool is_c_linkage,
                                                        const ClassInfo * owner_class = nullptr) const
  {
    if(is_c_linkage) {
      return symbol_linkage::SL_EXTERNAL;
    }
    if(scope_has_internal_namespace_linkage(scope)) {
      return symbol_linkage::SL_INTERNAL;
    }
    if(class_scope_has_template_identity(scope) ||
       owner_class_has_template_identity(owner_class)) {
      return symbol_linkage::SL_WEAK;
    }
    if(owner_class && declaration_marks_inline(declaration_node)) {
      return symbol_linkage::SL_WEAK;
    }
    if(!owner_class && declaration_marks_static(declaration_node)) {
      return symbol_linkage::SL_INTERNAL;
    }
    if(!owner_class && declaration_marks_inline_specifier(declaration_node)) {
      return symbol_linkage::SL_WEAK;
    }
    if(!owner_class &&
       !declaration_marks_extern(declaration_node) &&
       (declaration_specifiers_contain_token(declaration_node, KW_CONST) ||
        declaration_specifiers_contain_token(declaration_node, KW_CONSTEXPR))) {
      return symbol_linkage::SL_INTERNAL;
    }
    return symbol_linkage::SL_EXTERNAL;
  }

  void populate_function_symbol_options(
      symbol_linkage::FunctionSymbolOptions & options,
      bool is_const_method,
      bool is_volatile_method,
      RefQualifier ref_qualifier,
      bool is_constructor,
      bool is_destructor,
      const FunctionTemplateRegistrationIdentity & template_identity =
          FunctionTemplateRegistrationIdentity(),
      const ClassInfo * owner_class = nullptr,
      const Scope * declaration_scope = nullptr,
      const CppAstNode * declaration_node = nullptr,
      const CppAstNode * definition_node = nullptr)
  {
    options.is_member_function = owner_class != nullptr;
    options.is_const_method = is_const_method;
    options.is_volatile_method = is_volatile_method;
    options.ref_qualifier = symbol_linkage_ref_qualifier(ref_qualifier);
    append_effective_function_abi_tags(options.abi_tags,
                                       declaration_node,
                                       definition_node);
    options.is_constructor = is_constructor;
    options.is_destructor = is_destructor;
    options.lookup_scope = declaration_scope;
    template_api::apply_function_template_symbol_options(template_identity.decl,
                                                        template_identity.arguments,
                                                        template_identity.has_arguments(),
                                                        owner_class,
                                                        is_constructor,
                                                        is_destructor,
                                                        options);
  }

  bool function_symbol_qualified_name_syntax(const string & simple_name,
                                             const ClassInfo * owner_class,
                                             const Scope * declaration_scope,
                                             QualifiedName & out) const
  {
    if(simple_name.empty()) {
      return false;
    }
    if(owner_class &&
       !owner_class->symbol_qualified_name_syntax.name.empty()) {
      out = owner_class->symbol_qualified_name_syntax;
      out.qualifiers.push_back(out.name);
      out.name = simple_name;
      return true;
    }
    if(owner_class) {
      return false;
    }
    if(declaration_scope && simple_name.find("::") == string::npos) {
      out = semantic_lookup::scope_symbol_qualified_name_syntax(*declaration_scope,
                                                                simple_name);
      return true;
    }
    return false;
  }

  void upgrade_function_symbol_linkage(FunctionBinding & binding,
                                       const string & qualified_name,
                                       const string & name,
                                       const TypePtr & type,
                                       symbol_linkage::SymbolLinkage linkage) override
  {
    const bool odr_mergeable_definition =
        binding.odr_mergeable_definition ||
        linkage == symbol_linkage::SL_WEAK ||
        template_api::function_or_owner_has_template_identity(&binding);
    if(odr_mergeable_definition && linkage == symbol_linkage::SL_EXTERNAL) {
      linkage = symbol_linkage::SL_WEAK;
    }
    symbol_linkage::FunctionSymbolOptions options;
    populate_function_symbol_options(options,
                                     binding.is_const_method,
                                     binding.is_volatile_method,
                                     binding.ref_qualifier,
                                     binding.is_constructor,
                                     binding.is_destructor,
                                     function_template_registration_identity(binding),
                                     binding.owner_class,
                                     binding.declaration_scope,
                                     binding.declaration_node,
                                     binding.definition_node);
    if(!binding.instantiation_pack_sizes.empty()) {
      options.template_argument_pack_sizes = &binding.instantiation_pack_sizes;
    }
    options.abi_tags = function_binding_abi_tags(binding);
    QualifiedName qualified_name_syntax;
    const bool has_qualified_name_syntax =
        function_binding_qualified_name_syntax_for_symbol(binding,
                                                          qualified_name_syntax);
    if(!has_qualified_name_syntax && !binding.is_c_linkage) {
      throw logic_error("missing qualified-name syntax for function symbol " +
                        qualified_name);
    }
    const string object_symbol_key =
        function_object_symbol_key(qualified_name,
                                   type,
                                   binding.is_const_method,
                                   binding.is_volatile_method,
                                   binding.ref_qualifier,
                                   binding.is_constructor,
                                   binding.is_destructor,
                                   function_template_registration_identity(binding),
                                   binding.owner_class);
    symbol_linkage::SymbolIdentity updated =
        has_qualified_name_syntax ?
            symbol_linkage::make_function_symbol_identity(qualified_name_syntax,
                                                          name,
                                                          binding.is_c_linkage,
                                                          type,
                                                          options,
                                                          object_symbol_key,
                                                          linkage) :
            symbol_linkage::make_c_function_symbol_identity(name, linkage);
    updated.internal_symbol = binding.symbol.internal_symbol;
    updated.keep_internal_alias = binding.symbol.keep_internal_alias;
    updated.prefer_local_object_binding =
        binding.symbol.prefer_local_object_binding ||
        function_prefers_local_object_binding(binding.definition_node ?
                                                  binding.definition_node :
                                                  binding.declaration_node,
                                              binding.body,
                                              binding.is_defaulted,
                                              binding.owner_class,
                                              binding.is_constructor,
                                              binding.is_destructor,
                                              binding.is_copy_assignment,
                                              binding.is_move_assignment);
    if(binding.symbol.internal_symbol == updated.internal_symbol &&
       binding.symbol.object_symbol == updated.object_symbol &&
       binding.symbol.keep_internal_alias == updated.keep_internal_alias &&
       binding.symbol.prefer_local_object_binding ==
           updated.prefer_local_object_binding &&
       binding.symbol.linkage == updated.linkage &&
       binding.odr_mergeable_definition == odr_mergeable_definition) {
      return;
    }
    binding.symbol = updated;
    binding.odr_mergeable_definition = odr_mergeable_definition;
    claim_exported_symbol(binding.symbol,
                          describe_exported_symbol_owner("function", qualified_name, type));
  }

  symbol_linkage::SymbolIdentity make_registered_function_symbol(const string & qualified_name,
                                                                 const string & name,
                                                                 const TypePtr & type,
                                                                 bool is_c_linkage,
                                                                 symbol_linkage::SymbolLinkage linkage,
                                                                 bool is_const_method = false,
                                                                 bool is_volatile_method = false,
                                                                 RefQualifier ref_qualifier = RQ_NONE,
                                                                 bool is_constructor = false,
                                                                 bool is_destructor = false,
                                                                 const FunctionTemplateRegistrationIdentity
                                                                     & template_identity =
                                                                     FunctionTemplateRegistrationIdentity(),
                                                                 const ClassInfo * owner_class = nullptr,
                                                                 bool prefer_overload_suffix = false,
                                                                 const CppAstNode * declaration_node = nullptr,
                                                                 const CppAstNode * definition_node = nullptr,
                                                                 const Scope * declaration_scope = nullptr,
                                                                 bool is_copy_assignment = false,
                                                                 bool is_move_assignment = false,
                                                                 const QualifiedName * qualified_name_syntax = nullptr)
  {
    symbol_linkage::FunctionSymbolOptions options;
    populate_function_symbol_options(options,
                                     is_const_method,
                                     is_volatile_method,
                                     ref_qualifier,
                                     is_constructor,
                                     is_destructor,
                                     template_identity,
                                     owner_class,
                                     declaration_scope,
                                     declaration_node,
                                     definition_node);
    const string object_symbol_key =
        function_object_symbol_key(qualified_name,
                                   type,
                                   is_const_method,
                                   is_volatile_method,
                                   ref_qualifier,
                                   is_constructor,
                                   is_destructor,
                                   template_identity,
                                   owner_class);
    QualifiedName generated_qualified_name_syntax;
    const QualifiedName * selected_qualified_name_syntax = qualified_name_syntax;
    if(!selected_qualified_name_syntax &&
       function_symbol_qualified_name_syntax(name,
                                             owner_class,
                                             declaration_scope,
                                             generated_qualified_name_syntax)) {
      selected_qualified_name_syntax = &generated_qualified_name_syntax;
    }
    if(!selected_qualified_name_syntax && !is_c_linkage) {
      throw logic_error("missing qualified-name syntax for function symbol " +
                        qualified_name);
    }
    symbol_linkage::SymbolIdentity symbol =
        selected_qualified_name_syntax ?
            symbol_linkage::make_function_symbol_identity(*selected_qualified_name_syntax,
                                                          name,
                                                          is_c_linkage,
                                                          type,
                                                          options,
                                                          object_symbol_key,
                                                          linkage) :
            symbol_linkage::make_c_function_symbol_identity(name, linkage);
    symbol.prefer_local_object_binding =
        function_prefers_local_object_binding(definition_node ? definition_node :
                                                 declaration_node,
                                              definition_node ? definition_node : nullptr,
                                              false,
                                              owner_class,
                                              is_constructor,
                                              is_destructor,
                                              is_copy_assignment,
                                              is_move_assignment);
    if(!is_c_linkage && !symbol.keep_internal_alias) {
      symbol.internal_symbol = allocate_internal_symbol(symbol.internal_symbol);
    } else if(used_internal_symbols.count(symbol.internal_symbol) != 0) {
      if(is_c_linkage && symbol_linkage::has_object_symbol(symbol)) {
        FunctionBinding * existing =
            find_existing_c_linkage_function_symbol(symbol.object_symbol, type);
        if(existing) {
          return existing->symbol;
        }
      }
      throw logic_error("duplicate internal symbol " + symbol.internal_symbol +
                        " [function " + qualified_name + "]");
    } else {
      used_internal_symbols.insert(symbol.internal_symbol);
    }
    claim_exported_symbol(symbol, describe_exported_symbol_owner("function", qualified_name, type));
    return symbol;
  }

  symbol_linkage::SymbolIdentity make_registered_function_symbol(Scope & scope,
                                                                 const string & name,
                                                                 const TypePtr & type,
                                                                 bool is_c_linkage,
                                                                 symbol_linkage::SymbolLinkage linkage,
                                                                 bool is_const_method = false,
                                                                 bool is_volatile_method = false,
                                                                 RefQualifier ref_qualifier = RQ_NONE,
                                                                 bool is_constructor = false,
                                                                 bool is_destructor = false,
                                                                 const FunctionTemplateRegistrationIdentity
                                                                     & template_identity =
                                                                     FunctionTemplateRegistrationIdentity(),
                                                                 const ClassInfo * owner_class = nullptr,
                                                                 bool prefer_overload_suffix = false,
                                                                 const CppAstNode * declaration_node = nullptr,
                                                                 const CppAstNode * definition_node = nullptr,
                                                                 bool is_copy_assignment = false,
                                                                 bool is_move_assignment = false)
  {
    const QualifiedName qualified_name_syntax =
        semantic_lookup::scope_symbol_qualified_name_syntax(scope, name);
    return make_registered_function_symbol(scope_symbol_qualified_name(scope, name),
                                           name,
                                           type,
                                           is_c_linkage,
                                           linkage,
                                           is_const_method,
                                           is_volatile_method,
                                           ref_qualifier,
                                           is_constructor,
                                           is_destructor,
                                           template_identity,
                                           owner_class,
                                           prefer_overload_suffix,
                                           declaration_node,
                                           definition_node,
                                           &scope,
                                           is_copy_assignment,
                                           is_move_assignment,
                                           &qualified_name_syntax);
  }

  symbol_linkage::SymbolIdentity make_registered_variable_symbol(Scope & scope,
                                                                 const string & name,
                                                                 const TypePtr & type,
                                                                 bool is_c_linkage,
                                                                 symbol_linkage::SymbolLinkage linkage =
                                                                     symbol_linkage::SL_EXTERNAL)
  {
    const string qualified_name = scope_symbol_qualified_name(scope, name);
    symbol_linkage::SymbolIdentity symbol =
        symbol_linkage::make_scoped_variable_symbol_identity(scope,
                                                            name,
                                                            is_c_linkage,
                                                            linkage);
    if(used_internal_symbols.count(symbol.internal_symbol) != 0) {
      throw logic_error("duplicate internal symbol " + symbol.internal_symbol +
                        " [variable " + qualified_name + "]");
    }
    used_internal_symbols.insert(symbol.internal_symbol);
    claim_exported_symbol(symbol, describe_exported_symbol_owner("variable", qualified_name, type));
    return symbol;
  }

  void upgrade_value_symbol_linkage(ValueBinding & binding,
                                    const string & qualified_name,
                                    symbol_linkage::SymbolLinkage linkage)
  {
    if(binding.symbol.linkage == linkage) {
      return;
    }
    if(!binding.declaration_scope) {
      throw logic_error("missing declaration scope for variable symbol " + qualified_name);
    }
    binding.symbol =
        symbol_linkage::make_scoped_variable_symbol_identity(*binding.declaration_scope,
                                                            binding.name,
                                                            binding.is_c_linkage,
                                                            linkage);
    claim_exported_symbol(binding.symbol,
                          describe_exported_symbol_owner("variable",
                                                         qualified_name,
                                                         binding.type));
  }

  void refresh_definition_parameter_names(FunctionBinding & binding,
                                          const vector<pair<string, TypePtr> > & explicit_params)
      override
  {
    const size_t explicit_offset =
        binding.is_method && !binding.params.empty() &&
                binding.params[0].first == "this" ?
            1 :
            0;
    if(binding.params.size() != explicit_params.size() + explicit_offset) {
      return;
    }
    for(size_t i = 0; i < explicit_params.size(); ++i) {
      if(!explicit_params[i].first.empty()) {
        binding.params[explicit_offset + i].first = explicit_params[i].first;
      }
    }
  }

  void refresh_definition_parameter_names(FunctionBinding & binding,
                                          const FunctionBinding & definition_binding) override
  {
    if(binding.params.size() != definition_binding.params.size()) {
      return;
    }
    for(size_t i = 0; i < definition_binding.params.size(); ++i) {
      if(i == 0 && binding.is_method && binding.params[i].first == "this") {
        continue;
      }
      if(!definition_binding.params[i].first.empty()) {
        binding.params[i].first = definition_binding.params[i].first;
      }
    }
  }

  void update_parameter_alias_after_canonical_rename(vector<string> & aliases,
                                                     size_t index,
                                                     const string & old_name,
                                                     const string & new_name)
  {
    if(index >= aliases.size()) {
      return;
    }
    if(aliases[index].empty() || aliases[index] == old_name) {
      aliases[index] = new_name;
    }
  }

  void record_definition_parameter_aliases(FunctionBinding & binding,
                                           const vector<pair<string, TypePtr> > & params)
  {
    callsemantic::record_definition_parameter_aliases(binding, params);
  }

  void record_definition_parameter_aliases(FunctionTemplateDecl & decl,
                                           const vector<pair<string, TypePtr> > & params)
  {
    callsemantic::record_definition_parameter_aliases(decl, params);
  }

  FunctionBinding * preserve_owner_prefixed_template_binding(FunctionBinding * original,
                                                             FunctionBinding * materialized)
  {
    if(!original || !materialized ||
       !template_api::should_preserve_owner_prefixed_template_identity(
           *original,
           *materialized,
           types_equivalent_for_member_binding(original->type, materialized->type))) {
      return materialized;
    }

    original->output_requirements |= materialized->output_requirements;
    original->exclude_from_explicit_instantiation =
        original->exclude_from_explicit_instantiation ||
        materialized->exclude_from_explicit_instantiation;
    original->is_constexpr = original->is_constexpr || materialized->is_constexpr;
    original->odr_mergeable_definition =
        original->odr_mergeable_definition ||
        materialized->odr_mergeable_definition;
    template_api::adopt_function_template_identity_from_materialized(
        *original,
        *materialized);
    upgrade_function_symbol_linkage(*original,
                                    original->name,
                                    original->display_name,
                                    original->type,
                                    original->symbol.linkage);
    if(!original->has_definition && materialized->has_definition) {
      original->body = materialized->body;
      original->ctor_initializer = materialized->ctor_initializer;
      original->has_definition = true;
      original->definition_node =
          materialized->definition_node ?
              materialized->definition_node :
              materialized->declaration_node;
      if(!original->parameter_syntax_node) {
        original->parameter_syntax_node = materialized->parameter_syntax_node;
      }
      if(!original->function_qualifier) {
        original->function_qualifier = materialized->function_qualifier;
      }
      if(!original->lexical_access_class) {
        original->lexical_access_class = materialized->lexical_access_class;
      }
      if(!original->lexical_access_function) {
        original->lexical_access_function = materialized->lexical_access_function;
      }
    }
    return original;
  }

  struct FunctionBindingRefreshKey
  {
    FunctionBinding * original = nullptr;
    ClassInfo * owner_class = nullptr;
    Scope * declaration_scope = nullptr;
    string name;
    string display_name;
    TypePtr type;
    RefQualifier ref_qualifier = RQ_NONE;
    bool is_method = false;
    bool is_constructor = false;
    bool is_destructor = false;
    symbol_linkage::SymbolIdentity symbol;
    FunctionTemplateRegistrationIdentity template_identity;
    unsigned int output_requirements = 0;
  };

  FunctionBindingRefreshKey make_function_binding_refresh_key(
      const FunctionBinding & binding) const
  {
    FunctionBindingRefreshKey key;
    key.original = const_cast<FunctionBinding *>(&binding);
    key.owner_class = binding.owner_class;
    key.declaration_scope = binding.declaration_scope;
    key.name = binding.name;
    key.display_name = binding.display_name;
    key.type = binding.type;
    key.ref_qualifier = binding.ref_qualifier;
    key.is_method = binding.is_method;
    key.is_constructor = binding.is_constructor;
    key.is_destructor = binding.is_destructor;
    key.symbol = binding.symbol;
    key.template_identity = template_api::function_binding_registration_identity(binding);
    key.output_requirements = binding.output_requirements;
    return key;
  }

  FunctionBinding * refresh_function_binding_after_class_update(
      const FunctionBindingRefreshKey & key,
      FunctionBinding * current)
  {
    FunctionBinding * refreshed = nullptr;
    if(key.owner_class &&
       (key.is_method || key.is_constructor || key.is_destructor)) {
      const string member_name =
          unqualified_member_name(canonical_function_lookup_name(key.name));
      refreshed = find_matching_class_function(*key.owner_class,
                                               member_name,
                                               key.type,
                                               key.template_identity,
                                               key.ref_qualifier);
      if(!refreshed &&
         !key.template_identity.has_decl() &&
         !key.template_identity.has_arguments() &&
         key.template_identity.key.empty()) {
        refreshed = find_equivalent_class_function(*key.owner_class,
                                                   member_name,
                                                   key.type,
                                                   key.ref_qualifier);
      }
    }
    if(!refreshed && key.owner_class && key.declaration_scope) {
      const string lookup_name =
          key.display_name.empty() ?
              unqualified_member_name(canonical_function_lookup_name(key.name)) :
              key.display_name;
      refreshed = find_matching_function(*key.declaration_scope,
                                         lookup_name,
                                         key.type,
                                         key.template_identity);
    }
    if(!refreshed && key.declaration_scope) {
      refreshed = find_matching_function(*key.declaration_scope,
                                         key.name,
                                         key.type,
                                         key.template_identity);
    }
    if(!refreshed && !key.symbol.internal_symbol.empty()) {
      refreshed = find_function_by_symbol(key.symbol, key.name, key.type);
    }
    if(!refreshed && function_binding_is_live(current)) {
      refreshed = current;
    }
    if(refreshed) {
      refreshed->output_requirements |= key.output_requirements;
    }
    return refreshed;
  }

  vector<const CppAstNode *> normalize_default_arguments(
      const vector<pair<string, TypePtr> > & params,
      const vector<const CppAstNode *> & default_arguments)
  {
    vector<const CppAstNode *> normalized_defaults(params.size(), nullptr);
    for(size_t i = 0; i < params.size() && i < default_arguments.size(); ++i) {
      normalized_defaults[i] = default_arguments[i];
    }
    return normalized_defaults;
  }

  void register_function(Scope & scope,
                         const string & name,
                         const TypePtr & type,
                         const vector<pair<string, TypePtr> > & params,
                         const vector<const CppAstNode *> & default_arguments,
                         const CppAstNode * body,
                         const CppAstNode * declaration_node = nullptr,
                         const CppAstNode * parameter_syntax_node = nullptr,
                         bool is_c_linkage = false,
                         Scope * declaration_scope = nullptr,
                         const CppAstNode * function_qualifier = nullptr,
                         bool is_constexpr = false,
                         const FunctionTemplateRegistrationIdentity & template_identity =
                             FunctionTemplateRegistrationIdentity(),
                         bool prefer_overload_suffix = false,
                         ClassInfo * lexical_access_class = nullptr,
                         FunctionBinding * lexical_access_function = nullptr)
  {
    const vector<const CppAstNode *> normalized_defaults =
        normalize_default_arguments(params, default_arguments);
    if(!scope.class_info &&
       requires_class_or_enum_operand_for_nonmember_operator(name)) {
      bool has_class_or_enum_operand = false;
      for(size_t i = 0; i < params.size(); ++i) {
        if(is_operator_overload_operand_type(params[i].second)) {
          has_class_or_enum_operand = true;
          break;
        }
      }
      if(!has_class_or_enum_operand) {
        throw logic_error(string("non-member overloaded operator requires a class or enum operand") +
                          semantic_trace::current_location_note(
                              *this,
                              declaration_node ? declaration_node : parameter_syntax_node));
      }
    }
    const auto scope_binding_weight = [](const Scope * candidate) -> std::size_t
    {
      if(!candidate) {
        return 0;
      }
      return candidate->named_types.size() +
             candidate->named_type_packs.size() +
             candidate->named_value_packs.size() +
             candidate->named_pack_sizes.size() +
             candidate->template_bound_type_names.size() +
             candidate->template_bound_type_pack_names.size() +
             candidate->template_bound_value_names.size() +
             candidate->template_bound_template_names.size() +
             candidate->values.size();
    };
    FunctionBinding * existing =
        find_matching_function(scope, name, type, template_identity);
    const string qualified_name = scope_qualified_name(scope, name);
    const symbol_linkage::SymbolLinkage linkage =
        function_symbol_linkage(scope,
                                declaration_node,
                                body,
                                is_c_linkage,
                                function_qualifier,
                                template_identity,
                                false,
                                lexical_access_class);
    if(existing) {
      template_api::adopt_materialized_owner_template_identity(
          *existing,
          template_identity,
          declaration_scope ? declaration_scope : &scope);
      vector<TypePtr> explicit_param_types;
      explicit_param_types.reserve(params.size());
      for(size_t i = 0; i < params.size(); ++i) {
        explicit_param_types.push_back(params[i].second);
      }
      if(template_identity.arguments) {
        template_api::record_function_template_arguments_preserving_pack_sizes(
            *existing,
            *template_identity.arguments,
            true);
      }
      ensure_function_parameter_aliases(*existing);
      upgrade_function_symbol_linkage(*existing, qualified_name, name, type, linkage);
      if(existing->params.size() == params.size()) {
        for(size_t i = 0; i < params.size(); ++i) {
          const string generated_name = string("arg") + to_string(i);
          const bool existing_unnamed =
              existing->params[i].first.empty() || existing->params[i].first == generated_name;
          if(existing_unnamed && !params[i].first.empty()) {
            const string old_name = existing->params[i].first;
            existing->params[i].first = params[i].first;
            update_parameter_alias_after_canonical_rename(existing->parameter_aliases,
                                                          i,
                                                          old_name,
                                                          params[i].first);
          }
          if(i < existing->default_arguments.size() &&
             !existing->default_arguments[i] &&
             normalized_defaults[i]) {
            existing->default_arguments[i] = normalized_defaults[i];
          }
        }
      }
      if(existing->is_c_linkage != is_c_linkage) {
        throw logic_error(string("mismatched function linkage") +
                          semantic_trace::current_location_note(*this, declaration_node) +
                          semantic_trace::previous_function_location_note(
                              *this, "previous declaration", existing));
      }
      if(!explicit_function_nothrow_specifications_match(*existing, function_qualifier)) {
        throw logic_error(string("mismatched function exception specification [cs-register]") +
                          semantic_trace::current_location_note(*this, declaration_node) +
                          semantic_trace::previous_function_location_note(
                              *this, "previous declaration", existing));
      }
      if(!existing->declaration_node && declaration_node) {
        existing->declaration_node = declaration_node;
      }
      if(existing->declaration_abi_tags.empty()) {
        append_function_declaration_abi_tags(existing->declaration_abi_tags, declaration_node);
      }
      if(!existing->parameter_syntax_node && parameter_syntax_node) {
        existing->parameter_syntax_node = parameter_syntax_node;
      }
      existing->exclude_from_explicit_instantiation =
          existing->exclude_from_explicit_instantiation ||
          declaration_marks_exclude_from_explicit_instantiation(declaration_node);
      if(template_identity.decl &&
         declaration_scope &&
         (!existing->declaration_scope ||
          existing->declaration_scope == &scope ||
          scope_binding_weight(existing->declaration_scope) <
              scope_binding_weight(declaration_scope))) {
        existing->declaration_scope = declaration_scope;
      }
      if(body) {
        if(existing->has_definition) {
          ostringstream out;
          out << "duplicate function definition";
          out << " [name " << name << "]";
          out << " [scope " << scope_qualified_name(scope, scope.name) << "]";
          out << " [type " << describe_type(type) << "]";
          out << semantic_trace::current_location_note(*this, declaration_node);
          out << semantic_trace::previous_function_location_note(
              *this, "previous definition", existing);
          throw logic_error(out.str());
        }
        refresh_definition_parameter_names(*existing, params);
        existing->body = body;
        existing->has_definition = true;
        existing->definition_node = declaration_node ? declaration_node : body;
        existing->definition_abi_tags.clear();
        append_function_declaration_abi_tags(existing->definition_abi_tags, declaration_node);
        if(parameter_syntax_node) {
          existing->parameter_syntax_node = parameter_syntax_node;
        }
        record_definition_parameter_aliases(*existing, params);
      }
      if(!existing->function_qualifier && function_qualifier) {
        existing->function_qualifier = function_qualifier;
      }
      if(!existing->lexical_access_class && lexical_access_class) {
        existing->lexical_access_class = lexical_access_class;
      }
      if(!existing->lexical_access_function && lexical_access_function) {
        existing->lexical_access_function = lexical_access_function;
      }
      existing->is_constexpr = existing->is_constexpr || is_constexpr;
      apply_builtin_host_runtime_symbol(*existing, explicit_param_types);
      return;
    }

    unique_ptr<FunctionBinding> binding(new FunctionBinding());
    binding->name = scope_qualified_name(scope, name);
    binding->display_name = name;
    binding->declaration_scope = declaration_scope ? declaration_scope : &scope;
    binding->declaration_node = declaration_node;
    append_function_declaration_abi_tags(binding->declaration_abi_tags, declaration_node);
    binding->exclude_from_explicit_instantiation =
        declaration_marks_exclude_from_explicit_instantiation(declaration_node);
    binding->definition_node = body ? (declaration_node ? declaration_node : body) : nullptr;
    if(binding->definition_node) {
      append_function_declaration_abi_tags(binding->definition_abi_tags, declaration_node);
    }
    binding->parameter_syntax_node =
        parameter_syntax_node ? parameter_syntax_node : declaration_node;
    binding->type = type;
    binding->params = params;
    initialize_function_parameter_aliases(*binding);
    binding->default_arguments = normalized_defaults;
    binding->body = body;
    binding->has_definition = body != nullptr;
    binding->function_qualifier = function_qualifier;
    binding->lexical_access_class = lexical_access_class;
    binding->lexical_access_function = lexical_access_function;
    binding->is_c_linkage = is_c_linkage;
    binding->symbol =
        make_registered_function_symbol(scope,
                                        name,
                                        type,
                                        is_c_linkage,
                                        linkage,
                                        false,
                                        false,
                                        RQ_NONE,
                                        false,
                                        false,
                                        template_identity,
                                        scope.class_info,
                                        prefer_overload_suffix,
                                        declaration_node,
                                        binding->definition_node);
    binding->odr_mergeable_definition =
        symbol_linkage::has_weak_linkage(binding->symbol);
    vector<TypePtr> explicit_param_types;
    explicit_param_types.reserve(params.size());
    for(size_t i = 0; i < params.size(); ++i) {
      explicit_param_types.push_back(params[i].second);
    }
    apply_builtin_host_runtime_symbol(*binding, explicit_param_types);
    binding->is_constexpr = is_constexpr;
    template_api::record_function_template_identity(*binding, template_identity);
    semantic_lookup::direct_function_set_slot(scope, name).push_back(binding.get());
    functions.push_back(std::move(binding));
    index_function_binding(functions.back().get());
  }

  void register_function(Scope & scope,
                         const string & name,
                         const TypePtr & type,
                         const vector<pair<string, TypePtr> > & params,
                         const CppAstNode * body,
                         const CppAstNode * declaration_node = nullptr,
                         const CppAstNode * parameter_syntax_node = nullptr,
                         bool is_c_linkage = false,
                         Scope * declaration_scope = nullptr,
                         const CppAstNode * function_qualifier = nullptr,
                         bool is_constexpr = false,
                         const FunctionTemplateRegistrationIdentity & template_identity =
                             FunctionTemplateRegistrationIdentity(),
                         bool prefer_overload_suffix = false,
                         ClassInfo * lexical_access_class = nullptr,
                         FunctionBinding * lexical_access_function = nullptr)
  {
    register_function(scope, name, type, params, vector<const CppAstNode *>(),
                      body, declaration_node, parameter_syntax_node, is_c_linkage,
                      declaration_scope, function_qualifier,
                      is_constexpr, template_identity, prefer_overload_suffix,
                      lexical_access_class, lexical_access_function);
  }

  ValueBinding & register_namespace_value(Scope & scope,
                                          const string & name,
                                          const TypePtr & type,
                                          bool is_c_linkage,
                                          bool is_thread_local,
                                          bool is_definition,
                                          const CppAstNode * declaration_node = nullptr,
                                          const CppAstNode * linkage_node = nullptr)
  {
    const CppAstNode * symbol_linkage_node =
        linkage_node ? linkage_node : declaration_node;
    const string qualified_name = scope_symbol_qualified_name(scope, name);
    map<string, ValueBinding>::iterator existing = scope.values.find(name);
    if(existing != scope.values.end()) {
      TypePtr merged = merge_types(existing->second.type, type);
      if(!merged) {
        throw logic_error(string("mismatched variable declaration") +
                          semantic_trace::current_location_note(*this, declaration_node) +
                          semantic_trace::previous_value_location_note(
                              *this, "previous declaration", &existing->second));
      }
      existing->second.type = merged;
      if(existing->second.is_c_linkage != is_c_linkage) {
        throw logic_error(string("mismatched variable linkage") +
                          semantic_trace::current_location_note(*this, declaration_node) +
                          semantic_trace::previous_value_location_note(
                              *this, "previous declaration", &existing->second));
      }
      if(existing->second.is_thread_local != is_thread_local) {
        throw logic_error(string("mismatched variable thread_local storage") +
                          semantic_trace::current_location_note(*this, declaration_node) +
                          semantic_trace::previous_value_location_note(
                              *this, "previous declaration", &existing->second));
      }
      if(is_definition && existing->second.has_storage_definition) {
        throw logic_error(string("duplicate variable definition") +
                          semantic_trace::current_location_note(*this, declaration_node) +
                          semantic_trace::previous_value_location_note(
                              *this, "previous definition", &existing->second));
      }
      if(!existing->second.declaration_node && declaration_node) {
        existing->second.declaration_node = declaration_node;
      }
      if(!existing->second.declaration_scope) {
        existing->second.declaration_scope = &scope;
      }
      symbol_linkage::SymbolLinkage linkage =
          variable_symbol_linkage(scope,
                                  symbol_linkage_node,
                                  is_c_linkage,
                                  existing->second.owner_class);
      if(linkage == symbol_linkage::SL_INTERNAL &&
         !existing->second.has_storage_definition &&
         existing->second.symbol.linkage != symbol_linkage::SL_INTERNAL) {
        linkage = existing->second.symbol.linkage;
      }
      upgrade_value_symbol_linkage(existing->second,
                                   qualified_name,
                                   linkage);
      existing->second.has_storage_definition =
          existing->second.has_storage_definition || is_definition;
      if(is_definition && !existing->second.definition_node) {
        existing->second.definition_node = declaration_node;
      }
      return existing->second;
    }

    ValueBinding binding(ValueBinding::VK_VARIABLE, name, type);
    binding.is_c_linkage = is_c_linkage;
    binding.is_thread_local = is_thread_local;
    binding.has_storage_definition = is_definition;
    binding.declaration_scope = &scope;
    binding.declaration_node = declaration_node;
    binding.definition_node = is_definition ? declaration_node : nullptr;
    binding.symbol =
        make_registered_variable_symbol(scope,
                                        name,
                                        type,
                                        is_c_linkage,
                                        variable_symbol_linkage(scope,
                                                                symbol_linkage_node,
                                                                is_c_linkage,
                                                                binding.owner_class));
    map<string, ValueBinding>::iterator inserted =
        scope.values.insert(make_pair(name, binding)).first;
    return inserted->second;
  }

  const TemplateParameterInfo * find_template_parameter(const vector<TemplateParameterInfo> & params,
                                                        const string & placeholder_key) const
  {
    return template_model::find_template_parameter(params, placeholder_key);
  }

  bool is_dependent_non_type_parameter_type(const vector<TemplateParameterInfo> & parameters,
                                            const TypePtr & type) const
  {
    TypePtr base = strip_top_level_cv(type);
    if(!base || base->kind != Type::TK_NAMED) {
      return false;
    }
    if(named_type_has_dependent_semantic(base)) {
      return true;
    }
    const TemplateParameterInfo * parameter =
        find_template_parameter(parameters, base->named_key);
    return parameter && parameter->kind == TemplateParameterInfo::TP_TYPE;
  }

  bool is_supported_non_type_template_parameter_type(
      const vector<TemplateParameterInfo> & parameters,
      const TypePtr & type) const
  {
    TypePtr base = strip_top_level_cv(type);
    if(!base) {
      return false;
    }
    if(is_integral_type(base) || is_named_enum_type(base) || is_pointer_type(base)) {
      return true;
    }
    if(base->kind == Type::TK_LVALUE_REFERENCE) {
      return true;
    }
    if(base->kind == Type::TK_FUNDAMENTAL && base->fundamental == FT_NULLPTR_T) {
      return true;
    }
    return is_dependent_non_type_parameter_type(parameters, type);
  }

  bool substitute_type(const TypePtr & type,
                       const vector<TemplateParameterInfo> & parameters,
                       const vector<TemplateArgument> & arguments,
                       TypePtr & out)
  {
    return template_api::type::substitute_type(type, parameters, arguments, out);
  }

  bool resolve_instantiated_dependent_type(Scope & scope,
                                           const TypePtr & type,
                                           TypePtr & out)
  {
    if(!type) {
      return false;
    }

    semantic_cache::DependentTypeResolutionCacheKey cache_key;
    const std::size_t cache_scope_key = scope_cache_key(scope);
    std::string cache_text;
    const bool cacheable = try_dependent_type_resolution_cache_key(type, cache_text);
    if(!cacheable) {
      if(parser_trace::enabled("template.resolve")) {
        std::ostringstream trace;
        trace << "resolve-instantiated-dependent-type bypass-cyclic-cache"
              << " scope-key=" << cache_scope_key
              << " type=" << describe_type(type);
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      return template_api::type::resolve_instantiated_dependent_type(*this, scope, type, out);
    }
    cache_key.scope_key = cache_scope_key;
    cache_key.type_key = intern_query_text(cache_text);
    map<semantic_cache::DependentTypeResolutionCacheKey,
        DependentTypeResolutionCacheEntry,
        semantic_cache::DependentTypeResolutionCacheKeyLess>::iterator found =
        cache_state_.dependent_type_resolution_cache.find(cache_key);
    if(found != cache_state_.dependent_type_resolution_cache.end()) {
      metrics_.note_cache_hit(semantic_metrics::CK_DEPENDENT_TYPE_RESOLUTION);
      if(found->second.status == DependentTypeResolutionCacheEntry::DTS_RESOLVED) {
        if(!found->second.resolved) {
          if(parser_trace::enabled("template.resolve")) {
            std::ostringstream trace;
            trace << "resolve-instantiated-dependent-type invalid-cached-null key="
                  << (cache_key.type_key ? *cache_key.type_key : string("<null>"))
                  << " scope-key=" << cache_key.scope_key;
            parser_trace::note("template.resolve", std::string(), trace.str());
          }
          found->second.status = DependentTypeResolutionCacheEntry::DTS_UNRESOLVED;
          return false;
        }
        out = found->second.resolved;
        return true;
      }
      return false;
    }
    metrics_.note_cache_miss(semantic_metrics::CK_DEPENDENT_TYPE_RESOLUTION);

    DependentTypeResolutionCacheEntry & entry =
        cache_state_.dependent_type_resolution_cache[cache_key];
    entry.status = DependentTypeResolutionCacheEntry::DTS_IN_PROGRESS;

    TypePtr resolved;
    const bool ok =
        template_api::type::resolve_instantiated_dependent_type(*this, scope, type, resolved);
    if(ok && resolved) {
      entry.status = DependentTypeResolutionCacheEntry::DTS_RESOLVED;
      entry.resolved = resolved;
      out = resolved;
      return true;
    }

    if(ok && !resolved && parser_trace::enabled("template.resolve")) {
      std::ostringstream trace;
      trace << "resolve-instantiated-dependent-type invalid-null key="
            << (cache_key.type_key ? *cache_key.type_key : string("<null>"))
            << " scope-key=" << cache_key.scope_key;
      parser_trace::note("template.resolve", std::string(), trace.str());
    }

    entry.status = DependentTypeResolutionCacheEntry::DTS_UNRESOLVED;
    entry.resolved.reset();
    return false;
  }

  bool parse_out_of_class_member_qualified_name(const string & qualified_name,
                                                QualifiedName & out) override
  {
    return split_qualified_name_text(qualified_name, out) && !out.qualifiers.empty();
  }

  string qualified_name_syntax_text(const QualifiedName & qualified) const
  {
    string out;
    if(qualified.rooted) {
      out += "::";
    }
    for(size_t i = 0; i < qualified.qualifiers.size(); ++i) {
      if(!out.empty() && out != "::") {
        out += "::";
      }
      out += qualified.qualifiers[i];
    }
    if(!out.empty() && out != "::") {
      out += "::";
    }
    out += qualified.name;
    return out;
  }

  string qualified_owner_name_text(const QualifiedName & qualified) const
  {
    string class_name;
    if(qualified.rooted) {
      class_name += "::";
    }
    for(size_t i = 0; i < qualified.qualifiers.size(); ++i) {
      if(!class_name.empty() && class_name != "::") {
        class_name += "::";
      }
      class_name += qualified.qualifiers[i];
    }
    return class_name;
  }

  ClassInfo * resolve_out_of_class_owner_class(Scope & scope,
                                               const QualifiedName & qualified,
                                               QualifiedOwnerClassResolution resolution =
                                                   QualifiedOwnerClassResolution::Complete)
      override
  {
    if(qualified.qualifiers.empty()) {
      return nullptr;
    }
    const string class_name = qualified_owner_name_text(qualified);
    return resolve_qualified_owner_class(scope, class_name, resolution);
  }

  bool resolve_out_of_class_named_method_binding(Scope & scope,
                                                 const QualifiedName & qualified,
                                                 const string & member_name,
                                                 const TypePtr & declared_type,
                                                 bool is_const_method,
                                                 bool is_volatile_method,
                                                 RefQualifier ref_qualifier,
                                                 FunctionBinding *& out,
                                                 QualifiedOwnerClassResolution resolution =
                                                     QualifiedOwnerClassResolution::Complete)
      override
  {
    ClassInfo * info = resolve_out_of_class_owner_class(scope, qualified, resolution);
    if(!info) {
      return false;
    }
    if(info->member_scope && !info->reference_member_collection_in_progress) {
      ensure_class_reference_members(*info);
    }

    const TypePtr expected_type =
        method_function_type(info->type,
                             is_const_method,
                             is_volatile_method,
                             declared_type);
    out = find_exact_class_function(*info,
                                    member_name,
                                    expected_type,
                                    ref_qualifier);
    if(out) {
      return true;
    }

    out = find_equivalent_class_function(*info,
                                         member_name,
                                         expected_type,
                                         ref_qualifier);
    if(out) {
      return true;
    }

    out = find_exact_function(*info->member_scope, member_name, declared_type);
    if(!out && is_conversion_function_name(member_name)) {
      FunctionBinding * by_type = nullptr;
      bool ambiguous_conversion = false;
      for(map<string, vector<FunctionBinding *> >::const_iterator it = info->methods.begin();
          it != info->methods.end(); ++it) {
        if(!is_conversion_function_name(it->first)) {
          continue;
        }
        for(size_t i = 0; i < it->second.size(); ++i) {
          FunctionBinding * candidate = it->second[i];
          if(candidate->owner_class != info ||
             candidate->ref_qualifier != ref_qualifier ||
             !type_equals(candidate->type, expected_type)) {
            continue;
          }
          if(by_type && by_type != candidate) {
            ambiguous_conversion = true;
            break;
          }
          by_type = candidate;
        }
        if(ambiguous_conversion) {
          break;
        }
      }
      if(!ambiguous_conversion) {
        out = by_type;
      }
      if(!out) {
        FunctionBinding * unique_conversion = nullptr;
        for(map<string, vector<FunctionBinding *> >::const_iterator it = info->methods.begin();
            it != info->methods.end(); ++it) {
          if(!is_conversion_function_name(it->first)) {
            continue;
          }
          for(size_t i = 0; i < it->second.size(); ++i) {
            FunctionBinding * candidate = it->second[i];
            if(candidate->owner_class != info ||
               candidate->ref_qualifier != ref_qualifier) {
              continue;
            }
            if(unique_conversion && unique_conversion != candidate) {
              unique_conversion = nullptr;
              ambiguous_conversion = true;
              break;
            }
            unique_conversion = candidate;
          }
          if(ambiguous_conversion) {
            break;
          }
        }
        if(!ambiguous_conversion) {
          out = unique_conversion;
        }
      }
    }
    if(!out && parser_trace::enabled("template.resolve")) {
      ostringstream trace;
      trace << "out-of-class-method-miss owner=" << info->qualified_name
            << " requested=" << member_name
            << " declared_type=" << describe_type(declared_type)
            << " is_const=" << (is_const_method ? "yes" : "no")
            << " is_volatile=" << (is_volatile_method ? "yes" : "no")
            << " refq=" << static_cast<int>(ref_qualifier)
            << " method_keys";
      for(map<string, vector<FunctionBinding *> >::const_iterator it = info->methods.begin();
          it != info->methods.end(); ++it) {
        trace << " " << it->first;
      }
      const vector<FunctionBinding *> & candidates = info->methods[member_name];
      trace << " candidate_count=" << candidates.size();
      for(size_t i = 0; i < candidates.size(); ++i) {
        trace << " [" << i << " type=" << describe_type(candidates[i]->type) << "]";
      }
      trace << " conversion_candidates";
      for(map<string, vector<FunctionBinding *> >::const_iterator it = info->methods.begin();
          it != info->methods.end(); ++it) {
        if(!is_conversion_function_name(it->first)) {
          continue;
        }
        for(size_t i = 0; i < it->second.size(); ++i) {
          trace << " [" << it->first
                << " type=" << describe_type(it->second[i]->type)
                << " refq=" << static_cast<int>(it->second[i]->ref_qualifier)
                << "]";
        }
      }
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
    return out != nullptr && out->owner_class == info;
  }

  bool resolve_out_of_class_named_method_binding(Scope & scope,
                                                 const string & qualified_name,
                                                 const string & member_name,
                                                 const TypePtr & declared_type,
                                                 bool is_const_method,
                                                 bool is_volatile_method,
                                                 RefQualifier ref_qualifier,
                                                 FunctionBinding *& out,
                                                 QualifiedOwnerClassResolution resolution =
                                                     QualifiedOwnerClassResolution::Complete)
      override
  {
    QualifiedName qualified;
    if(!parse_out_of_class_member_qualified_name(qualified_name, qualified)) {
      return false;
    }
    return resolve_out_of_class_named_method_binding(scope,
                                                     qualified,
                                                     member_name,
                                                     declared_type,
                                                     is_const_method,
                                                     is_volatile_method,
                                                     ref_qualifier,
                                                     out,
                                                     resolution);
  }

  bool resolve_out_of_class_static_member_binding(Scope & scope,
                                                  const QualifiedName & qualified,
                                                  ValueBinding *& out) override
  {
    ClassInfo * info = resolve_out_of_class_owner_class(scope, qualified);
    if(!info) {
      return false;
    }
    if(info->member_scope && !info->reference_member_collection_in_progress) {
      ensure_class_reference_members(*info);
    }

    map<string, ValueBinding>::iterator found = info->member_scope->values.find(qualified.name);
    if(found == info->member_scope->values.end() ||
       found->second.kind != ValueBinding::VK_VARIABLE ||
       found->second.owner_class != info) {
      return false;
    }

    out = &found->second;
    return true;
  }

  bool resolve_out_of_class_static_member_binding(Scope & scope,
                                                  const string & qualified_name,
                                                  ValueBinding *& out) override
  {
    QualifiedName qualified;
    if(!parse_out_of_class_member_qualified_name(qualified_name, qualified)) {
      return false;
    }
    return resolve_out_of_class_static_member_binding(scope, qualified, out);
  }

  bool resolve_out_of_class_method_binding_with_resolution(
      Scope & scope,
      const QualifiedName & qualified,
      const TypePtr & declared_type,
      bool is_const_method,
      bool is_volatile_method,
      RefQualifier ref_qualifier,
      FunctionBinding *& out,
      QualifiedOwnerClassResolution resolution) override
  {
    return resolve_out_of_class_named_method_binding(scope,
                                                     qualified,
                                                     qualified.name,
                                                     declared_type,
                                                     is_const_method,
                                                     is_volatile_method,
                                                     ref_qualifier,
                                                     out,
                                                     resolution);
  }

  bool resolve_out_of_class_method_binding(Scope & scope,
                                           const QualifiedName & qualified,
                                           const TypePtr & declared_type,
                                           bool is_const_method,
                                           bool is_volatile_method,
                                           RefQualifier ref_qualifier,
                                           FunctionBinding *& out)
  {
    return resolve_out_of_class_method_binding_with_resolution(
        scope,
        qualified,
        declared_type,
        is_const_method,
        is_volatile_method,
        ref_qualifier,
        out,
        QualifiedOwnerClassResolution::Complete);
  }

  bool resolve_out_of_class_method_binding_with_resolution(
      Scope & scope,
      const string & qualified_name,
      const TypePtr & declared_type,
      bool is_const_method,
      bool is_volatile_method,
      RefQualifier ref_qualifier,
      FunctionBinding *& out,
      QualifiedOwnerClassResolution resolution) override
  {
    QualifiedName qualified;
    if(!parse_out_of_class_member_qualified_name(qualified_name, qualified)) {
      return false;
    }
    return resolve_out_of_class_method_binding_with_resolution(scope,
                                                               qualified,
                                                               declared_type,
                                                               is_const_method,
                                                               is_volatile_method,
                                                               ref_qualifier,
                                                               out,
                                                               resolution);
  }

  bool resolve_out_of_class_method_binding(Scope & scope,
                                           const string & qualified_name,
                                           const TypePtr & declared_type,
                                           bool is_const_method,
                                           bool is_volatile_method,
                                           RefQualifier ref_qualifier,
                                           FunctionBinding *& out)
  {
    return resolve_out_of_class_method_binding_with_resolution(
        scope,
        qualified_name,
        declared_type,
        is_const_method,
        is_volatile_method,
        ref_qualifier,
        out,
        QualifiedOwnerClassResolution::Complete);
  }

  bool resolve_out_of_class_special_member_binding(Scope & scope,
                                                   const QualifiedName & qualified,
                                                   const vector<pair<string, TypePtr> > & params,
                                                   FunctionBinding *& out)
  {
    ClassInfo * info = resolve_out_of_class_owner_class(scope, qualified);
    if(!info) {
      return false;
    }
    if(info->member_scope && !info->reference_member_collection_in_progress) {
      ensure_class_reference_members(*info);
    }

    vector<TypePtr> effective_params;
    effective_params.push_back(make_pointer(info->type));
    for(size_t i = 0; i < params.size(); ++i) {
      effective_params.push_back(params[i].second);
    }

    const TypePtr expected_type =
        make_function(make_fundamental(FT_VOID), effective_params, false);

    out = find_exact_class_function(*info, qualified.name, expected_type);
    if(out) {
      return true;
    }

    out = find_equivalent_class_function(*info, qualified.name, expected_type);
    if(out) {
      return true;
    }

    const auto explicit_special_member_params_match =
        [&](FunctionBinding & candidate) -> bool
    {
      const auto normalize_named_text =
          [&](string text) -> string
      {
        text = trim_space(text);
        const string typename_prefix = "typename ";
        while(text.compare(0, typename_prefix.size(), typename_prefix) == 0) {
          text = trim_space(text.substr(typename_prefix.size()));
        }
        return text;
      };

      std::function<bool(const TypePtr &, const TypePtr &)> explicit_param_types_match =
          [&](const TypePtr & lhs, const TypePtr & rhs) -> bool
      {
        if(type_equals(lhs, rhs)) {
          return true;
        }
        if(!lhs || !rhs || lhs->kind != rhs->kind) {
          return false;
        }

        switch(lhs->kind) {
        case Type::TK_FUNDAMENTAL:
          return lhs->fundamental == rhs->fundamental;

        case Type::TK_NAMED:
        {
          const string lhs_key = normalize_named_text(lhs->named_key);
          const string rhs_key = normalize_named_text(rhs->named_key);
          const string lhs_display = normalize_named_text(lhs->named_display);
          const string rhs_display = normalize_named_text(rhs->named_display);
          return lhs_key == rhs_key || lhs_key == rhs_display ||
                 lhs_display == rhs_key || lhs_display == rhs_display;
        }

        case Type::TK_CV:
          return lhs->cv_const == rhs->cv_const &&
                 lhs->cv_volatile == rhs->cv_volatile &&
                 explicit_param_types_match(lhs->inner, rhs->inner);

        case Type::TK_ATOMIC:
        case Type::TK_POINTER:
        case Type::TK_BLOCK_POINTER:
        case Type::TK_LVALUE_REFERENCE:
        case Type::TK_RVALUE_REFERENCE:
          return explicit_param_types_match(lhs->inner, rhs->inner);

        case Type::TK_MEMBER_POINTER:
          return explicit_param_types_match(lhs->owner, rhs->owner) &&
                 explicit_param_types_match(lhs->inner, rhs->inner);

        case Type::TK_ARRAY:
          return lhs->has_bound == rhs->has_bound &&
                 lhs->bound == rhs->bound &&
                 lhs->bound_text == rhs->bound_text &&
                 explicit_param_types_match(lhs->inner, rhs->inner);

        case Type::TK_FUNCTION:
          if(lhs->variadic != rhs->variadic ||
             lhs->prototype_relaxed != rhs->prototype_relaxed ||
             lhs->function_const != rhs->function_const ||
             lhs->function_volatile != rhs->function_volatile ||
             lhs->params.size() != rhs->params.size() ||
             !explicit_param_types_match(lhs->inner, rhs->inner)) {
            return false;
          }
          for(size_t i = 0; i < lhs->params.size(); ++i) {
            if(!explicit_param_types_match(lhs->params[i], rhs->params[i])) {
              return false;
            }
          }
          return true;
        }

        return false;
      };

      if(candidate.ref_qualifier != RQ_NONE) {
        return false;
      }

      TypePtr candidate_type = strip_top_level_cv(candidate.type);
      if(!candidate_type || candidate_type->kind != Type::TK_FUNCTION ||
         candidate_type->params.size() != params.size() + 1) {
        return false;
      }

      for(size_t i = 0; i < params.size(); ++i) {
        TypePtr expected_param = params[i].second;
        TypePtr resolved_expected;
        if(info->member_scope &&
           resolve_instantiated_dependent_type(*info->member_scope,
                                              expected_param,
                                              resolved_expected) &&
           resolved_expected) {
          expected_param = resolved_expected;
        }

        TypePtr candidate_param = candidate_type->params[i + 1];
        TypePtr resolved_candidate;
        if(info->member_scope &&
           resolve_instantiated_dependent_type(*info->member_scope,
                                              candidate_param,
                                              resolved_candidate) &&
           resolved_candidate) {
          candidate_param = resolved_candidate;
        }

        if(!explicit_param_types_match(candidate_param, expected_param)) {
          return false;
        }
      }
      return true;
    };

    FunctionBinding * matched = nullptr;
    const vector<FunctionBinding *> & candidates = info->methods[qualified.name];
    for(size_t i = 0; i < candidates.size(); ++i) {
      FunctionBinding * candidate = candidates[i];
      if(!candidate || !explicit_special_member_params_match(*candidate)) {
        continue;
      }
      if(matched && matched != candidate) {
        return false;
      }
      matched = candidate;
    }
    out = matched;
    return out != nullptr;
  }

  bool resolve_out_of_class_special_member_binding(Scope & scope,
                                                   const string & qualified_name,
                                                   const vector<pair<string, TypePtr> > & params,
                                                   FunctionBinding *& out)
  {
    QualifiedName qualified;
    if(!parse_out_of_class_member_qualified_name(qualified_name, qualified)) {
      return false;
    }
    return resolve_out_of_class_special_member_binding(scope, qualified, params, out);
  }

  void emit_out_of_class_owner_class_use_if_needed_impl(
      Scope & scope,
      const string & qualified_name,
      const QualifiedName * qualified_syntax,
      const CppAstNode * anchor_node,
      ClassInfo * owner_override,
      const vector<TemplateParameterInfo> * canonical_parameters)
  {
    const bool trace_enabled = parser_trace::enabled("template.resolve");
    if(trace_enabled) {
      std::ostringstream trace;
      trace << "owner-class-use-enter"
            << " qualified=" << qualified_name
            << " anchor=" << (anchor_node ? "yes" : "no")
            << " owner-override="
            << (owner_override ? owner_override->qualified_name : std::string("<none>"))
            << " params="
            << (canonical_parameters ? canonical_parameters->size() : 0);
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
    if(template_witness_context().session == nullptr) {
      if(trace_enabled) {
        parser_trace::note("template.resolve",
                           std::string(),
                           "owner-class-use-skip session");
      }
      return;
    }
    if(parser_trace::use_location_suppressed() && anchor_node == nullptr) {
      if(trace_enabled) {
        parser_trace::note("template.resolve",
                           std::string(),
                           "owner-class-use-skip use-location-paused");
      }
      return;
    }

    const bool parsed_qualified =
        qualified_syntax != nullptr &&
        !qualified_syntax->qualifiers.empty();

    string owner_name;
    if(parsed_qualified) {
      owner_name = qualified_owner_name_text(*qualified_syntax);
    } else {
      const std::size_t split = semantic_utils::top_level_scope_split(qualified_name);
      if(split == std::string::npos) {
        if(trace_enabled) {
          parser_trace::note("template.resolve",
                             std::string(),
                             "owner-class-use-skip split");
        }
        return;
      }
      owner_name = trim_space(qualified_name.substr(0, split));
    }
    if(owner_name.empty()) {
      if(trace_enabled) {
        parser_trace::note("template.resolve",
                           std::string(),
                           "owner-class-use-skip owner-name");
      }
      return;
    }

    std::string event_location;
    std::string anchor_identifier;
    const bool explicit_pattern_anchor_context =
        owner_override &&
        canonical_parameters &&
        !canonical_parameters->empty();
    if(anchor_node) {
      std::string anchor_name =
          parsed_qualified ? qualified_syntax->qualifiers.front() : owner_name;
      anchor_name = strip_trailing_top_level_template_arguments(anchor_name);
      anchor_identifier = anchor_name;

      if(!anchor_name.empty()) {
        event_location =
            template_api::normalize_template_witness_source_location(
                source_location_for_name_in_subtree(*anchor_node, anchor_name));
      }
      if(event_location.empty()) {
        event_location =
            template_api::normalize_template_witness_source_location(
                source_location_for_node(*anchor_node));
      }
      const std::string anchor_node_location =
          template_api::normalize_template_witness_source_location(
              source_location_for_node(*anchor_node));
      if(!anchor_node_location.empty() &&
         (anchor_identifier.empty() ||
          source_location_points_at_identifier(anchor_node_location,
                                               anchor_identifier) ||
          explicit_pattern_anchor_context)) {
        event_location =
            prefer_later_source_location(event_location, anchor_node_location);
      }
      if(!anchor_identifier.empty() &&
         !source_location_points_at_identifier(event_location, anchor_identifier) &&
         !explicit_pattern_anchor_context) {
        event_location.clear();
      }
    }
    if(event_location.empty()) {
      event_location =
          template_api::normalize_template_witness_source_location(
              earliest_qualified_use_location_for_value(qualified_name));
    }
    const std::string prefix_event_location =
        template_api::normalize_template_witness_source_location(
            earliest_qualified_use_location_for_prefix(owner_name));
    event_location =
        prefer_later_source_location(event_location, prefix_event_location);
    if(event_location.empty()) {
      if(trace_enabled) {
        parser_trace::note("template.resolve",
                           std::string(),
                           "owner-class-use-skip event-location");
      }
      return;
    }

    ClassInfo * owner = owner_override ?
        owner_override :
        resolve_qualified_owner_class(scope, owner_name);
    template_api::ClassTemplateUseInfo owner_use;
    if(!template_api::class_template_use_info_for_class(*this,
                                                                  scope,
                                                                  owner,
                                                                  owner_use)) {
      if(trace_enabled) {
        std::ostringstream trace;
        trace << "owner-class-use-skip owner"
              << " resolved=" << (owner ? owner->qualified_name : std::string("<none>"));
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      return;
    }

    const bool owner_explicit_pattern_context =
        explicit_pattern_anchor_context &&
        owner_use.dependent_arguments &&
        owner_use.selection.kind != template_api::MS_PRIMARY;
    const bool owner_primary_pattern_only =
        !owner_use.selects_specialized_definition &&
        !owner_use.explicit_case &&
        !owner_explicit_pattern_context &&
        owner_use.dependent_arguments;
    if(owner_primary_pattern_only) {
      if(trace_enabled) {
        parser_trace::note("template.resolve",
                           std::string(),
                           "owner-class-use-skip primary-pattern-only");
      }
      return;
    }
    witness::ClassUseEmitRequest request;
    request.location = event_location;
    request.template_name = strip_trailing_top_level_template_arguments(
        semantic_model::class_output_qualified_name(*owner));
    const std::string resolved_owner_identifier =
        unqualified_member_name(request.template_name).empty() ?
            request.template_name :
            unqualified_member_name(request.template_name);
    if(!source_location_points_at_identifier(event_location,
                                             resolved_owner_identifier)) {
      return;
    }
    if(source_location_points_at_identifier(event_location,
                                            resolved_owner_identifier)) {
      request.use_anchor_present = true;
      request.use_anchor_location = event_location;
    }

    if(owner_use.selects_specialized_definition ||
       owner_use.explicit_case ||
       owner_explicit_pattern_context) {
      const auto append_owner_binding_texts =
          [this, canonical_parameters](
              const template_api::ClassTemplateUseInfo & info,
              std::vector<witness::TemplateWitnessSourceBinding> & out_bindings) -> bool
      {
        if(!(info.origin && info.parameters && info.arguments)) {
          return false;
        }
        const std::vector<TemplateParameterInfo> & parameters =
            *info.parameters;
        std::vector<std::string> rendered_arg_texts;
        if(canonical_parameters && !canonical_parameters->empty()) {
          rendered_arg_texts.reserve(info.arguments->size());
          for(std::size_t i = 0; i < info.arguments->size(); ++i) {
            rendered_arg_texts.push_back(template_model::template_argument_text(
                (*info.arguments)[i],
                [&](const TypePtr & type) -> std::string
                {
                  std::string text = lookup_text_for_type_argument(type);
                  if(text.empty()) {
                    text = describe_type(type);
                  }
                  return text;
                }));
          }
        } else {
          rendered_arg_texts = info.canonical_texts && !info.canonical_texts->empty() ?
              *info.canonical_texts :
              canonical_instantiation_arg_texts(*info.arguments);
        }
        const auto canonicalize_parameter_text =
            [&](const std::string & text) -> std::string
        {
          if(text.empty()) {
            return text;
          }

          std::string out = text;
          if(canonical_parameters && !canonical_parameters->empty()) {
            auto stable_parameter_placeholder =
                [](const TemplateParameterInfo & parameter,
                   std::size_t index) -> std::string
            {
              switch(parameter.kind) {
              case TemplateParameterInfo::TP_NON_TYPE:
                return std::string("value-parameter-0-") + std::to_string(index);
              case TemplateParameterInfo::TP_TEMPLATE_TEMPLATE:
                return std::string("template-parameter-0-") + std::to_string(index);
              case TemplateParameterInfo::TP_TYPE:
              default:
                return std::string("type-parameter-0-") + std::to_string(index);
              }
            };
            auto replace_all_plain =
                [](std::string & target,
                   const std::string & from,
                   const std::string & to,
                   bool & changed)
            {
              if(from.empty() || from == to) {
                return;
              }
              std::size_t pos = 0;
              while((pos = target.find(from, pos)) != std::string::npos) {
                target.replace(pos, from.size(), to);
                pos += to.size();
                changed = true;
              }
            };
            std::vector<std::pair<std::string, std::string> > placeholders;
            for(std::size_t i = 0; i < canonical_parameters->size(); ++i) {
              const TemplateParameterInfo & parameter = (*canonical_parameters)[i];
              const std::string replacement =
                  stable_parameter_placeholder(parameter, i);
              const std::string placeholder =
                  "__cppgm_witness_placeholder_" + std::to_string(i) + "__";
              bool parameter_changed = false;
              replace_all_plain(out,
                                parameter.placeholder_key,
                                placeholder,
                                parameter_changed);
              replace_all_plain(out,
                                std::string("template-parameter ") +
                                    parameter.placeholder_key,
                                placeholder,
                                parameter_changed);
              auto replace_name = [&](const std::string & name)
              {
                if(name.empty()) {
                  return;
                }
                out = replace_elaborated_identifier_token_text(
                    out, name, placeholder, parameter_changed);
                out = replace_identifier_token_text(
                    out, name, placeholder, parameter_changed);
              };
              replace_name(parameter.name);
              for(std::size_t j = 0; j < parameter.alternate_names.size(); ++j) {
                replace_name(parameter.alternate_names[j]);
              }
              if(parameter_changed) {
                placeholders.push_back(std::make_pair(placeholder, replacement));
              }
            }
            for(std::size_t i = 0; i < placeholders.size(); ++i) {
              bool ignored = false;
              replace_all_plain(
                  out, placeholders[i].first, placeholders[i].second, ignored);
            }
          }
          out = trim_space(collapse_reparseable_scope_operators(out));
          const std::string prefixes[] = {"typename ", "class ", "struct "};
          bool stripped = true;
          while(stripped) {
            stripped = false;
            for(std::size_t i = 0; i < sizeof(prefixes) / sizeof(prefixes[0]); ++i) {
              if(out.compare(0, prefixes[i].size(), prefixes[i]) != 0) {
                continue;
              }
              const std::string candidate =
                  trim_space(out.substr(prefixes[i].size()));
              if(candidate.find("::") != std::string::npos) {
                continue;
              }
              out = candidate;
              stripped = true;
              break;
            }
          }
          return out;
        };
        const std::size_t count =
            std::min(parameters.size(), rendered_arg_texts.size());
        if(count == 0) {
          return false;
        }
        for(std::size_t i = 0; i < count; ++i) {
          witness::TemplateWitnessSourceBinding binding;
          binding.param = parameters[i].name.empty() ?
              std::string("$") + std::to_string(i + 1) :
              parameters[i].name;
          binding.arg = canonicalize_parameter_text(rendered_arg_texts[i]);
          binding.source = "explicit";
          binding.type_like =
              parameters[i].kind == TemplateParameterInfo::TP_TYPE;
          out_bindings.push_back(binding);
        }
        return true;
      };

      request.selection = witness::SourceSelectionKind::ExplicitSpecialization;
      const semantic_model::SourceDeclAnchorCache & owner_anchor =
          semantic_trace::class_decl_anchor(*this, owner);
      witness::set_selected_decl_anchor(request.selected_decl_location,
                                        request.selected_decl_anchor,
                                        owner_anchor);
      if(!append_owner_binding_texts(owner_use, request.bindings)) {
        template_api::append_class_template_witness_bindings(*this,
                                                             owner,
                                                             request.bindings);
      }
    } else {
      request.selection = source_selection_kind_for_match_kind(owner_use.selection.kind);
      witness::set_selected_decl_anchor(
          request.selected_decl_location,
          request.selected_decl_anchor,
          class_use_selected_decl_anchor(owner_use.origin, owner_use.selection));
      template_api::append_template_witness_source_bindings(
          *this,
          request.bindings,
          *owner_use.parameters,
          *owner_use.arguments,
          "explicit");
      if(owner_use.selection.parameters &&
         owner_use.selection.parameters != owner_use.parameters) {
        template_api::append_template_witness_source_bindings(
            *this,
            request.specialization_bindings,
            *owner_use.selection.parameters,
            owner_use.selection.arguments,
            "deduced");
      }
    }
    request.role = witness::SourceUseRole::QualifierUse;
    witness::emit_class_use(request);
    if(trace_enabled) {
      std::ostringstream trace;
      trace << "owner-class-use-recorded"
            << " location=" << request.location
            << " template=" << request.template_name
            << " selection=" << witness::source_selection_text(request.selection);
      parser_trace::note("template.resolve", std::string(), trace.str());
    }
    emit_nested_class_use_source_events_from_location(scope,
                                                      request.location,
                                                      witness::SourceUseOwnership::NestedDerived,
                                                      request.template_name);
  }

  void emit_out_of_class_owner_class_use_if_needed(
      Scope & scope,
      const QualifiedName & qualified,
      const string & qualified_name,
      const CppAstNode * anchor_node = nullptr,
      ClassInfo * owner_override = nullptr,
      const vector<TemplateParameterInfo> * canonical_parameters = nullptr)
      override
  {
    emit_out_of_class_owner_class_use_if_needed_impl(scope,
                                                     qualified_name,
                                                     &qualified,
                                                     anchor_node,
                                                     owner_override,
                                                     canonical_parameters);
  }

  void emit_out_of_class_owner_class_use_if_needed(
      Scope & scope,
      const string & qualified_name,
      const CppAstNode * anchor_node = nullptr,
      ClassInfo * owner_override = nullptr,
      const vector<TemplateParameterInfo> * canonical_parameters = nullptr)
      override
  {
    QualifiedName qualified;
    const QualifiedName * qualified_syntax = nullptr;
    if(parse_out_of_class_member_qualified_name(qualified_name, qualified)) {
      qualified_syntax = &qualified;
    }
    emit_out_of_class_owner_class_use_if_needed_impl(scope,
                                                     qualified_name,
                                                     qualified_syntax,
                                                     anchor_node,
                                                     owner_override,
                                                     canonical_parameters);
  }

  string describe_out_of_class_special_member_binding_lookup(
      Scope & scope,
      const QualifiedName & qualified,
      const vector<pair<string, TypePtr> > & params) override
  {
    ostringstream out;
    out << " [special-member-binding class " << qualified_owner_name_text(qualified) << "]";
    out << " [member " << qualified.name << "]";
    out << " [params";
    if(params.empty()) {
      out << " <none>";
    } else {
      for(size_t i = 0; i < params.size(); ++i) {
        out << (i == 0 ? " " : ", ")
            << (params[i].first.empty() ? string("<anon>") : params[i].first)
            << ":" << describe_type(params[i].second);
      }
    }
    out << "]";

    ClassInfo * info = resolve_out_of_class_owner_class(scope, qualified);
    if(!info) {
      out << " [owner-class missing]";
      return out.str();
    }
    out << " [method-keys";
    for(map<string, vector<FunctionBinding *> >::const_iterator it = info->methods.begin();
        it != info->methods.end(); ++it) {
      out << (it == info->methods.begin() ? " " : ", ") << it->first;
    }
    out << "]";
    const vector<FunctionBinding *> & candidates = info->methods[qualified.name];
    out << " [candidate-count " << candidates.size() << "]";
    for(size_t i = 0; i < candidates.size(); ++i) {
      FunctionBinding * binding = candidates[i];
      out << " [candidate " << i << " type=" << describe_type(binding->type) << "]";
    }
    return out.str();
  }

  string describe_out_of_class_special_member_binding_lookup(
      Scope & scope,
      const string & qualified_name,
      const vector<pair<string, TypePtr> > & params) override
  {
    QualifiedName qualified;
    if(!parse_out_of_class_member_qualified_name(qualified_name, qualified)) {
      return " [special-member-binding lookup invalid qualified name]";
    }
    return describe_out_of_class_special_member_binding_lookup(scope, qualified, params);
  }

  bool out_of_class_special_member_template_param_types_match(
      const TypePtr & lhs,
      const vector<TemplateParameterInfo> & lhs_parameters,
      const TypePtr & rhs,
      const vector<TemplateParameterInfo> & rhs_parameters) const override
  {
    auto canonicalize_parameter_names =
        [&](const vector<TemplateParameterInfo> & parameters,
            const string & text) -> string {
      if(text.empty()) {
        return text;
      }
      string out = text;
      bool changed = false;
      for(size_t i = 0; i < parameters.size(); ++i) {
        if(parameters[i].name.empty()) {
          continue;
        }
        const string replacement = string("__cppgm_tparam_") + to_string(i);
        out = replace_elaborated_identifier_token_text(out,
                                                       parameters[i].name,
                                                       replacement,
                                                       changed);
        out = replace_identifier_token_text(out,
                                            parameters[i].name,
                                            replacement,
                                            changed);
      }
      return changed ? out : text;
    };

    auto template_parameter_index_for_type =
        [&](const vector<TemplateParameterInfo> & parameters,
            const TypePtr & type,
            size_t & index) -> bool {
      TypePtr base = strip_top_level_cv(type);
      if(!base || base->kind != Type::TK_NAMED) {
        return false;
      }
      const TemplateParameterInfo * parameter =
          find_template_parameter(parameters, base->named_key);
      if(!parameter && base->named_display != base->named_key) {
        parameter = find_template_parameter(parameters, base->named_display);
      }
      if(!parameter) {
        return false;
      }
      index = static_cast<size_t>(parameter - &parameters[0]);
      return true;
    };
    auto dependent_member_alias_target =
        [&](const TypePtr & type) -> TypePtr
    {
      TypePtr base = strip_top_level_cv(type);
      if(!base || base->kind != Type::TK_NAMED) {
        return TypePtr();
      }
      TypePtr owner;
      vector<string> members;
      vector<TemplateIdSyntax> member_template_ids;
      bool leading_typename = false;
      if(!named_type_dependent_qualified_member(base,
                                                owner,
                                                members,
                                                leading_typename,
                                                &member_template_ids) ||
         members.size() != 1 ||
         !member_template_ids.empty()) {
        return TypePtr();
      }
      (void)leading_typename;
      ClassInfo * owner_info = class_info_for_type(owner);
      if(!owner_info || !owner_info->member_scope) {
        return TypePtr();
      }
      map<string, TypePtr>::const_iterator found =
          owner_info->member_scope->named_types.find(members[0]);
      if(found != owner_info->member_scope->named_types.end()) {
        return found->second;
      }

      return lookup_enclosing_type_before_reference_placeholder(
          *owner_info->member_scope,
          members[0]);
    };

    if(!lhs || !rhs) {
      return lhs == rhs;
    }
    if(type_equals(lhs, rhs)) {
      return true;
    }

    TypePtr lhs_alias_target = dependent_member_alias_target(lhs);
    if(lhs_alias_target &&
       !type_equals(lhs_alias_target, lhs) &&
       out_of_class_special_member_template_param_types_match(
           lhs_alias_target, lhs_parameters, rhs, rhs_parameters)) {
      return true;
    }
    TypePtr rhs_alias_target = dependent_member_alias_target(rhs);
    if(rhs_alias_target &&
       !type_equals(rhs_alias_target, rhs) &&
       out_of_class_special_member_template_param_types_match(
           lhs, lhs_parameters, rhs_alias_target, rhs_parameters)) {
      return true;
    }

    size_t lhs_parameter_index = 0;
    size_t rhs_parameter_index = 0;
    const bool lhs_is_parameter =
        template_parameter_index_for_type(lhs_parameters, lhs, lhs_parameter_index);
    const bool rhs_is_parameter =
        template_parameter_index_for_type(rhs_parameters, rhs, rhs_parameter_index);
    if(lhs_is_parameter || rhs_is_parameter) {
      if(!lhs_is_parameter || !rhs_is_parameter ||
         lhs_parameter_index != rhs_parameter_index) {
        return false;
      }
      const TemplateParameterInfo & lhs_parameter =
          lhs_parameters[lhs_parameter_index];
      const TemplateParameterInfo & rhs_parameter =
          rhs_parameters[rhs_parameter_index];
      return lhs_parameter.kind == rhs_parameter.kind &&
             lhs_parameter.parameter_pack == rhs_parameter.parameter_pack &&
             lhs_parameter.template_parameter_count ==
                 rhs_parameter.template_parameter_count;
    }

    if(lhs->kind != rhs->kind) {
      return false;
    }

    switch(lhs->kind) {
    case Type::TK_FUNDAMENTAL:
      return lhs->fundamental == rhs->fundamental;

    case Type::TK_NAMED:
    {
      if(lhs->named_key == rhs->named_key &&
         lhs->named_display == rhs->named_display) {
        return true;
      }

      const string lhs_key =
          canonicalize_parameter_names(lhs_parameters, lhs->named_key);
      const string rhs_key =
          canonicalize_parameter_names(rhs_parameters, rhs->named_key);
      const string lhs_display =
          canonicalize_parameter_names(lhs_parameters, lhs->named_display);
      const string rhs_display =
          canonicalize_parameter_names(rhs_parameters, rhs->named_display);
      if(lhs_key == rhs_key && lhs_display == rhs_display) {
        return true;
      }

      const string lhs_texts[] = {lhs_key, lhs_display};
      const string rhs_texts[] = {rhs_key, rhs_display};
      for(size_t i = 0; i < sizeof(lhs_texts) / sizeof(lhs_texts[0]); ++i) {
        const string lhs_text = trim_space(strip_elaborated_type_prefix(lhs_texts[i]));
        const string lhs_unqualified = unqualified_member_name(lhs_text);
        for(size_t j = 0; j < sizeof(rhs_texts) / sizeof(rhs_texts[0]); ++j) {
          const string rhs_text = trim_space(strip_elaborated_type_prefix(rhs_texts[j]));
          const string rhs_unqualified = unqualified_member_name(rhs_text);
          if(normalized_type_lookup_text_matches(lhs_text, rhs_text) ||
             normalized_type_lookup_text_matches(lhs_unqualified, rhs_text) ||
             normalized_type_lookup_text_matches(lhs_text, rhs_unqualified) ||
             normalized_type_lookup_text_matches(lhs_unqualified, rhs_unqualified)) {
            return true;
          }
        }
      }
      return false;
    }

    case Type::TK_CV:
      return lhs->cv_const == rhs->cv_const &&
             lhs->cv_volatile == rhs->cv_volatile &&
             out_of_class_special_member_template_param_types_match(
                 lhs->inner, lhs_parameters, rhs->inner, rhs_parameters);

    case Type::TK_ATOMIC:
    case Type::TK_POINTER:
    case Type::TK_BLOCK_POINTER:
    case Type::TK_LVALUE_REFERENCE:
    case Type::TK_RVALUE_REFERENCE:
      return out_of_class_special_member_template_param_types_match(
          lhs->inner, lhs_parameters, rhs->inner, rhs_parameters);

    case Type::TK_MEMBER_POINTER:
      return out_of_class_special_member_template_param_types_match(
                 lhs->owner, lhs_parameters, rhs->owner, rhs_parameters) &&
             out_of_class_special_member_template_param_types_match(
                 lhs->inner, lhs_parameters, rhs->inner, rhs_parameters);

    case Type::TK_ARRAY:
      return lhs->has_bound == rhs->has_bound &&
             lhs->bound == rhs->bound &&
             lhs->bound_text == rhs->bound_text &&
             out_of_class_special_member_template_param_types_match(
                 lhs->inner, lhs_parameters, rhs->inner, rhs_parameters);

    case Type::TK_FUNCTION:
      if(lhs->variadic != rhs->variadic ||
         lhs->prototype_relaxed != rhs->prototype_relaxed ||
         lhs->function_const != rhs->function_const ||
         lhs->function_volatile != rhs->function_volatile ||
         lhs->params.size() != rhs->params.size() ||
         !out_of_class_special_member_template_param_types_match(
             lhs->inner, lhs_parameters, rhs->inner, rhs_parameters)) {
        return false;
      }
      for(size_t i = 0; i < lhs->params.size(); ++i) {
        if(!out_of_class_special_member_template_param_types_match(
               lhs->params[i], lhs_parameters, rhs->params[i], rhs_parameters)) {
          return false;
        }
      }
      return true;
    }

    return false;
  }

  bool out_of_class_special_member_template_parameters_match(
      Scope & lhs_scope,
      const vector<TemplateParameterInfo> & lhs,
      Scope & rhs_scope,
      const vector<TemplateParameterInfo> & rhs) const override
  {
    if(lhs.size() != rhs.size()) {
      return false;
    }
    for(size_t i = 0; i < lhs.size(); ++i) {
      if(lhs[i].kind != rhs[i].kind ||
         lhs[i].parameter_pack != rhs[i].parameter_pack ||
         lhs[i].template_parameter_count != rhs[i].template_parameter_count) {
        return false;
      }
      if(lhs[i].kind != TemplateParameterInfo::TP_NON_TYPE) {
        if(!template_parameter_redeclarations_compatible(lhs, rhs, i)) {
          return false;
        }
        continue;
      }

      if(lhs[i].value_type &&
         rhs[i].value_type &&
         !out_of_class_special_member_template_param_types_match(
             lhs[i].value_type,
             lhs,
             rhs[i].value_type,
             rhs)) {
        return false;
      }

      if(!(lhs[i].value_type && rhs[i].value_type)) {
        if(!template_parameter_redeclarations_compatible(lhs, rhs, i)) {
          return false;
        }
        if(parser_trace::enabled("template.resolve")) {
          ostringstream trace;
          trace << "special-member-template-param-fallback-text index=" << i;
          parser_trace::note("template.resolve", std::string(), trace.str());
        }
      }
    }
    (void)lhs_scope;
    (void)rhs_scope;
    return true;
  }

  FunctionTemplateDecl * resolve_out_of_class_special_member_template(
      Scope & scope,
      const QualifiedName & qualified,
      const vector<TemplateParameterInfo> & template_parameters,
      const vector<pair<string, TypePtr> > & params) override
  {
    const string qualified_name = qualified_name_syntax_text(qualified);
    ClassInfo * info = resolve_out_of_class_owner_class(scope, qualified);
    if(!info) {
      return nullptr;
    }
    vector<FunctionTemplateDecl *> templates =
        lookup_direct_function_templates(*info->member_scope, qualified.name);
    for(size_t i = 0; i < templates.size(); ++i) {
      FunctionTemplateDecl * decl = templates[i];
      Scope & decl_scope = decl->pattern_scope ? *decl->pattern_scope : scope;
      const auto matches_signature =
          [&](const vector<TemplateParameterInfo> & candidate_parameters,
              const vector<TemplateParameterInfo> & lhs_type_parameters,
              const vector<TemplateParameterInfo> & rhs_type_parameters) -> bool
          {
            const bool parameters_match =
                out_of_class_special_member_template_parameters_match(decl_scope,
                                                                     decl->parameters,
                                                                     scope,
                                                                     candidate_parameters);
            if(!parameters_match ||
               decl->params_pattern.size() != params.size()) {
              if(parser_trace::enabled("template.resolve")) {
                ostringstream trace;
                trace << "special-member-template-candidate"
                      << " name=" << qualified_name
                      << " candidate-params=" << decl->parameters.size()
                      << " incoming-params=" << candidate_parameters.size()
                      << " param-list-match=" << (parameters_match ? "yes" : "no")
                      << " candidate-param-count=" << decl->params_pattern.size()
                      << " incoming-param-count=" << params.size();
                parser_trace::note("template.resolve", std::string(), trace.str());
              }
              return false;
            }
            for(size_t j = 0; j < params.size(); ++j) {
              const bool type_match =
                  out_of_class_special_member_template_param_types_match(
                      decl->params_pattern[j].second,
                      lhs_type_parameters,
                      params[j].second,
                      rhs_type_parameters);
              if(!type_match) {
                if(parser_trace::enabled("template.resolve")) {
                  ostringstream trace;
                  trace << "special-member-template-candidate"
                        << " name=" << qualified_name
                        << " candidate-params=" << decl->parameters.size()
                        << " incoming-params=" << candidate_parameters.size()
                        << " param-index=" << j
                        << " type-match=no"
                        << " candidate-type="
                        << describe_type(decl->params_pattern[j].second)
                        << " incoming-type=" << describe_type(params[j].second);
                  parser_trace::note("template.resolve", std::string(), trace.str());
                }
                return false;
              }
            }
            if(parser_trace::enabled("template.resolve")) {
              ostringstream trace;
              trace << "special-member-template-candidate"
                    << " name=" << qualified_name
                    << " candidate-params=" << decl->parameters.size()
                    << " incoming-params=" << candidate_parameters.size()
                    << " match=yes";
              parser_trace::note("template.resolve", std::string(), trace.str());
            }
            return true;
          };
      if(matches_signature(template_parameters, decl->parameters, template_parameters)) {
        return decl;
      }
      if(template_parameters.size() > decl->parameters.size()) {
        const size_t suffix_start = template_parameters.size() - decl->parameters.size();
        vector<TemplateParameterInfo> suffix_parameters(template_parameters.begin() + suffix_start,
                                                        template_parameters.end());
        vector<TemplateParameterInfo> lhs_type_parameters(template_parameters.begin(),
                                                          template_parameters.begin() +
                                                              suffix_start);
        lhs_type_parameters.insert(lhs_type_parameters.end(),
                                   decl->parameters.begin(),
                                   decl->parameters.end());
        if(matches_signature(suffix_parameters, lhs_type_parameters, template_parameters)) {
          return decl;
        }
      }
    }

    return nullptr;
  }

  FunctionTemplateDecl * resolve_out_of_class_special_member_template(
      Scope & scope,
      const string & qualified_name,
      const vector<TemplateParameterInfo> & template_parameters,
      const vector<pair<string, TypePtr> > & params) override
  {
    QualifiedName qualified;
    if(!parse_out_of_class_member_qualified_name(qualified_name, qualified)) {
      return nullptr;
    }
    return resolve_out_of_class_special_member_template(scope,
                                                        qualified,
                                                        template_parameters,
                                                        params);
  }

  FunctionTemplateDecl * resolve_out_of_class_method_template(
      Scope & scope,
      const QualifiedName & qualified,
      const vector<TemplateParameterInfo> & template_parameters,
      const vector<pair<string, TypePtr> > & params,
      bool is_const_method,
      bool is_volatile_method,
      RefQualifier ref_qualifier) override
  {
    ClassInfo * info = resolve_out_of_class_owner_class(scope, qualified);
    if(!info) {
      return nullptr;
    }
    if(info->member_scope && !info->reference_member_collection_in_progress) {
      ensure_class_reference_members(*info);
    }
    vector<FunctionTemplateDecl *> templates =
        lookup_direct_function_templates(*info->member_scope, qualified.name);
    for(size_t i = 0; i < templates.size(); ++i) {
      FunctionTemplateDecl * decl = templates[i];
      if(decl->is_const_method != is_const_method ||
         decl->is_volatile_method != is_volatile_method ||
         decl->ref_qualifier != ref_qualifier) {
        continue;
      }
      Scope & decl_scope = decl->pattern_scope ? *decl->pattern_scope : scope;
      const auto matches_signature =
          [&](const vector<TemplateParameterInfo> & candidate_parameters,
              const vector<TemplateParameterInfo> & lhs_type_parameters,
              const vector<TemplateParameterInfo> & rhs_type_parameters) -> bool
          {
            if(!out_of_class_special_member_template_parameters_match(decl_scope,
                                                                     decl->parameters,
                                                                     scope,
                                                                     candidate_parameters) ||
               decl->params_pattern.size() != params.size()) {
              return false;
            }
            for(size_t j = 0; j < params.size(); ++j) {
              if(!out_of_class_special_member_template_param_types_match(
                     decl->params_pattern[j].second,
                     lhs_type_parameters,
                     params[j].second,
                     rhs_type_parameters)) {
                return false;
              }
            }
            return true;
          };
      if(matches_signature(template_parameters, decl->parameters, template_parameters)) {
        return decl;
      }
      if(template_parameters.size() > decl->parameters.size()) {
        const size_t suffix_start = template_parameters.size() - decl->parameters.size();
        vector<TemplateParameterInfo> suffix_parameters(template_parameters.begin() + suffix_start,
                                                        template_parameters.end());
        vector<TemplateParameterInfo> lhs_type_parameters(template_parameters.begin(),
                                                          template_parameters.begin() +
                                                              suffix_start);
        lhs_type_parameters.insert(lhs_type_parameters.end(),
                                   decl->parameters.begin(),
                                   decl->parameters.end());
        if(matches_signature(suffix_parameters, lhs_type_parameters, template_parameters)) {
          return decl;
        }
      }
    }

    return nullptr;
  }

  FunctionTemplateDecl * resolve_out_of_class_method_template(
      Scope & scope,
      const string & qualified_name,
      const string & member_name,
      const vector<TemplateParameterInfo> & template_parameters,
      const vector<pair<string, TypePtr> > & params,
      bool is_const_method,
      bool is_volatile_method,
      RefQualifier ref_qualifier) override
  {
    QualifiedName qualified;
    if(!parse_out_of_class_member_qualified_name(qualified_name, qualified)) {
      return nullptr;
    }
    if(!member_name.empty() && member_name != qualified.name) {
      qualified.name = member_name;
    }
    return resolve_out_of_class_method_template(scope,
                                                qualified,
                                                template_parameters,
                                                params,
                                                is_const_method,
                                                is_volatile_method,
                                                ref_qualifier);
  }

  string describe_out_of_class_special_member_template_lookup(
      Scope & scope,
      const QualifiedName & qualified,
      const vector<TemplateParameterInfo> & template_parameters,
      const vector<pair<string, TypePtr> > & params) override
  {
    ostringstream out;
    out << " [special-member-template class " << qualified_owner_name_text(qualified) << "]";
    out << " [member " << qualified.name << "]";
    out << " [template-params " << template_parameters.size() << "]";
    out << " [params";
    if(params.empty()) {
      out << " <none>";
    } else {
      for(size_t i = 0; i < params.size(); ++i) {
        out << (i == 0 ? " " : ", ")
            << (params[i].first.empty() ? string("<anon>") : params[i].first)
            << ":" << describe_type(params[i].second);
      }
    }
    out << "]";

    ClassInfo * info = resolve_out_of_class_owner_class(scope, qualified);
    if(!info) {
      out << " [owner-class missing]";
      return out.str();
    }
    vector<FunctionTemplateDecl *> templates =
        lookup_direct_function_templates(*info->member_scope, qualified.name);
    out << " [candidate-count " << templates.size() << "]";
    for(size_t i = 0; i < templates.size(); ++i) {
      out << " [candidate " << i
          << " template-params=" << templates[i]->parameters.size()
          << " params=" << templates[i]->params_pattern.size();
      if(!templates[i]->params_pattern.empty()) {
        out << " types=";
        for(size_t j = 0; j < templates[i]->params_pattern.size(); ++j) {
          if(j != 0) {
            out << ";";
          }
          out << describe_type(templates[i]->params_pattern[j].second);
        }
      }
      out << "]";
    }

    return out.str();
  }

  string describe_out_of_class_special_member_template_lookup(
      Scope & scope,
      const string & qualified_name,
      const vector<TemplateParameterInfo> & template_parameters,
      const vector<pair<string, TypePtr> > & params) override
  {
    QualifiedName qualified;
    if(!parse_out_of_class_member_qualified_name(qualified_name, qualified)) {
      return " [special-member-template lookup invalid qualified name]";
    }
    return describe_out_of_class_special_member_template_lookup(scope,
                                                                qualified,
                                                                template_parameters,
                                                                params);
  }

  string describe_template_parameter_infos(
      const vector<TemplateParameterInfo> & parameters) const override
  {
    ostringstream out;
    out << "[";
    for(size_t i = 0; i < parameters.size(); ++i) {
      if(i != 0) {
        out << "; ";
      }
      const TemplateParameterInfo & parameter = parameters[i];
      out << i << ":";
      switch(parameter.kind) {
      case TemplateParameterInfo::TP_TYPE:
        out << "type";
        break;
      case TemplateParameterInfo::TP_NON_TYPE:
        out << "value";
        break;
      case TemplateParameterInfo::TP_TEMPLATE_TEMPLATE:
        out << "template";
        break;
      }
      if(parameter.parameter_pack) {
        out << "...";
      }
      if(!parameter.name.empty()) {
        out << " " << parameter.name;
      }
      if(parameter.kind == TemplateParameterInfo::TP_NON_TYPE) {
        if(!parameter.non_type_decl_specifier_text.empty()) {
          out << " spec=" << parameter.non_type_decl_specifier_text;
        }
        if(parameter.value_type) {
          out << " type=" << describe_type(parameter.value_type);
        }
      } else if(parameter.kind == TemplateParameterInfo::TP_TEMPLATE_TEMPLATE) {
        out << " inner=" << parameter.template_parameter_count;
      }
    }
    out << "]";
    return out.str();
  }


  void collect_simple_declaration(Scope & scope,
                                  const CppAstNode & node,
                                  bool is_c_linkage,
                                  bool linkage_has_braces = false)
  {
    DIAG_CONTEXT("collect_simple_declaration [" + spaced_node_text(node) + "]" +
                 source_location_for_node(node));
    const CppAstNode * specifiers = find_child_kind(node, CppAstKind::decl_specifier_seq);
    const CppAstNode * declarators = find_child_kind(node, CppAstKind::init_declarator_list);
    if(!specifiers) {
      throw logic_error("simple-declaration missing decl-specifier-seq");
    }

    CppAstNode synthetic_decl;
    std::string synthetic_type_name;
    std::string synthetic_storage_name;
    if(semantic_class_model::synthesize_anonymous_union_storage_declaration(node,
                                                                            synthetic_decl,
                                                                            synthetic_type_name,
                                                                            synthetic_storage_name)) {
      const CppAstNode * owned_synthetic_decl = own_synthetic_ast(std::move(synthetic_decl));
      collect_simple_declaration(scope, *owned_synthetic_decl, is_c_linkage, linkage_has_braces);
      TypePtr storage_type = lookup_type(scope, synthetic_type_name);
      ClassInfo * storage_info = storage_type ? class_info_for_type(storage_type) : nullptr;
      if(!storage_info) {
        throw logic_error("missing anonymous union storage class");
      }
      semantic_class_model::inject_anonymous_union_variable_bindings(scope,
                                                                     *storage_info,
                                                                     synthetic_storage_name);
      return;
    }

    PreparedDeclarationSpecifiers prepared_specifiers;
    if(!prepare_namespace_scope_declaration_specifiers(scope,
                                                       *specifiers,
                                                       declarators,
                                                       true,
                                                       true,
                                                       prepared_specifiers)) {
      throw logic_error("unsupported namespace-scope embedded type-specifier");
    }

    if(!prepared_specifiers.has_auto && !prepared_specifiers.parsed_decl_spec) {
      throw logic_error(string("unsupported namespace-scope decl-specifier-seq: ") +
                        spaced_node_text(node));
    }

    if(!declarators) {
      return;
    }

    for(size_t i = 0; i < declarators->children.size(); ++i) {
      const CppAstNode & init_decl = declarators->children[i];
      if(init_decl.kind != CppAstKind::init_declarator || init_decl.children.empty()) {
        throw logic_error("unsupported top-level init-declarator");
      }

      string name;
      TypePtr type;
      const CppAstNode * initializer =
          init_decl.children.size() > 1 ? &init_decl.children[1] : nullptr;
      bool is_typedef = prepared_specifiers.declaration_is_typedef;
      CppAstNode synthesized_initializer;
      string recover_namespace_init_error;
      Scope * parse_scope =
          semantic_lookup::resolve_qualified_variable_parse_scope(*this,
                                                                  scope,
                                                                  init_decl.children[0]);
      auto try_recover_function_style_namespace_initializer =
          [&](const CppAstNode & declarator_node,
              string & recovered_name,
              TypePtr & recovered_type,
              bool & recovered_typedef,
              CppAstNode & recovered_initializer) -> bool
      {
        recover_namespace_init_error.clear();
        if(initializer || recovered_typedef) {
          recover_namespace_init_error = "has explicit initializer or typedef";
          return false;
        }

        const CppAstNode * parameter_clause =
            find_child_kind(declarator_node, CppAstKind::parameter_clause);
        if(!parameter_clause || parameter_clause->children.empty()) {
          recover_namespace_init_error = "no parameter-clause children";
          return false;
        }

        CppAstNode stripped_declarator = declarator_node;
        vector<CppAstNode> kept;
        for(size_t j = 0; j < stripped_declarator.children.size(); ++j) {
          if(stripped_declarator.children[j].kind != CppAstKind::parameter_clause) {
            kept.push_back(stripped_declarator.children[j]);
          }
        }
        stripped_declarator.children.swap(kept);

        string local_name;
        TypePtr local_type;
        bool local_typedef = prepared_specifiers.declaration_is_typedef;
        if(!parse_variable_declaration_type(*parse_scope,
                                           prepared_specifiers.resolved_specifiers,
                                           stripped_declarator,
                                           nullptr,
                                           true,
                                           local_name,
                                           local_type,
                                           local_typedef)) {
          recover_namespace_init_error = "failed stripped declarator parse";
          return false;
        }
        if(local_typedef) {
          recover_namespace_init_error = "recovered declarator is typedef";
          return false;
        }
        if(!local_type) {
          recover_namespace_init_error = "null recovered namespace type";
          return false;
        }
        if(strip_top_level_cv(local_type)->kind == Type::TK_FUNCTION) {
          recover_namespace_init_error = "recovered type still function";
          return false;
        }

        if(!semantic_parameter_recovery::recover_parameter_clause_initializer(
               *parameter_clause,
               recovered_initializer,
               recover_namespace_init_error)) {
          return false;
        }
        recovered_name = local_name;
        recovered_type = local_type;
        recovered_typedef = false;
        return true;
      };

      bool parsed_as_variable =
          false;
      const bool reference_function_parameter_types_only =
          find_child_kind(init_decl.children[0], CppAstKind::parameter_clause) != nullptr;
      const CppAstNode * declaration_identifier =
          find_descendant_kind(init_decl.children[0], CppAstKind::identifier);
      const QualifiedName * declaration_name_syntax =
          declaration_identifier ? cppast_qualified_name_syntax(*declaration_identifier) : nullptr;
      parsed_as_variable =
          parse_variable_declaration_type(*parse_scope,
                                         prepared_specifiers.resolved_specifiers,
                                         init_decl.children[0],
                                         initializer,
                                         true,
                                         name,
                                         type,
                                         is_typedef,
                                         reference_function_parameter_types_only);
      if(!parsed_as_variable &&
         find_child_kind(init_decl.children[0], CppAstKind::parameter_clause)) {
        bool function_typedef = prepared_specifiers.declaration_is_typedef;
        TypePtr function_base;
        if(parse_trailing_return_base(*parse_scope,
                                     *specifiers,
                                     init_decl.children[0],
                                     function_typedef,
                                     function_base,
                                     reference_function_parameter_types_only)) {
          const CppAstNode filtered_declarator =
              filtered_function_declarator(init_decl.children[0]);
          TypePtr function_type;
          string function_name;
          if(parse_declarator(*parse_scope,
                              filtered_declarator,
                              function_base,
                              function_name,
                              function_type,
                              reference_function_parameter_types_only) &&
             function_type &&
             strip_top_level_cv(function_type)->kind == Type::TK_FUNCTION) {
            name = function_name;
            type = function_type;
            is_typedef = function_typedef;
            parsed_as_variable = true;
          }
        }
      }
      if(!parsed_as_variable &&
         !try_recover_function_style_namespace_initializer(init_decl.children[0],
                                                           name,
                                                           type,
                                                           is_typedef,
                                                           synthesized_initializer)) {
        throw logic_error("unsupported namespace-scope declarator");
      }
      if(!initializer && !synthesized_initializer.children.empty()) {
        initializer = &synthesized_initializer;
      }

      Scope * declaration_scope = &scope;
      string declaration_name = name;
      QualifiedName declaration_target;
      if(declaration_name_syntax) {
        declaration_target = *declaration_name_syntax;
      } else {
        declaration_target.name = name;
      }
      if(!is_typedef &&
         !semantic_lookup::resolve_qualified_namespace_entity_target(*this,
                                                                    scope,
                                                                    declaration_target,
                                                                    declaration_scope,
                                                                    declaration_name)) {
        throw logic_error("missing qualified declaration target");
      }

      if(is_typedef) {
        scope.named_types[name] = type;
      } else if(type && strip_top_level_cv(type)->kind == Type::TK_FUNCTION) {
        vector<pair<string, TypePtr> > params;
        vector<const CppAstNode *> default_args;
        const CppAstNode * parameter_clause =
            find_child_kind(init_decl.children[0], CppAstKind::parameter_clause);
        if(parameter_clause &&
           !parse_parameter_clause(scope,
                                   *parameter_clause,
                                   params,
                                   &default_args,
                                   reference_function_parameter_types_only)) {
          throw logic_error("unsupported namespace-scope parameter-clause");
        }
        register_function(*declaration_scope,
                          declaration_name,
                          type,
                          params,
                          default_args,
                          nullptr,
                          &node,
                          &init_decl.children[0],
                          is_c_linkage,
                          declaration_scope,
                          declarator_function_qualifier(init_decl.children[0]),
                          decl_spec_contains_token(prepared_specifiers.resolved_specifiers,
                                                   KW_CONSTEXPR));
      } else {
        const bool is_thread_local_variable =
            decl_spec_contains_token(prepared_specifiers.resolved_specifiers,
                                     KW_THREAD_LOCAL);
        const bool is_definition =
            variable_declaration_is_definition(prepared_specifiers.resolved_specifiers,
                                               initializer,
                                               type,
                                               is_c_linkage,
                                               linkage_has_braces);
        ValueBinding * out_of_class_static_member = nullptr;
        if(resolve_out_of_class_static_member_binding(scope,
                                                      name,
                                                      out_of_class_static_member)) {
          emit_out_of_class_owner_class_use_if_needed(scope,
                                                      name,
                                                      &init_decl.children[0]);
          TypePtr merged = merge_types(out_of_class_static_member->type, type);
          if(!merged) {
            throw logic_error(string("mismatched variable declaration") +
                              semantic_trace::current_location_note(*this, &init_decl) +
                              semantic_trace::previous_value_location_note(
                                  *this, "previous declaration", out_of_class_static_member));
          }
          out_of_class_static_member->type = merged;
          if(out_of_class_static_member->is_thread_local != is_thread_local_variable) {
            throw logic_error(string("mismatched variable thread_local storage") +
                              semantic_trace::current_location_note(*this, &init_decl) +
                              semantic_trace::previous_value_location_note(
                                  *this, "previous declaration", out_of_class_static_member));
          }
          if(!out_of_class_static_member->declaration_node) {
            out_of_class_static_member->declaration_node = &node;
          }
          if(is_definition) {
            if(out_of_class_static_member->has_storage_definition) {
              throw logic_error(string("duplicate class static member definition") +
                                semantic_trace::current_location_note(*this, &init_decl) +
                                semantic_trace::previous_value_location_note(
                                    *this, "previous definition", out_of_class_static_member));
            }
            out_of_class_static_member->definition_node = &node;
            out_of_class_static_member->has_storage_definition = true;
          }

          const bool is_constexpr_variable =
              decl_spec_contains_token(*specifiers, KW_CONSTEXPR);
          out_of_class_static_member->requires_constant_initializer = is_constexpr_variable;
          if(initializer &&
             (is_constexpr_variable || is_const_object_type(type) || is_reference_type(type))) {
            out_of_class_static_member->constant_initializer = initializer;
            out_of_class_static_member->constant_initializer_scope = parse_scope;
          }
          if(initializer &&
             (is_constexpr_variable || is_const_object_type(type))) {
            try_note_binding_initializer_constexpr_value(*out_of_class_static_member,
                                                         *parse_scope,
                                                         *initializer,
                                                         type);
          }
          if(is_constexpr_variable &&
             !out_of_class_static_member->has_constexpr_value &&
             !initializer) {
            try_note_binding_default_initialized_constexpr_value(*out_of_class_static_member,
                                                                 *parse_scope,
                                                                 type);
          }
          if(is_constexpr_variable && !out_of_class_static_member->has_constexpr_value) {
            throw logic_error("unsupported constexpr variable initializer" +
                              source_location_for_node(init_decl));
          }
          continue;
        }
        ValueBinding & binding = register_namespace_value(*declaration_scope,
                                                         declaration_name,
                                                         type,
                                                         is_c_linkage,
                                                         is_thread_local_variable,
                                                         is_definition,
                                                         &init_decl,
                                                         specifiers);
        const bool is_constexpr_variable =
            decl_spec_contains_token(*specifiers, KW_CONSTEXPR);
        binding.requires_constant_initializer = is_constexpr_variable;
        TypePtr value_base = strip_top_level_cv(remove_reference_type(type));
        if(initializer &&
           initializer->kind == CppAstKind::initializer &&
           (is_constexpr_variable ||
            is_const_object_type(type) ||
            (value_base && value_base->kind == Type::TK_ARRAY))) {
          binding.constant_initializer = initializer;
          binding.constant_initializer_scope = &scope;
        }
        if(initializer &&
           initializer->kind == CppAstKind::initializer &&
           (is_constexpr_variable ||
            is_const_object_type(type) ||
            is_reference_type(type))) {
          binding.constant_initializer = initializer;
          binding.constant_initializer_scope = declaration_scope;
        }
        if(initializer &&
           (is_constexpr_variable || is_const_object_type(type))) {
          try_note_binding_initializer_constexpr_value(binding,
                                                       *declaration_scope,
                                                       *initializer,
                                                       type);
        }
        if(is_constexpr_variable &&
           !binding.has_constexpr_value &&
           !initializer) {
          try_note_binding_default_initialized_constexpr_value(binding,
                                                               *declaration_scope,
                                                               type);
        }
        if(is_constexpr_variable && !binding.has_constexpr_value) {
          throw logic_error("unsupported constexpr variable initializer" +
                            source_location_for_node(init_decl));
        }
      }
    }
  }

  void collect_explicit_instantiation(Scope & scope,
                                      const CppAstNode & node)
  {
    DIAG_CONTEXT("collect_explicit_instantiation [" + spaced_node_text(node) + "]" +
                 source_location_for_node(node));
    const bool is_declaration =
        node.kind == CppAstKind::explicit_instantiation_declaration;
    const ScopedSuppressedTemplateUseLocation suppressed_use_location(is_declaration);
    if(node.children.empty()) {
      throw logic_error("explicit-instantiation missing target");
    }

    const CppAstNode & target = node.children[0];
    const auto normalize_witness_location =
        [](const std::string & location) -> std::string
    {
      const std::string without_at =
          location.size() > 1 && location[0] == '@' ? location.substr(1) :
                                                      location;
      return template_api::normalize_template_witness_source_location(without_at);
    };
    const std::string explicit_instantiation_location =
        normalize_witness_location(source_location_for_node(node));
    const auto class_template_decl_location =
        [&](ClassTemplateDecl * primary) -> std::string
    {
      if(!(primary && primary->class_node)) {
        return std::string();
      }
      std::string location =
          normalize_witness_location(
              source_location_for_name_in_node(*primary->class_node,
                                               primary->name));
      if(!location.empty()) {
        return location;
      }
      return normalize_witness_location(source_location_for_node(*primary->class_node));
    };
    const auto function_binding_decl_location =
        [&](FunctionBinding * binding) -> std::string
    {
      if(!binding) {
        return std::string();
      }
      if(binding->definition_node) {
        std::string location =
            normalize_witness_location(
                source_location_for_node(*binding->definition_node));
        if(!location.empty()) {
          return location;
        }
      }
      if(binding->declaration_node) {
        std::string location =
            normalize_witness_location(
                source_location_for_node(*binding->declaration_node));
        if(!location.empty()) {
          return location;
        }
      }
      return std::string();
    };
    const auto note_direct_explicit_class_finalization =
        [&](const std::string & entity,
            const std::string & decl_location,
            template_api::TemplateLifecycleCause cause) -> void
    {
      if(entity.empty() || template_witness_context().session == nullptr) {
        return;
      }
      const std::string event_decl_location =
          !decl_location.empty() ? decl_location : explicit_instantiation_location;
      const witness::ScopedTemplateWitnessEntryContext entry_context(
          witness::make_template_closure_entry_context(
              witness::TemplateClosureReason::FinalizeClass,
              entity,
              event_decl_location,
              true));
      witness::note_template_witness_log_event(
          witness::TemplateWitnessLogEventKind::ClassFinalization,
          explicit_instantiation_location,
          entity,
          event_decl_location,
          std::string(),
          cause,
          true);
    };
    const auto note_direct_explicit_function_instantiation =
        [&](FunctionBinding * binding,
            template_api::TemplateLifecycleCause cause) -> void
    {
      if(!binding || template_witness_context().session == nullptr) {
        return;
      }
      const std::string entity = semantic_model::function_output_name(*binding);
      if(entity.empty()) {
        return;
      }
      const std::string decl_location = function_binding_decl_location(binding);
      const std::string event_decl_location =
          !decl_location.empty() ? decl_location : explicit_instantiation_location;
      const witness::ScopedTemplateWitnessEntryContext entry_context(
          witness::make_template_closure_entry_context(
              witness::TemplateClosureReason::TrackInstantiation,
              entity,
              event_decl_location,
              template_api::function_or_owner_has_template_identity(binding)));
      witness::note_template_witness_log_event(
          witness::TemplateWitnessLogEventKind::FunctionInstantiation,
          explicit_instantiation_location,
          entity,
          event_decl_location,
          "created-new=no",
          cause,
          template_api::function_or_owner_has_template_identity(binding));
    };
    if(target.kind == CppAstKind::class_specifier ||
       target.kind == CppAstKind::class_forward_declaration) {
      if(parser_trace::enabled("template.resolve")) {
        parser_trace::note("template.resolve",
                           std::string(),
                           std::string("explicit-class-instantiation target=") +
                               target.value +
                               " declaration=" + (is_declaration ? "yes" : "no"));
      }
      const TemplateIdSyntax * target_template_id = cppast_template_id_syntax(target);
      if(!target_template_id) {
        if(parser_trace::enabled("template.resolve")) {
          parser_trace::note("template.resolve",
                             std::string(),
                             std::string("explicit-class-instantiation parse-failed target=") +
                                 target.value);
        }
        if(is_declaration) {
          return;
        }
        throw logic_error("explicit instantiation requires a class template-id");
      }
      QualifiedName specialization_name = target_template_id->name;
      vector<string> arg_texts = target_template_id->arguments;

      QualifiedName owner_scope_name;
      owner_scope_name.rooted = specialization_name.rooted;
      if(!specialization_name.qualifiers.empty()) {
        owner_scope_name.name = specialization_name.qualifiers.back();
        owner_scope_name.qualifiers.assign(specialization_name.qualifiers.begin(),
                                           specialization_name.qualifiers.end() - 1);
      }
      Scope * target_scope = &scope;
      if(specialization_name.rooted || !specialization_name.qualifiers.empty()) {
        target_scope =
            owner_scope_name.name.empty() && owner_scope_name.qualifiers.empty() ?
                root.get() :
                semantic_lookup::resolve_qualified_scope_for_class_or_namespace(
                    *this, scope, owner_scope_name);
      }
      ClassTemplateDecl * primary =
          target_scope ?
              semantic_lookup::lookup_class_template(*this,
                                                    *target_scope,
                                                    specialization_name.name) :
              nullptr;
      if(!primary) {
        if(parser_trace::enabled("template.resolve")) {
          parser_trace::note("template.resolve",
                             std::string(),
                             std::string("explicit-class-instantiation missing-template name=") +
                                 specialization_name.name);
        }
        if(is_declaration) {
          return;
        }
        throw logic_error("unknown class template explicit instantiation");
      }

      vector<TemplateArgument> arguments;
      if(!resolve_template_arguments(scope,
                                     primary->parameters,
                                     arg_texts,
                                     &target_template_id->argument_syntaxes,
                                     arguments,
                                     primary->declaring_scope)) {
        if(parser_trace::enabled("template.resolve")) {
          parser_trace::note("template.resolve",
                             std::string(),
                             std::string("explicit-class-instantiation arg-resolution-failed name=") +
                                 specialization_name.name);
        }
        if(is_declaration) {
          return;
        }
        throw logic_error("invalid class explicit instantiation arguments");
      }
      canonicalize_simple_dependent_argument_texts(arguments);
      const string specialization_key = template_args_identity_key(arguments);
      if(is_declaration) {
        primary->suppress_implicit_instantiation_definitions.insert(specialization_key);
        if(parser_trace::enabled("template.resolve")) {
          parser_trace::note("template.resolve",
                             std::string(),
                             std::string("explicit-class-instantiation suppress key=") +
                                 specialization_key +
                                 " name=" + specialization_name.name);
        }
        map<string, ClassInfo *>::iterator found =
            primary->instantiations.find(specialization_key);
        if(found != primary->instantiations.end()) {
          found->second->suppress_implicit_instantiation_definition = true;
        }
        found = primary->reference_instantiations.find(specialization_key);
        if(found != primary->reference_instantiations.end()) {
          found->second->suppress_implicit_instantiation_definition = true;
        }
        note_direct_explicit_class_finalization(
            primary->name,
            class_template_decl_location(primary),
            template_api::TemplateLifecycleCause::ExplicitInstantiationDeclaration);
        return;
      }

      primary->suppress_implicit_instantiation_definitions.erase(specialization_key);
      map<string, ClassInfo *>::iterator found =
          primary->instantiations.find(specialization_key);
      if(found != primary->instantiations.end() && found->second) {
        found->second->suppress_implicit_instantiation_definition = false;
      }
      found = primary->reference_instantiations.find(specialization_key);
      if(found != primary->reference_instantiations.end() && found->second) {
        found->second->suppress_implicit_instantiation_definition = false;
      }
      ClassInfo * explicit_info =
          reference_class_template_instantiation_with_syntax(
              *primary,
              scope,
              arg_texts,
              &target_template_id->argument_syntaxes);
      if(!explicit_info) {
        throw logic_error("failed explicit class template instantiation");
      }
      finalize_instantiated_class_materialization(*primary,
                                                  *explicit_info,
                                                  arguments);
      note_direct_explicit_class_finalization(
          primary->name,
          class_template_decl_location(primary),
          template_api::TemplateLifecycleCause::ExplicitInstantiationDefinition);
      for(size_t i = 0; i < explicit_info->method_declaration_order.size(); ++i) {
        FunctionBinding * binding = explicit_info->method_declaration_order[i];
        if(!binding || binding->is_deleted || binding->synthesized ||
           binding->is_defaulted || !binding->has_definition ||
           template_api::function_binding_excluded_from_explicit_instantiation(
               *binding)) {
          continue;
        }
        note_direct_explicit_function_instantiation(
            binding,
            template_api::TemplateLifecycleCause::ExplicitInstantiationDefinition);
      }
      return;
    }

    if(target.kind == CppAstKind::special_member_declaration) {
      if(is_declaration) {
        return;
      }
      throw logic_error(string("unsupported explicit instantiation target ") +
                        cppast_kind_text(target.kind));
    }

    if(target.kind != CppAstKind::simple_declaration) {
      throw logic_error(string("unsupported explicit instantiation target ") +
                        cppast_kind_text(target.kind));
    }

    const CppAstNode * specifiers = find_child_kind(target, CppAstKind::decl_specifier_seq);
    const CppAstNode * declarators =
        find_child_kind(target, CppAstKind::init_declarator_list);
    if(!specifiers || !declarators || declarators->children.size() != 1) {
      throw logic_error("unsupported explicit instantiation declaration form");
    }

    const CppAstNode & init_decl = declarators->children[0];
    if(init_decl.kind != CppAstKind::init_declarator || init_decl.children.empty()) {
      throw logic_error("unsupported explicit instantiation declarator");
    }
    if(init_decl.children.size() > 1) {
      throw logic_error("explicit instantiation cannot have an initializer");
    }

    CppAstNode resolved_specifiers;
    if(!prepare_namespace_scope_specifiers(scope,
                                           *specifiers,
                                           declarators,
                                           true,
                                           true,
                                           resolved_specifiers)) {
      throw logic_error("unsupported explicit instantiation specifiers");
    }

    bool is_typedef = false;
    string name;
    TypePtr type;
    const CppAstNode * instantiation_identifier =
        find_descendant_kind(init_decl.children[0], CppAstKind::identifier);
    const QualifiedName * instantiation_name_syntax =
        instantiation_identifier ?
            cppast_qualified_name_syntax(*instantiation_identifier) :
            nullptr;
    const TemplateIdSyntax * instantiation_template_id_syntax =
        instantiation_identifier ?
            cppast_template_id_syntax(*instantiation_identifier) :
            nullptr;
    Scope * parse_scope =
        semantic_lookup::resolve_qualified_variable_parse_scope(*this,
                                                                scope,
                                                                init_decl.children[0]);
    if(!parse_variable_declaration_type(*parse_scope,
                                        resolved_specifiers,
                                        init_decl.children[0],
                                        nullptr,
                                        false,
                                        name,
                                        type,
                                       is_typedef) ||
       is_typedef ||
       !type ||
       strip_top_level_cv(type)->kind != Type::TK_FUNCTION) {
      if(is_declaration) {
        return;
      }
      throw logic_error("unsupported explicit instantiation target");
    }

    QualifiedName specialization_name;
    vector<string> arg_texts;
    bool has_explicit_template_id = instantiation_template_id_syntax != nullptr;
    if(instantiation_template_id_syntax) {
      specialization_name = instantiation_template_id_syntax->name;
      arg_texts = instantiation_template_id_syntax->arguments;
    }

    vector<FunctionTemplateDecl *> templates =
        has_explicit_template_id ?
            ((specialization_name.rooted || !specialization_name.qualifiers.empty()) ?
                 lookup_function_templates(scope, specialization_name) :
                 lookup_function_templates(scope, specialization_name.name)) :
            ((instantiation_name_syntax &&
              (instantiation_name_syntax->rooted ||
               !instantiation_name_syntax->qualifiers.empty())) ?
                 lookup_function_templates(scope, *instantiation_name_syntax) :
                 lookup_function_templates(scope, name));
    if(templates.empty()) {
      if(is_declaration) {
        return;
      }
      throw logic_error("unknown function template explicit instantiation");
    }

    FunctionBinding * selected_binding = nullptr;
    std::size_t selected_parameter_count = 0;
    bool selected_initialized = false;
    for(size_t i = 0; i < templates.size(); ++i) {
      vector<TemplateArgument> arguments;
      std::map<std::string, std::size_t> pack_sizes;
      if(has_explicit_template_id) {
        if(!resolve_template_arguments(*parse_scope,
                                       templates[i]->parameters,
                                       arg_texts,
                                       &instantiation_template_id_syntax->argument_syntaxes,
                                       arguments,
                                       templates[i]->declaring_scope)) {
          continue;
        }
      } else {
        template_api::TemplateFunctionDeductionRequest request;
        request.decl = templates[i];
        request.target_type = type;
        request.use_scope = parse_scope;
        template_api::TemplateFunctionDeductionResult result;
        if(!template_api::deduce_function_template(*this, request, result)) {
          continue;
        }
        arguments.swap(result.arguments);
        pack_sizes.swap(result.pack_sizes);
      }

      FunctionBinding * binding = acquire_function_template(*templates[i],
                                                            arguments,
                                                            parse_scope,
                                                            pack_sizes.empty() ? nullptr :
                                                                                 &pack_sizes,
                                                            false)
                                     .function_binding;
      if(!binding) {
        continue;
      }
      if(!types_equivalent_for_member_binding(binding->type, type)) {
        if(parser_trace::enabled("template.resolve")) {
          parser_trace::note(
              "template.resolve",
              std::string(),
              std::string("explicit-function-instantiation skip-type-mismatch name=") +
                  binding->display_name);
        }
        continue;
      }
      const std::size_t parameter_count = templates[i]->parameters.size();
      if(!selected_initialized || parameter_count < selected_parameter_count) {
        selected_binding = binding;
        selected_parameter_count = parameter_count;
        selected_initialized = true;
      }
      if(has_explicit_template_id) {
        break;
      }
    }

    if(selected_binding) {
      const FunctionTemplateRegistrationIdentity selected_identity =
          template_api::function_binding_registration_identity(*selected_binding);
      if(is_declaration) {
        if(!selected_binding->has_definition) {
          selected_binding->suppress_implicit_instantiation_definition = true;
          if(parser_trace::enabled("template.resolve")) {
            parser_trace::note(
                "template.resolve",
                std::string(),
                std::string("explicit-function-instantiation suppress key=") +
                    selected_identity.key +
                    " name=" + selected_binding->display_name);
          }
        }
      } else {
        selected_binding->suppress_implicit_instantiation_definition = false;
        if(parser_trace::enabled("template.resolve")) {
          parser_trace::note(
              "template.resolve",
              std::string(),
              std::string("explicit-function-instantiation require key=") +
                  selected_identity.key +
                  " name=" + selected_binding->display_name);
        }
        note_direct_explicit_function_instantiation(
            selected_binding,
            template_api::TemplateLifecycleCause::ExplicitInstantiationDefinition);
        note_instantiated_function_output(selected_binding,
                                          InstantiatedFunctionOutputMode::RequireDefinition);
      }
      return;
    }

    if(is_declaration && !has_explicit_template_id) {
      return;
    }
    throw logic_error(has_explicit_template_id ?
                          "invalid function template explicit instantiation arguments" :
                          "invalid deduced function template explicit instantiation arguments");
  }

  void collect_function_definition(Scope & scope,
                                   const CppAstNode & node,
                                   bool is_c_linkage)
  {
    DIAG_CONTEXT("collect_function_definition [" + spaced_node_text(node) + "]" +
                 source_location_for_node(node));
    const CppAstNode * specifiers = find_child_kind(node, CppAstKind::decl_specifier_seq);
    const CppAstNode * declarator = find_child_kind(node, CppAstKind::declarator);
    const CppAstNode * body = find_function_body_node(node);
    const CppAstNode * ctor_initializer = find_child_kind(node, CppAstKind::ctor_initializer);
    if(!specifiers || !declarator || !body) {
      throw logic_error("function-definition missing children");
    }

    bool is_typedef = false;
    TypePtr base;
    Scope * parse_scope = resolve_qualified_function_parse_scope(scope, *declarator);
    const bool method_like_definition =
        parse_scope->class_info &&
        find_child_kind(*declarator, CppAstKind::parameter_clause) != nullptr;
    PreparedMethodParseContext prepared_method;
    semantic_class_model::prepare_method_parse_context(specifiers,
                                                       *declarator,
                                                       prepared_method,
                                                       method_like_definition,
                                                       true);
    if(!parse_function_definition_base(*parse_scope,
                                       *prepared_method.parse_specifiers_node(),
                                       prepared_method.parse_declarator_node(),
                                       *body,
                                       prepared_method.has_method_syntax &&
                                           prepared_method.syntax.is_const_method,
                                       prepared_method.has_method_syntax &&
                                           prepared_method.syntax.is_volatile_method,
                                       is_typedef,
                                       base) ||
       is_typedef) {
      ostringstream out;
      out << "unsupported function decl-specifier-seq";
      out << " [specifiers " << spaced_node_text(*specifiers) << "]";
      out << " [declarator " << spaced_node_text(*declarator) << "]";
      throw logic_error(out.str());
    }

    string name;
    TypePtr type;
    const CppAstNode filtered_declarator = prepared_method.has_method_syntax ?
        prepared_method.parse_declarator_node() : filtered_function_declarator(*declarator);
    if(!parse_declarator(*parse_scope, filtered_declarator, base, name, type) ||
       !type || strip_top_level_cv(type)->kind != Type::TK_FUNCTION) {
      const CppAstNode * debug_parameter_clause =
          find_child_kind(*declarator, CppAstKind::parameter_clause);
      vector<pair<string, TypePtr> > debug_params;
      const bool debug_parameter_clause_ok =
          !debug_parameter_clause ||
          parse_parameter_clause(*parse_scope, *debug_parameter_clause, debug_params);
      TypePtr debug_char_type = lookup_type(*parse_scope, "char_type");
      ostringstream out;
      out << "unsupported function declarator";
      out << " [specifiers " << spaced_node_text(*specifiers) << "]";
      out << " [declarator " << spaced_node_text(*declarator) << "]";
      out << " [filtered " << spaced_node_text(filtered_declarator) << "]";
      out << " [parse-scope " << scope_qualified_name(*parse_scope, "<here>") << "]";
      out << " [parse-bindings " << describe_scope_bindings_for_diagnostic(*parse_scope) << "]";
      out << " [parameter-clause-ok " << (debug_parameter_clause_ok ? "yes" : "no") << "]";
      out << " [char_type "
          << (debug_char_type ? describe_type(debug_char_type) : string("<missing>")) << "]";
      throw logic_error(out.str());
    }

    vector<pair<string, TypePtr> > params;
    vector<const CppAstNode *> default_args;
    const CppAstNode * parameter_clause = find_child_kind(*declarator, CppAstKind::parameter_clause);
    if(parameter_clause &&
       !parse_parameter_clause(*parse_scope, *parameter_clause, params, &default_args)) {
      throw logic_error("unsupported function parameter-clause");
    }

    if(parser_trace::enabled("class.collect")) {
      ostringstream trace;
      trace << "function-definition-binding"
            << " parsed-name=" << name
            << " parsed-type=" << describe_type(type)
            << " parse-scope=" << scope_qualified_name(*parse_scope, "<here>")
            << " declarator=" << spaced_node_text(*declarator);
      parser_trace::note("class.collect", std::string(), trace.str());
    }

    string out_of_class_lookup_name = name;
    const CppAstNode * function_identifier =
        find_descendant_kind(*declarator, CppAstKind::identifier);
    if(function_identifier &&
       function_identifier->value.find("::") != string::npos &&
       function_identifier->value.find("operator") != string::npos &&
       out_of_class_lookup_name.find("operator") == string::npos) {
      out_of_class_lookup_name = function_identifier->value;
    }

    FunctionBinding * method_binding = nullptr;
    if(resolve_out_of_class_method_binding(scope, out_of_class_lookup_name, type,
                                           declarator_is_const_method(*declarator),
                                           declarator_is_volatile_method(*declarator),
                                           semantic_class_model::declarator_ref_qualifier(*declarator),
                                           method_binding)) {
      if(!explicit_function_nothrow_specifications_match(
             *method_binding,
             declarator_function_qualifier(*declarator))) {
        throw logic_error(string("mismatched function exception specification [cs-function-def]") +
                          semantic_trace::current_location_note(*this, declarator) +
                          semantic_trace::previous_function_location_note(
                              *this, "previous declaration", method_binding));
      }
      if(method_binding->has_definition) {
        throw logic_error(string("duplicate class member definition") +
                          semantic_trace::current_location_note(*this, declarator) +
                          semantic_trace::previous_function_location_note(
                              *this, "previous definition", method_binding));
      }
      refresh_definition_parameter_names(*method_binding, params);
      method_binding->body = body;
      record_definition_parameter_aliases(*method_binding, params);
      method_binding->ctor_initializer = ctor_initializer;
      method_binding->has_definition = true;
      method_binding->definition_node = &node;
      method_binding->definition_abi_tags.clear();
      append_function_declaration_abi_tags(method_binding->definition_abi_tags, &node);
      method_binding->parameter_syntax_node = declarator;
      if(!method_binding->function_qualifier) {
        method_binding->function_qualifier = declarator_function_qualifier(*declarator);
      }
      upgrade_function_symbol_linkage(
          method_binding,
          function_symbol_linkage(*parse_scope,
                                  &node,
                                  body,
                                  is_c_linkage,
                                  method_binding->function_qualifier,
                                  function_template_registration_identity(*method_binding),
                                  false,
                                  method_binding->lexical_access_class));
      ClassInfo * owner_class = method_binding->owner_class;
      const std::vector<TemplateArgument> * origin_arguments =
          class_template_origin_arguments(owner_class);
      if(class_has_explicit_template_origin(owner_class) &&
         origin_arguments &&
         !template_arguments_are_dependent(*origin_arguments)) {
        if(specifiers) {
          const std::string qualified_template_name =
              strip_trailing_top_level_template_arguments(
                  semantic_model::class_output_qualified_name(
                      *owner_class));
          const std::string source_template_name =
              unqualified_member_name(qualified_template_name);
          const std::string use_location =
              template_api::normalize_template_witness_source_location(
                  source_location_for_name_in_subtree(
                      *specifiers,
                      !source_template_name.empty() ?
                          source_template_name :
                          qualified_template_name));
          if(!use_location.empty()) {
            const std::string key =
                template_api::class_template_effective_instantiation_key(
                    *this,
                    *owner_class);
            ClassTemplateDecl * origin_decl = class_template_origin_decl(owner_class);
            if(origin_decl && origin_arguments) {
              const template_api::specialization::ClassSpecializationSelection selection =
                  template_api::specialization::select_class_specialization(
                      *this,
                      *origin_decl,
                      scope,
                      key,
                      *origin_arguments);
              witness::ClassUseEmitRequest request;
              request.location = use_location;
              request.template_name = qualified_template_name;
              request.selection =
                  source_selection_kind_for_match_kind(selection.kind);
              request.use_anchor_present = true;
              request.use_anchor_location = use_location;
              witness::set_selected_decl_anchor(
                  request.selected_decl_location,
                  request.selected_decl_anchor,
                  class_use_selected_decl_anchor(
                      origin_decl,
                      selection));
              template_api::append_template_witness_source_bindings(
                  *this,
                  request.bindings,
                  origin_decl->parameters,
                  *origin_arguments,
                  "explicit");
              if(selection.parameters &&
                 selection.parameters !=
                     &origin_decl->parameters) {
                template_api::append_template_witness_source_bindings(
                    *this,
                    request.specialization_bindings,
                    *selection.parameters,
                    selection.arguments,
                    "deduced");
              }
              request.ownership = witness::SourceUseOwnership::SourceOwned;
              witness::emit_class_use(request);
              emit_nested_class_use_source_events_from_location(scope,
                                                                request.location,
                                                                witness::SourceUseOwnership::SourceOwned,
                                                                request.template_name);
            }
          }
        }
      }
      emit_out_of_class_owner_class_use_if_needed(
          scope,
          semantic_model::function_binding_qualified_name_for_symbol(
              *method_binding),
          declarator,
          method_binding->owner_class);
      return;
    }

    register_function(scope,
                      name,
                      type,
                      params,
                      default_args,
                      body,
                      &node,
                      declarator,
                      is_c_linkage,
                      nullptr,
                      declarator_function_qualifier(*declarator),
                      decl_spec_contains_token(*specifiers, KW_CONSTEXPR));
  }

  void collect_special_member_definition(Scope & scope, const CppAstNode & node)
  {
    const CppAstNode * declarator = find_child_kind(node, CppAstKind::declarator);
    const CppAstNode * body = find_function_body_node(node);
    if(!declarator || !body) {
      throw logic_error("special-member-definition missing children");
    }

    if(is_conversion_function_name(node.value)) {
      Scope * parse_scope = resolve_qualified_function_parse_scope(scope, *declarator);
      string name;
      TypePtr declared_type;
      MethodSyntaxInfo syntax;
      vector<pair<string, TypePtr> > params;
      if(!semantic_class_model::parse_conversion_operator_signature(*this,
                                                                    *parse_scope,
                                                                    node,
                                                                    name,
                                                                    declared_type,
                                                                    params,
                                                                    nullptr,
                                                                    &syntax)) {
        throw logic_error("unsupported out-of-class conversion operator");
      }

      FunctionBinding * binding = nullptr;
      if(!resolve_out_of_class_named_method_binding(scope,
                                                    node.value,
                                                    name,
                                                    declared_type,
                                                    syntax.is_const_method,
                                                    syntax.is_volatile_method,
                                                    syntax.ref_qualifier,
                                                    binding)) {
        throw logic_error("missing conversion operator binding");
      }
      if(!explicit_function_nothrow_specifications_match(
             *binding,
             declarator_function_qualifier(*declarator))) {
        throw logic_error(string("mismatched function exception specification [cs-conversion-special]") +
                          semantic_trace::current_location_note(*this, declarator) +
                          semantic_trace::previous_function_location_note(
                              *this, "previous declaration", binding));
      }
      if(binding->has_definition) {
        throw logic_error(string("duplicate class member definition") +
                          semantic_trace::current_location_note(*this, declarator) +
                          semantic_trace::previous_function_location_note(
                              *this, "previous definition", binding));
      }
      binding->body = body;
      record_definition_parameter_aliases(*binding, params);
      binding->has_definition = true;
      binding->definition_node = &node;
      binding->definition_abi_tags.clear();
      append_function_declaration_abi_tags(binding->definition_abi_tags, &node);
      binding->parameter_syntax_node = declarator;
      upgrade_function_symbol_linkage(
          binding,
          function_symbol_linkage(*parse_scope,
                                  &node,
                                  body,
                                  false,
                                  binding->function_qualifier,
                                  function_template_registration_identity(*binding),
                                  false,
                                  binding->lexical_access_class));
      return;
    }

    Scope * parse_scope = resolve_qualified_function_parse_scope(scope, *declarator);
    vector<pair<string, TypePtr> > params;
    const CppAstNode * parameter_clause = find_child_kind(*declarator, CppAstKind::parameter_clause);
    if(parameter_clause &&
       !parse_parameter_clause(*parse_scope, *parameter_clause, params)) {
      throw logic_error("unsupported special-member parameter-clause");
    }

    FunctionBinding * binding = nullptr;
    if(!resolve_out_of_class_special_member_binding(scope, node.value, params, binding)) {
      throw logic_error("missing special member binding");
    }
    const std::vector<TemplateArgument> * origin_arguments =
        binding ? class_template_origin_arguments(binding->owner_class) : nullptr;
    if(binding &&
       class_has_explicit_template_origin(binding->owner_class) &&
       origin_arguments &&
       !template_arguments_are_dependent(*origin_arguments)) {
      const std::string qualified_template_name =
          strip_trailing_top_level_template_arguments(
              semantic_model::class_output_qualified_name(
                  *binding->owner_class));
      const std::string source_template_name =
          unqualified_member_name(qualified_template_name);
      const std::string use_location =
          template_api::normalize_template_witness_source_location(
              source_location_for_name_in_subtree(
                  node,
                  !source_template_name.empty() ?
                      source_template_name :
                      qualified_template_name));
      if(!use_location.empty()) {
        emit_nested_class_use_source_events_from_location(scope,
                                                          use_location,
                                                          witness::SourceUseOwnership::SourceOwned);
      }
    }
    emit_out_of_class_owner_class_use_if_needed(scope, node.value, &node);
    if(!explicit_function_nothrow_specifications_match(
           *binding,
           declarator_function_qualifier(*declarator))) {
      throw logic_error(string("mismatched function exception specification [cs-special]") +
                        semantic_trace::current_location_note(*this, declarator) +
                        semantic_trace::previous_function_location_note(
                            *this, "previous declaration", binding));
    }
    if(parser_trace::enabled("class.collect")) {
      ostringstream trace;
      trace << "out-of-class-special-member-binding"
            << " qualified=" << node.value
            << " binding=" << static_cast<void *>(binding)
            << " type=" << (binding ? describe_type(binding->type) : string("<none>"))
            << " has-definition="
            << ((binding && binding->has_definition) ? "yes" : "no")
            << " decl="
            << (binding && binding->declaration_node ?
                    source_location_for_node(*binding->declaration_node) :
                    string("<none>"))
            << " def="
            << (binding && binding->definition_node ?
                    source_location_for_node(*binding->definition_node) :
                    string("<none>"));
      parser_trace::note("class.collect", std::string(), trace.str());
    }
    if(binding->has_definition) {
      throw logic_error(string("duplicate class member definition") +
                        semantic_trace::current_location_note(*this, declarator) +
                        semantic_trace::previous_function_location_note(
                            *this, "previous definition", binding));
    }
    refresh_definition_parameter_names(*binding, params);
    binding->body = body;
    record_definition_parameter_aliases(*binding, params);
    binding->ctor_initializer = find_child_kind(node, CppAstKind::ctor_initializer);
    binding->has_definition = true;
    binding->definition_node = &node;
    binding->definition_abi_tags.clear();
    append_function_declaration_abi_tags(binding->definition_abi_tags, &node);
    binding->parameter_syntax_node = declarator;
    upgrade_function_symbol_linkage(
        binding,
        function_symbol_linkage(*parse_scope,
                                &node,
                                body,
                                false,
                                binding->function_qualifier,
                                function_template_registration_identity(*binding),
                                false,
                                binding->lexical_access_class));
  }

  void collect_deduction_guide_declaration(
      Scope & scope,
      const CppAstNode & node,
      Scope * pattern_scope,
      const vector<TemplateParameterInfo> * template_parameters) override
  {
    DIAG_CONTEXT("collect_deduction_guide_declaration [" + spaced_node_text(node) + "]" +
                 source_location_for_node(node));
    const CppAstNode * declarator = find_child_kind(node, CppAstKind::declarator);
    if(!declarator) {
      throw logic_error("deduction-guide-declaration missing declarator");
    }

    const CppAstNode * identifier = find_child_kind(*declarator, CppAstKind::identifier);
    const CppAstNode * parameter_clause =
        find_child_kind(*declarator, CppAstKind::parameter_clause);
    const CppAstNode * trailing_return_type =
        find_child_kind(*declarator, CppAstKind::trailing_return_type);
    if(!identifier || !parameter_clause || !trailing_return_type) {
      throw logic_error("unsupported deduction-guide declarator");
    }

    vector<pair<string, TypePtr> > params;
    Scope & parse_scope = pattern_scope ? *pattern_scope : scope;
    if(!parse_parameter_clause(parse_scope, *parameter_clause, params)) {
      throw logic_error("unsupported deduction-guide parameter-clause");
    }

    QualifiedName return_name;
    vector<string> return_arg_texts;
    const TemplateIdSyntax * return_template_id =
        cppast_template_id_syntax(*trailing_return_type);
    if(!return_template_id ||
       return_template_id->name.rooted ||
       !return_template_id->name.qualifiers.empty()) {
      throw logic_error("unsupported deduction-guide return type");
    }
    return_name = return_template_id->name;
    return_arg_texts = return_template_id->arguments;
    if(return_name.name != identifier->value) {
      throw logic_error("deduction-guide return type must name the same class template");
    }

    ClassTemplateDecl * primary = lookup_class_template(scope, identifier->value);
    if(!primary) {
      throw logic_error("unknown class template deduction guide");
    }
    if(!trailing_pack_accepts_argument_count(primary->parameters, return_arg_texts.size())) {
      throw logic_error("invalid deduction-guide return type");
    }

    DeductionGuideDecl guide;
    guide.declaring_scope = &scope;
    guide.pattern_scope = pattern_scope;
    guide.name = identifier->value;
    guide.node = &node;
    guide.declarator = declarator;
    if(template_parameters) {
      guide.parameters = *template_parameters;
    }
    guide.return_arg_texts = return_arg_texts;
    primary->deduction_guides.push_back(guide);
  }

  string make_template_specialization_name(const string & name,
                                           const vector<TemplateArgument> & arguments) const
  {
    return template_api::specialization_name_for_instantiation(
        *const_cast<Analyzer *>(this),
        name,
        arguments);
  }

  string display_template_specialization_name(const string & name,
                                              const vector<TemplateArgument> & arguments) const
  {
    return template_api::display_specialization_name_for_instantiation(
        *const_cast<Analyzer *>(this),
        name,
        arguments);
  }

  void collect_template_declaration_impl(
      Scope & scope,
      const CppAstNode & node,
      MemberAccess access,
      const vector<TemplateParameterInfo> * inherited_template_parameters)
  {
    callsemantic::TemplateDeclarationCollectorState state = {
        class_templates,
        alias_templates,
        function_templates,
        variable_templates};
    callsemantic::TemplateDeclarationCollectorServices callbacks;
    callbacks.declaration_services = this;
    callbacks.source_services = this;
    callbacks.out_of_class_services = this;
    callbacks.function_policy = this;
    callsemantic::collect_template_declaration_impl(
        *this, state, callbacks, scope, node, access, inherited_template_parameters);
  }

  void collect_deduction_guide_declaration(Scope & scope, const CppAstNode & node)
  {
    collect_deduction_guide_declaration(scope, node, nullptr, nullptr);
  }

  ClassInfo * instantiate_class_template(ClassTemplateDecl & decl,
                                         Scope & use_scope,
                                         const vector<string> & arg_texts,
                                         const vector<TemplateArgumentSyntax> * arg_syntaxes = nullptr)
  {
    const callsemantic::ClassTemplateReferenceCallbacks callbacks =
        class_template_reference_callbacks();
    return callsemantic::instantiate_class_template_with_syntax(
        *this, callbacks, decl, use_scope, arg_texts, arg_syntaxes);
  }

  ClassInfo * instantiate_selected_class_template(
      ClassTemplateDecl & decl,
      Scope & use_scope,
      const vector<TemplateArgument> & arguments,
      const template_api::ClassSpecializationSelection & specialization)
  {
    if(specialization.kind == template_api::MS_EXPLICIT_SPECIALIZATION &&
       specialization.class_node) {
      const std::string use_location = parser_trace::current_use_location();
      const std::string specialization_location =
          source_location_for_node(*specialization.class_node);
      if(!use_location.empty() &&
         !specialization_location.empty() &&
         source_location_is_later(use_location, specialization_location)) {
        throw logic_error(string("explicit specialization after instantiation") +
                          " [use " + use_location + "]" +
                          " [specialization " + specialization_location + "]");
      }
    }
    template_api::TemplateSelectedClassInstantiationRequest request;
    request.decl = &decl;
    request.use_scope = template_api::make_template_environment(use_scope);
    request.arguments = arguments;
    request.specialization = specialization;
    return template_api::acquire_selected_class_instantiation(*this, request).class_info;
  }

  void promote_referenced_class_template_instantiation(ClassInfo & info)
  {
    if(!info.source_template || info.instantiation_key.empty()) {
      return;
    }
    ClassTemplateDecl & decl = *info.source_template;
    map<string, ClassInfo *>::iterator found =
        decl.instantiations.find(info.instantiation_key);
    if(found == decl.instantiations.end()) {
      decl.instantiations[info.instantiation_key] = &info;
    }
    map<string, ClassInfo *>::iterator reference_found =
        decl.reference_instantiations.find(info.instantiation_key);
    if(reference_found != decl.reference_instantiations.end() &&
       reference_found->second == &info) {
      decl.reference_instantiations.erase(reference_found);
    }
  }

  FunctionBinding * instantiate_function_template(FunctionTemplateDecl & decl,
                                                  const vector<TemplateArgument> & arguments,
                                                  const CppAstNode * body_override = nullptr,
                                                  bool explicit_specialization = false,
                                                  bool include_body = true,
                                                  Scope * use_scope = nullptr,
                                                  const map<string, size_t> * pack_sizes = nullptr,
                                                  bool prefer_overload_suffix = false,
                                                  ClassInfo * active_owner = nullptr)
  {
    ++metrics_.template_instantiation_requests;
    return acquire_function_template(decl,
                                     arguments,
                                     use_scope,
                                     pack_sizes,
                                     include_body,
                                     explicit_specialization,
                                     body_override,
                                     prefer_overload_suffix,
                                     active_owner)
        .function_binding;
  }

  void finalize_instantiated_class_materialization(
      ClassTemplateDecl & decl,
      ClassInfo & info,
      const vector<TemplateArgument> & arguments)
  {
    template_api::TemplateClassFinalizationRequest request;
    request.decl = &decl;
    request.info = &info;
    request.arguments = arguments;
    template_api::finalize_class_instantiation(*this, request);
  }

  template_api::TemplateInstantiationResult acquire_function_template(
      FunctionTemplateDecl & decl,
      const vector<TemplateArgument> & arguments,
      Scope * use_scope = nullptr,
      const map<string, size_t> * pack_sizes = nullptr,
      bool include_body = true,
      bool explicit_specialization = false,
      const CppAstNode * body_override = nullptr,
      bool prefer_overload_suffix = false,
      ClassInfo * active_owner = nullptr,
      template_api::TemplateInstantiationIntent intent =
          template_api::TemplateInstantiationIntent::LookupOnly)
  {
    template_api::TemplateFunctionInstantiationRequest request;
    request.decl = &decl;
    request.arguments = arguments;
    request.active_owner = active_owner;
    request.use_scope = use_scope ? template_api::make_template_environment(*use_scope) :
                                    template_api::TemplateEnvironmentHandle();
    request.body_override = body_override;
    request.explicit_specialization = explicit_specialization;
    request.include_body = include_body;
    request.prefer_overload_suffix = prefer_overload_suffix;
    request.intent = intent;
    if(pack_sizes) {
      request.pack_sizes = *pack_sizes;
      request.has_pack_sizes = true;
    }
    return template_api::acquire_function_instantiation(*this, request);
  }

  FunctionBinding * ensure_function_template_definition(FunctionBinding * binding,
                                                        Scope & use_scope)
  {
    if(!binding) {
      return binding;
    }
    if(!function_binding_is_live(binding)) {
      return nullptr;
    }
    const template_api::TemplateFunctionDefinitionClosureState closure_state =
        template_api::function_definition_closure_state(*this, binding);
    ClassInfo * template_owner = closure_state.template_owner;
    const bool template_owned_binding = closure_state.template_owned_binding;
    const bool closure_trigger_differs = closure_state.closure_trigger_differs;
    if(template_owned_binding) {
      if(binding->has_definition) {
        if(closure_trigger_differs) {
          template_api::note_function_definition_ensure_requested(
              *this,
              binding,
              closure_state);
          template_api::note_closure_owner_class_instantiation_if_needed(
              *this,
              binding->owner_class,
              closure_state);
          template_api::note_function_definition_materialized_by_closure(
              *this,
              binding,
              closure_state);
          template_api::mark_function_definition_materialized_by_enclosing_closure(
              binding);
          return binding;
        }
        if(template_api::function_definition_materialized_by_enclosing_closure(
               binding)) {
          return binding;
        }
      }
      template_api::note_function_definition_ensure_requested(
          *this,
          binding,
          closure_state);
      if(binding->has_definition) {
        template_api::note_function_definition_materialized_by_closure(
            *this,
            binding,
            closure_state);
        return binding;
      }
      if(!template_api::function_binding_has_source_template_identity(binding)) {
        template_api::note_function_declaration_instantiated_by_closure(
            *this,
            binding,
            closure_state);
      }
    }
    if(template_api::function_binding_has_source_template_identity(binding)) {
      FunctionBinding * original_binding = binding;
      Scope * instantiation_scope =
          binding->declaration_scope ? binding->declaration_scope : &use_scope;
      template_api::TemplateFunctionInstantiationRequest upgrade_request;
      if(!template_api::build_function_definition_upgrade_request(*binding,
                                                                  *instantiation_scope,
                                                                  upgrade_request)) {
        throw logic_error("missing function template definition upgrade request");
      }
      const FunctionTemplateRegistrationIdentity template_identity =
          template_api::function_binding_registration_identity(*binding);
      if(parser_trace::enabled("template.resolve")) {
        std::ostringstream trace;
        trace << "ensure-template-definition binding=" << static_cast<void *>(binding)
              << " function=" << binding->name
              << " owner="
              << (binding->owner_class ? binding->owner_class->qualified_name :
                                         std::string("<none>"))
              << " source_owner="
              << (upgrade_request.decl &&
                          upgrade_request.decl->declaring_scope &&
                          upgrade_request.decl->declaring_scope->class_info ?
                      upgrade_request.decl->declaring_scope->class_info->qualified_name :
                      std::string("<none>"))
              << " decl_scope="
              << (binding->declaration_scope ?
                      scope_qualified_name(*binding->declaration_scope, "<here>") :
                      std::string("<none>"))
              << " decl_scope_owner="
              << (binding->declaration_scope && binding->declaration_scope->class_info ?
                      binding->declaration_scope->class_info->qualified_name :
                      std::string("<none>"))
              << " caller_scope=" << scope_qualified_name(use_scope, "<here>")
              << " caller_scope_owner="
              << (use_scope.class_info ? use_scope.class_info->qualified_name :
                                         std::string("<none>"))
              << " chosen_scope="
              << scope_qualified_name(*instantiation_scope, "<here>")
              << " chosen_scope_owner="
              << (instantiation_scope->class_info ?
                      instantiation_scope->class_info->qualified_name :
                      std::string("<none>"))
              << " key=" << template_identity.key;
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      ++metrics_.template_definition_upgrades;
      binding =
          template_api::acquire_function_instantiation(*this, upgrade_request)
              .function_binding;
      binding = preserve_owner_prefixed_template_binding(original_binding, binding);
      if(!binding || binding->has_definition) {
        return binding;
      }
    }

    const FunctionBindingRefreshKey refresh_key =
        make_function_binding_refresh_key(*binding);
    if(template_owner &&
       template_owner->source_template &&
       !template_owner->out_of_class_member_function_definitions_applied) {
      template_api::apply_out_of_class_member_function_abi_metadata(*this,
                                                                    *template_owner);
      binding = refresh_function_binding_after_class_update(refresh_key, binding);
      if(!binding) {
        return nullptr;
      }
    }

    if(template_owner &&
       template_owner->complete &&
       !template_owner->template_instantiation_in_progress &&
       (!template_owner->out_of_class_member_function_template_definitions_applied ||
        !template_owner->out_of_class_member_function_definitions_applied ||
        !template_owner->out_of_class_special_member_definitions_applied ||
        !template_owner->out_of_class_static_member_definitions_applied)) {
      const template_api::ScopedTemplateWitnessEntryContext entry_context =
          template_api::maybe_enter_class_finalization_context(*this, template_owner);
      template_api::TemplateClassFinalizationRequest finalization_request;
      if(!template_api::build_class_finalization_request(*template_owner,
                                                        finalization_request)) {
        throw logic_error("missing class template finalization request");
      }
      template_api::finalize_class_instantiation(*this, finalization_request);
      binding = refresh_function_binding_after_class_update(refresh_key, binding);
      if(!binding) {
        return nullptr;
      }
      if(binding->has_definition) {
        if(template_owned_binding) {
          if(closure_trigger_differs) {
            template_api::note_closure_owner_class_instantiation_if_needed(
                *this,
                binding->owner_class,
                closure_state);
            template_api::mark_function_definition_materialized_by_enclosing_closure(
                binding);
          }
          template_api::note_function_definition_materialized_by_closure(
              *this,
              binding,
              closure_state);
        }
        return binding;
      }
    }

    FunctionBinding * upgraded = nullptr;
    if(binding->owner_class) {
      if(binding->is_method ||
         binding->is_constructor ||
         binding->is_destructor) {
        const string member_name =
            unqualified_member_name(canonical_function_lookup_name(binding->name));
        upgraded =
            template_api::find_defined_class_function_matching_template_identity(
                *this,
                *binding->owner_class,
                member_name,
                *binding);
        if(!upgraded &&
           template_api::function_binding_has_empty_template_identity(*binding)) {
          upgraded = find_equivalent_class_function(*binding->owner_class,
                                                    member_name,
                                                    binding->type,
                                                    binding->ref_qualifier);
        }
      } else if(binding->declaration_scope) {
        const string lookup_name =
            binding->display_name.empty() ?
                unqualified_member_name(canonical_function_lookup_name(binding->name)) :
                binding->display_name;
        upgraded = template_api::find_defined_function_matching_template_identity(
            *this,
            *binding->declaration_scope,
            lookup_name,
            *binding);
      }
    } else if(binding->declaration_scope) {
      upgraded = template_api::find_defined_function_matching_template_identity(
          *this,
          *binding->declaration_scope,
          binding->name,
          *binding);
    }
    if(upgraded && upgraded != binding) {
      output_requirement_engine::State state = make_output_requirement_state();
      output_requirement_engine::Hooks hooks = make_output_requirement_hooks();
      upgraded = output_requirement_engine::adopt_output_requirements(state,
                                                                      hooks,
                                                                      binding,
                                                                      upgraded,
                                                                      OutputReason::TemplateUpgrade);
      if(parser_trace::enabled("template.resolve")) {
        std::ostringstream trace;
        trace << "upgrade-function-definition binding=" << static_cast<void *>(binding)
              << " upgraded=" << static_cast<void *>(upgraded)
              << " function=" << binding->name
              << " owner="
              << (binding->owner_class ? binding->owner_class->qualified_name :
                                         std::string("<none>"))
              << " key="
              << template_api::function_binding_registration_identity(*binding).key;
        parser_trace::note("template.resolve", std::string(), trace.str());
      }
      return upgraded;
    }
    return binding;
  }

  bool deduce_function_template_arguments(FunctionTemplateDecl & decl,
                                          const vector<ExprInfo> & args,
                                          vector<TemplateArgument> & out,
                                          Scope * use_scope = nullptr,
                                          map<string, size_t> * pack_sizes_out = nullptr)
  {
    template_api::TemplateFunctionDeductionRequest request;
    request.decl = &decl;
    request.args = &args;
    request.use_scope = use_scope;
    template_api::TemplateFunctionDeductionResult result;
    const bool okay = template_api::deduce_function_template(*this, request, result);
    if(okay) {
      out = result.arguments;
      if(pack_sizes_out) {
        *pack_sizes_out = result.pack_sizes;
      }
    }
    return okay;
  }

  bool build_partial_ordering_placeholder_arguments(
      const vector<TemplateParameterInfo> & parameters,
      vector<TemplateArgument> & out)
  {
    out.clear();
    for(size_t i = 0; i < parameters.size(); ++i) {
      if(parameters[i].kind != TemplateParameterInfo::TP_TYPE ||
         parameters[i].parameter_pack) {
        return false;
      }

      TemplateArgument argument;
      argument.kind = TemplateArgument::TA_TYPE;
      ostringstream key;
      key << "partial-order "
          << (parameters[i].name.empty() ? string("<unnamed>") : parameters[i].name)
          << "#" << i;
      argument.type =
          make_named(string("typename ") +
                         (parameters[i].name.empty() ? string("<unnamed>") : parameters[i].name),
                     key.str(),
                     false);
      argument.text = key.str();
      out.push_back(argument);
    }
    return true;
  }

  void collect_namespace_alias_definition(Scope & scope, const CppAstNode & node)
  {
    semantic_declaration::collect_namespace_alias_definition(*this, scope, node);
  }

  void collect_using_directive(Scope & scope, const CppAstNode & node)
  {
    semantic_declaration::collect_using_directive(*this, scope, node);
  }

  void collect_using_declaration(Scope & scope, const CppAstNode & node)
  {
    semantic_declaration::collect_using_declaration(*this, scope, node);
  }

  void analyze_static_assert_declaration(Scope & scope, const CppAstNode & node)
  {
    semantic_declaration::analyze_static_assert_declaration(*this, scope, node);
  }

  void inject_inline_namespace_members(Scope & scope, Scope & target)
  {
    for(map<string, Scope *>::iterator it = target.namespace_bindings.begin();
        it != target.namespace_bindings.end(); ++it) {
      scope.namespace_bindings[it->first] = it->second;
    }

    for(map<string, TypePtr>::iterator it = target.named_types.begin();
        it != target.named_types.end(); ++it) {
      if(scope.named_types.count(it->first) == 0) {
        scope.named_types[it->first] = it->second;
      }
    }

    for(map<string, ValueBinding>::iterator it = target.values.begin();
        it != target.values.end(); ++it) {
      if(scope.values.count(it->first) == 0) {
        scope.values[it->first] = it->second;
      }
    }

    for(map<string, vector<FunctionBinding *> >::iterator it = target.function_sets.begin();
        it != target.function_sets.end(); ++it) {
      vector<FunctionBinding *> & slot =
          semantic_lookup::direct_function_set_slot(scope, it->first);
      for(size_t i = 0; i < it->second.size(); ++i) {
        bool duplicate = find(slot.begin(), slot.end(), it->second[i]) != slot.end();
        if(!duplicate) {
          for(size_t j = 0; j < slot.size(); ++j) {
            if(types_equivalent_for_member_binding(slot[j]->type, it->second[i]->type)) {
              duplicate = true;
              break;
            }
          }
        }
        if(!duplicate) {
          slot.push_back(it->second[i]);
        }
      }
    }

    for(map<string, ClassTemplateDecl *>::iterator it = target.class_templates.begin();
        it != target.class_templates.end(); ++it) {
      if(scope.class_templates.count(it->first) == 0) {
        scope.class_templates[it->first] = it->second;
      }
    }

    for(map<string, vector<FunctionTemplateDecl *> >::iterator it = target.function_templates.begin();
        it != target.function_templates.end(); ++it) {
      vector<FunctionTemplateDecl *> & slot =
          semantic_lookup::direct_function_template_slot(scope, it->first);
      for(size_t i = 0; i < it->second.size(); ++i) {
        if(find(slot.begin(), slot.end(), it->second[i]) == slot.end()) {
          slot.push_back(it->second[i]);
        }
      }
    }

    for(map<string, AliasTemplateDecl *>::iterator it = target.alias_templates.begin();
        it != target.alias_templates.end(); ++it) {
      if(scope.alias_templates.count(it->first) == 0) {
        scope.alias_templates[it->first] = it->second;
      }
    }

    for(map<string, VariableTemplateDecl *>::iterator it = target.variable_templates.begin();
        it != target.variable_templates.end(); ++it) {
      if(scope.variable_templates.count(it->first) == 0) {
        scope.variable_templates[it->first] = it->second;
      }
    }

    if(find(scope.using_directives.begin(), scope.using_directives.end(), &target) ==
       scope.using_directives.end()) {
      scope.using_directives.push_back(&target);
    }
  }

  void collect_template_declaration(Scope & scope,
                                    const CppAstNode & node,
                                    MemberAccess access = MA_PUBLIC)
  {
    collect_template_declaration_impl(scope, node, access, nullptr);
  }

  void collect_namespace_definition(Scope & scope, const CppAstNode & node)
  {
    const string namespace_name = node.value.empty() ? string("<unnamed>") : node.value;
    DIAG_CONTEXT("collect_namespace_definition [" + namespace_name + "]" +
                 source_location_for_node(node));
    semantic_declaration::collect_namespace_definition(*this, scope, node);
  }

  void collect_declaration(Scope & scope,
                           const CppAstNode & child,
                           bool is_c_linkage = false,
                           bool linkage_has_braces = false)
  {
    semantic_declaration::collect_declaration(*this, scope, child, is_c_linkage, linkage_has_braces);
  }

  void collect_top_level_declarations()
  {
    semantic_declaration::collect_top_level_declarations(*this, *root, ast);
  }

  ExprInfo analyze_expression(Scope & scope, const CppAstNode & node)
  {
    DIAG_CONTEXT("analyze_expression [" + node_text(node) + "]" +
                 source_location_for_node(node));
    const std::string inherited_use_location = template_public_use_location_or(string());
    const std::string node_use_location = preferred_source_location_for_node(node);
    ScopedTemplateUseLocation use_location_guard(
        !inherited_use_location.empty() ? inherited_use_location : node_use_location);
    ExprInfo result = semantic_expression::analyze_expression(*this, scope, node);
    result.null_pointer_constant = classify_null_pointer_constant(scope, node, result);
    return result;
  }

  ExprInfo analyze_expression_for_target(Scope & scope,
                                         const CppAstNode & node,
                                         const TypePtr & target)
  {
    DIAG_CONTEXT("analyze_expression_for_target [" + node_text(node) +
                 " -> " + (target ? describe_type(target) : string("<null-type>")) + "]" +
                 source_location_for_node(node));
    const std::string inherited_use_location = template_public_use_location_or(string());
    const std::string node_use_location = preferred_source_location_for_node(node);
    ScopedTemplateUseLocation use_location_guard(
        !inherited_use_location.empty() ? inherited_use_location : node_use_location);
    return semantic_expression::analyze_expression_for_target(*this, scope, node, target);
  }

  ExprInfo analyze_braced_init_list_expression(Scope & scope,
                                               const CppAstNode & node)
  {
    return semantic_expression::analyze_braced_init_list_expression(*this, scope, node);
  }

  string sanitize_internal_identifier_fragment(const string & text) const
  {
    string out;
    out.reserve(text.size());
    for(size_t i = 0; i < text.size(); ++i) {
      const unsigned char ch = static_cast<unsigned char>(text[i]);
      out.push_back(std::isalnum(ch) ? static_cast<char>(ch) : '_');
    }
    if(out.empty()) {
      out = "anon";
    }
    return out;
  }

  string next_lambda_class_name(Scope & scope, const CppAstNode * anchor)
  {
    ostringstream name;
    name << "__lambda";

    if(FunctionBinding * current = current_function_scope(scope)) {
      string function_identity = current->name;
      const FunctionTemplateRegistrationIdentity template_identity =
          template_api::function_binding_registration_identity(*current);
      if(!template_identity.key.empty()) {
        function_identity += "__" + template_identity.key;
      }
      name << "_" << sanitize_internal_identifier_fragment(function_identity);
    } else if(scope.class_info && !scope.class_info->qualified_name.empty()) {
      name << "_" << sanitize_internal_identifier_fragment(scope.class_info->qualified_name);
    }

    const bool has_usable_token_range =
        anchor &&
        anchor->token_end >= anchor->token_start &&
        !(anchor->token_start == 0 && anchor->token_end == 0);
    if(has_usable_token_range) {
      name << "_t" << anchor->token_start << "_" << anchor->token_end;
    } else {
      name << "_n" << ++lambda_counter;
    }
    return name.str();
  }

  bool build_current_lambda_itanium_context_encoding(Scope & scope,
                                                     string & out,
                                                     vector<string> & substitution_keys)
  {
    FunctionBinding * current = current_function_scope(scope);
    if(!current || !current->type) {
      return false;
    }

    symbol_linkage::FunctionSymbolOptions options;
    populate_function_symbol_options(options,
                                     current->is_const_method,
                                     current->is_volatile_method,
                                     current->ref_qualifier,
                                     current->is_constructor,
                                     current->is_destructor,
                                     function_template_registration_identity(*current),
                                     current->owner_class,
                                     current->declaration_scope,
                                     current->declaration_node,
                                     current->definition_node);
    if(!current->instantiation_pack_sizes.empty()) {
      options.template_argument_pack_sizes = &current->instantiation_pack_sizes;
    }
    options.abi_tags = function_binding_abi_tags(*current);

    string function_encoding;
    QualifiedName qualified_name_syntax;
    const bool has_qualified_name_syntax =
        function_binding_qualified_name_syntax_for_symbol(*current,
                                                          qualified_name_syntax);
    if(!has_qualified_name_syntax) {
      return false;
    }
    const bool mangled =
        symbol_linkage::mangle_itanium_function_encoding_with_substitutions(
            qualified_name_syntax,
            function_binding_display_name_for_symbol(*current),
            current->type,
            options,
            function_encoding,
            &substitution_keys);
    if(!mangled) {
      return false;
    }

    out = "Z" + function_encoding + "E";
    return out.compare(0, 2, "ZN") == 0 || out.compare(0, 2, "ZZ") == 0;
  }

  bool build_lambda_itanium_signature_encoding(
      const vector<pair<string, TypePtr> > & params,
      const vector<string> & context_substitution_keys,
      string & out,
      vector<string> & substitution_keys) const
  {
    vector<TypePtr> param_types;
    param_types.reserve(params.size());
    for(size_t i = 0; i < params.size(); ++i) {
      param_types.push_back(params[i].second);
    }
    return symbol_linkage::mangle_itanium_lambda_signature_encoding(
        param_types,
        context_substitution_keys,
        out,
        &substitution_keys);
  }

  void assign_lambda_itanium_metadata(ClassInfo & info,
                                      Scope & scope,
                                      const vector<pair<string, TypePtr> > & params)
  {
    string context_encoding;
    string signature_encoding;
    vector<string> context_substitution_keys;
    vector<string> substitution_keys;
    if(!build_current_lambda_itanium_context_encoding(
           scope, context_encoding, context_substitution_keys) ||
       !build_lambda_itanium_signature_encoding(
           params,
           context_substitution_keys,
           signature_encoding,
           substitution_keys)) {
      return;
    }

    const string count_key = context_encoding + "|" + signature_encoding;
    size_t & count = lambda_itanium_signature_counts[count_key];
    const string discriminator = count > 0 ? to_string(count - 1) : string();
    signature_encoding += discriminator;
    ++count;
    signature_encoding += '_';

    vector<TypePtr> signature_parameter_types;
    signature_parameter_types.reserve(params.size());
    for(size_t i = 0; i < params.size(); ++i) {
      signature_parameter_types.push_back(params[i].second);
    }

    info.lambda_itanium_context_encoding = context_encoding;
    info.lambda_itanium_signature_encoding = signature_encoding;
    info.lambda_itanium_substitution_keys = substitution_keys;
    if(info.type && info.type->kind == Type::TK_NAMED) {
      info.type->named_itanium_abi_encoding =
          context_encoding + signature_encoding;
      info.type->named_itanium_abi_substitution_keys = substitution_keys;
      info.type->named_itanium_abi_context_encoding = context_encoding;
      info.type->named_itanium_abi_context_substitution_keys =
          context_substitution_keys;
      info.type->named_itanium_abi_signature_parameter_types =
          signature_parameter_types;
      info.type->named_itanium_abi_discriminator = discriminator;
    }
  }

  SyntheticLambdaKey make_synthetic_lambda_key(Scope & scope, const CppAstNode & node) const
  {
    SyntheticLambdaKey key;
    key.node = &node;
    key.function = current_function_scope(scope);
    key.class_info = current_class_scope(scope);
    if(key.function) {
      key.function_instantiation_key = key.function->template_instantiation_key;
    }
    if(key.class_info) {
      key.class_instantiation_key = key.class_info->instantiation_key;
    }
    if(!key.function && !key.class_info) {
      key.scope_key = scope_fragment_parse_fingerprint(scope);
    }
    return key;
  }

  FunctionBinding * lookup_synthetic_lambda_binding(Scope & scope,
                                                    const CppAstNode & node)
  {
    map<SyntheticLambdaKey, FunctionBinding *>::iterator found =
        synthetic_lambda_bindings.find(make_synthetic_lambda_key(scope, node));
    return found == synthetic_lambda_bindings.end() ? nullptr : found->second;
  }

  ClassInfo * lookup_synthetic_lambda_closure(Scope & scope,
                                              const CppAstNode & node)
  {
    map<SyntheticLambdaKey, ClassInfo *>::iterator found =
        synthetic_lambda_closures.find(make_synthetic_lambda_key(scope, node));
    return found == synthetic_lambda_closures.end() ? nullptr : found->second;
  }

  void register_synthetic_lambda_binding(Scope & scope,
                                         const CppAstNode & node,
                                         FunctionBinding & binding)
  {
    synthetic_lambda_bindings[make_synthetic_lambda_key(scope, node)] = &binding;
  }

  void register_synthetic_lambda_closure(Scope & scope,
                                         const CppAstNode & node,
                                         ClassInfo & info)
  {
    synthetic_lambda_closures[make_synthetic_lambda_key(scope, node)] = &info;
  }

  FunctionBinding * create_synthetic_lambda_function(
      Scope & scope,
      const TypePtr & function_type,
      const vector<pair<string, TypePtr> > & params,
      const vector<const CppAstNode *> & default_arguments,
      const CppAstNode * declarator,
      const CppAstNode * body)
  {
    const CppAstNode * durable_declarator =
        declarator ? own_synthetic_ast(*declarator) : nullptr;
    const CppAstNode * durable_body =
        body ? own_synthetic_ast(*body) : nullptr;
    unique_ptr<FunctionBinding> binding(new FunctionBinding());
    const CppAstNode * lambda_anchor = declarator ? declarator : body;
    const string function_name = next_lambda_class_name(scope, lambda_anchor);
    Scope & durable_scope = persistent_enclosing_scope(scope);
    binding->name = function_name;
    binding->display_name = function_name;
    binding->declaration_scope = &durable_scope;
    binding->declaration_node = durable_declarator;
    binding->definition_node = durable_declarator;
    binding->type = function_type;
    binding->params = params;
    binding->default_arguments = normalize_default_arguments(params, default_arguments);
    initialize_function_parameter_aliases(*binding);
    binding->body = durable_body;
    binding->has_definition = durable_body != nullptr;
    binding->lexical_access_class = current_class_scope(scope);
    binding->lexical_access_function = current_function_scope(scope);
    functions.push_back(std::move(binding));
    index_function_binding(functions.back().get());
    synthetic_functions.push_back(functions.back().get());
    return functions.back().get();
  }

  void mark_rtti_required_for_class(ClassInfo * info)
  {
    if(!info) {
      return;
    }
    for(size_t i = 0; i < classes.size(); ++i) {
      ClassInfo * candidate = classes[i].get();
      if(is_same_or_derived(candidate, info)) {
        candidate->rtti_required = true;
      }
    }
    info->rtti_required = true;
    for(size_t i = 0; i < info->bases.size(); ++i) {
      mark_rtti_required_for_class(info->bases[i].type);
    }
  }

  void note_rtti_use(const TypePtr & type, bool dynamic_use)
  {
    if(!type) {
      return;
    }
    const string symbol = rtti_symbol_for_type(type);
    emitted_rtti_symbols.insert(symbol);
    emitted_rtti_types[symbol] = type;
    if(dynamic_use) {
      mark_rtti_required_for_class(class_info_for_type(type));
    }
  }

  void note_dynamic_typeid_use(const TypePtr & type)
  {
    (void)type;
  }

  bool find_subobject_offset(const ClassInfo & info,
                             const ClassInfo * target,
                             size_t & out_offset) const
  {
    MemberAccess access = MA_PUBLIC;
    return find_unique_base_path(info, target, out_offset, access);
  }

  bool find_vtable_key_for_view(const ClassInfo & dynamic_class,
                                const ClassInfo * view_class,
                                string & out_key,
                                size_t & out_offset) const
  {
    if(!find_subobject_offset(dynamic_class, view_class, out_offset)) {
      return false;
    }
    if(out_offset == 0) {
      out_key = dynamic_class.qualified_name;
      return true;
    }
    for(size_t i = 0; i < dynamic_class.vtables.size(); ++i) {
      if(dynamic_class.vtables[i].view_offset == out_offset &&
         dynamic_class.vtables[i].view_type == view_class) {
        out_key = dynamic_class.vtables[i].key;
        return true;
      }
    }
    return false;
  }

  void append_rtti_candidates(const TypePtr & static_type,
                              DumpNode & node,
                              ClassInfo * required_target = nullptr)
  {
    ClassInfo * static_class = class_info_for_type(static_type);
    if(!static_class) {
      return;
    }

    for(size_t i = 0; i < classes.size(); ++i) {
      ClassInfo * candidate = classes[i].get();
      if(!candidate->complete || !candidate->is_polymorphic) {
        continue;
      }
      if(!is_same_or_derived(candidate, static_class)) {
        continue;
      }
      if(required_target && !is_same_or_derived(candidate, required_target)) {
        continue;
      }

      string table_key;
      size_t source_offset = 0;
      if(!find_vtable_key_for_view(*candidate, static_class, table_key, source_offset)) {
        continue;
      }

      DumpNode child = make_dump_node(CallSemKind::rtti_candidate, table_key);
      child.semantic_type = candidate->type;
      if(required_target) {
        size_t target_offset = 0;
        MemberAccess target_access = MA_PUBLIC;
        if(!find_unique_base_path(*candidate, required_target, target_offset, target_access)) {
          continue;
        }
        set_callsem_int_value(child,
                              static_cast<long long>(target_offset) -
                                  static_cast<long long>(source_offset));
      } else {
        set_callsem_int_value(child, -static_cast<long long>(source_offset));
      }
      node.children.push_back(child);
    }
  }

  void append_exception_candidates(const TypePtr & catch_type,
                                   DumpNode & node)
  {
    ClassInfo * catch_class = class_info_for_type(catch_type);
    if(!catch_class) {
      return;
    }

    for(size_t i = 0; i < classes.size(); ++i) {
      ClassInfo * candidate = classes[i].get();
      if(!candidate->complete || !is_same_or_derived(candidate, catch_class)) {
        continue;
      }

      size_t target_offset = 0;
      MemberAccess target_access = MA_PUBLIC;
      if(!find_unique_base_path(*candidate, catch_class, target_offset, target_access)) {
        continue;
      }

      note_rtti_use(candidate->type, false);
      DumpNode child = make_dump_node(CallSemKind::rtti_candidate,
                                      rtti_symbol_for_type(candidate->type));
      child.semantic_type = candidate->type;
      set_callsem_int_value(child, static_cast<long long>(target_offset));
      node.children.push_back(child);
    }
  }

  bool has_dynamic_cast_candidate(ClassInfo * source_class,
                                  ClassInfo * target_class)
  {
    if(!source_class || !target_class) {
      return false;
    }
    if(is_same_or_derived(source_class, target_class) ||
       is_same_or_derived(target_class, source_class)) {
      return true;
    }
    for(size_t i = 0; i < classes.size(); ++i) {
      ClassInfo * candidate = classes[i].get();
      if(!candidate->complete || !candidate->is_polymorphic) {
        continue;
      }
      if(is_same_or_derived(candidate, source_class) &&
         is_same_or_derived(candidate, target_class)) {
        return true;
      }
    }
    return false;
  }

  ClassInfo * synthesize_lambda_closure_class(Scope & scope,
                                              const vector<pair<string, bool> > & captures,
                                              const vector<pair<string, TypePtr> > & params,
                                              const vector<const CppAstNode *> & default_arguments,
                                              const TypePtr & result_type,
                                              bool mutable_lambda,
                                              const CppAstNode * declarator,
                                              const CppAstNode * body,
                                              FunctionBinding *& call_operator)
  {
    const CppAstNode * lambda_anchor = declarator ? declarator : body;
    const string class_name = next_lambda_class_name(scope, lambda_anchor);
    ClassInfo * info = create_class_info(scope, "class", class_name);
    info->is_lambda_closure = true;

    const auto resolve_this_capture_type =
        [&](TypePtr & out) -> bool
    {
      for(Scope * current = &scope; current; current = current->parent) {
        if(!current->function) {
          continue;
        }
        ClassInfo * current_class =
            current->class_info ? current->class_info : current->function->owner_class;
        if(!current_class) {
          continue;
        }
        if(current_class->is_lambda_closure) {
          MemberValueLookupResult captured_this =
              lookup_member_value(*current_class, "this");
          if(captured_this.binding &&
             captured_this.binding->kind == ValueBinding::VK_FIELD &&
             captured_this.binding->type) {
            out = captured_this.binding->type;
            return true;
          }
          continue;
        }
        map<string, ValueBinding>::iterator found = current->values.find("this");
        if(found != current->values.end() && found->second.type) {
          out = found->second.type;
          return true;
        }
      }
      return false;
    };

    for(size_t i = 0; i < captures.size(); ++i) {
      const string & capture_name = captures[i].first;
      const bool capture_by_reference = captures[i].second;

      FieldInfo field;
      field.name = capture_name;
      if(capture_name == "this") {
        if(!resolve_this_capture_type(field.type) || !field.type) {
          throw logic_error("unsupported lambda capture");
        }
      } else {
        const ValueBinding * outer = lookup_value(scope, capture_name);
        const bool capture_from_lambda_field =
            outer &&
            outer->kind == ValueBinding::VK_FIELD &&
            outer->owner_class &&
            outer->owner_class->is_lambda_closure &&
            capture_name != "this";
        if(!outer || (outer->kind == ValueBinding::VK_FIELD && !capture_from_lambda_field)) {
          throw logic_error("unsupported lambda capture");
        }
        if(!capture_by_reference && complete_class_type(outer->type)) {
          throw logic_error("PA24 supports only non-class lambda captures");
        }
        field.type = capture_by_reference && !is_reference_type(outer->type) ?
                         make_lvalue_reference_raw(outer->type) :
                         outer->type;
      }
      field.access = MA_PRIVATE;
      info->fields.push_back(field);

      ValueBinding binding(ValueBinding::VK_FIELD, capture_name, field.type);
      binding.owner_class = info;
      binding.access = MA_PRIVATE;
      info->member_scope->values[capture_name] = binding;
    }

    const CppAstNode * template_parameters =
        declarator ? find_child_kind(*declarator, CppAstKind::template_parameter_clause) : nullptr;
    if(template_parameters) {
      call_operator = nullptr;

      Scope & template_scope = append_template_scope(*info->member_scope);
      vector<TemplateParameterInfo> template_parameter_infos;
      string parameter_failure;
      if(!parse_template_parameters(*template_parameters,
                                    template_parameter_infos,
                                    &template_scope,
                                    &parameter_failure)) {
        ostringstream out;
        out << "unsupported lambda template parameter clause";
        if(!parameter_failure.empty()) {
          out << " [reason " << parameter_failure << "]";
        }
        throw logic_error(out.str());
      }

      CppAstNode synthetic_specifiers;
      synthetic_specifiers.kind = CppAstKind::decl_specifier_seq;
      CppAstNode auto_kw;
      auto_kw.kind = CppAstKind::keyword_literal;
      auto_kw.has_token = true;
      auto_kw.token_kind = RT_SIMPLE;
      auto_kw.simple_type = KW_AUTO;
      auto_kw.value = "auto";
      synthetic_specifiers.children.push_back(auto_kw);

      CppAstNode synthetic_declarator;
      synthetic_declarator.kind = CppAstKind::declarator;
      CppAstNode identifier;
      identifier.kind = CppAstKind::identifier;
      identifier.value = "operator()";
      synthetic_declarator.children.push_back(identifier);
      if(declarator) {
        const CppAstNode * parameter_clause =
            find_child_kind(*declarator, CppAstKind::parameter_clause);
        const CppAstNode * trailing_return_type =
            find_child_kind(*declarator, CppAstKind::trailing_return_type);
        if(parameter_clause) {
          synthetic_declarator.children.push_back(*parameter_clause);
        }
        if(trailing_return_type) {
          CppAstNode trailing = *trailing_return_type;
          if(trailing.value.empty()) {
            const CppAstNode * type_id = find_child_kind(trailing, CppAstKind::type_id);
            trailing.value = type_id ? node_text(*type_id) : node_text(*trailing_return_type);
          }
          synthetic_declarator.children.push_back(trailing);
        }
      }

      vector<pair<string, TypePtr> > params_pattern;
      vector<const CppAstNode *> default_args;
      const CppAstNode * parameter_clause =
          find_child_kind(synthetic_declarator, CppAstKind::parameter_clause);
      if(parameter_clause &&
         !parse_parameter_clause(template_scope,
                                 *parameter_clause,
                                 params_pattern,
                                 &default_args,
                                 true)) {
        throw logic_error("unsupported templated lambda parameter-clause");
      }
      TypePtr type_pattern =
          make_function(result_type, vector<TypePtr>(params_pattern.size(), TypePtr()), false);
      for(size_t i = 0; i < params_pattern.size(); ++i) {
        type_pattern->params[i] = params_pattern[i].second;
      }

      unique_ptr<FunctionTemplateDecl> decl(new FunctionTemplateDecl());
      decl->declaring_scope = info->member_scope.get();
      decl->pattern_scope = &template_scope;
      decl->name = "operator()";
      decl->lexical_access_class = current_class_scope(scope);
      decl->lexical_access_function = current_function_scope(scope);
      decl->access = MA_PUBLIC;
      decl->is_const_method = !mutable_lambda;
      decl->is_lambda_call_operator_template = true;
      decl->specifiers = own_synthetic_ast(synthetic_specifiers);
      decl->declarator = own_synthetic_ast(synthetic_declarator);
      decl->declaration_node = declarator ? declarator : decl->declarator;
      decl->body = body;
      decl->parameters = template_parameter_infos;
      decl->type_pattern = type_pattern;
      decl->params_pattern = params_pattern;
      if(decl->specifiers && decl->declarator) {
        decl->result_type_pattern =
            template_api::signature::build_function_result_type_pattern(
                *decl->specifiers, *decl->declarator);
      }
      const CppAstNode * durable_parameter_clause =
          decl->declarator ?
              find_child_kind(*decl->declarator, CppAstKind::parameter_clause) :
              nullptr;
      if(durable_parameter_clause) {
        vector<pair<string, TypePtr> > ignored_params;
        vector<const CppAstNode *> durable_default_args;
        if(!parse_parameter_clause(template_scope,
                                   *durable_parameter_clause,
                                   ignored_params,
                                   &durable_default_args,
                                   true)) {
          throw logic_error("unsupported durable templated lambda parameter-clause");
        }
        decl->default_arguments_pattern =
            normalize_default_arguments(params_pattern, durable_default_args);
      } else {
        decl->default_arguments_pattern =
            normalize_default_arguments(params_pattern, default_args);
      }
      initialize_function_template_parameter_aliases(*decl);
      snapshot_function_template_debug_info(*this, *decl);
      semantic_lookup::direct_function_template_slot(*info->member_scope, decl->name)
          .push_back(decl.get());
      function_templates.push_back(std::move(decl));

      ensure_implicit_special_members(*info);
      finalize_class_virtuals(*info);
      finalize_class_layout(*info);
      info->reference_members_collected = true;
      return info;
    }

    TypePtr declared_type =
        make_function(result_type, vector<TypePtr>(params.size(), TypePtr()), false);
    for(size_t i = 0; i < params.size(); ++i) {
      declared_type->params[i] = params[i].second;
    }
    assign_lambda_itanium_metadata(*info, scope, params);

    const CppAstNode * durable_declarator =
        declarator ? own_synthetic_ast(*declarator) : nullptr;
    FunctionSemanticFlags call_operator_flags;
    call_operator_flags.access = MA_PUBLIC;
    call_operator_flags.is_const_method = !mutable_lambda;
    call_operator =
        register_class_function(*info, "operator()", declared_type, params,
                                default_arguments, body, nullptr,
                                call_operator_flags,
                                FunctionTemplateRegistrationIdentity(),
                                durable_declarator);
    call_operator->has_definition = true;
    call_operator->lexical_access_class = current_class_scope(scope);
    call_operator->lexical_access_function = current_function_scope(scope);
    synthetic_functions.push_back(call_operator);

    ensure_implicit_special_members(*info);
    finalize_class_virtuals(*info);
    finalize_class_layout(*info);
    info->reference_members_collected = true;
    return info;
  }

  ExprInfo analyze_type_trait_expression(Scope & scope,
                                         const CppAstNode & node)
  {
    return semantic_expression::analyze_type_trait_expression(*this, scope, node);
  }

  ExprInfo analyze_cast_expression(Scope & scope,
                                   const CppAstNode & node)
  {
    return semantic_expression::analyze_cast_expression(*this, scope, node);
  }

  ExprInfo analyze_lambda_expression(Scope & scope,
                                     const CppAstNode & node)
  {
    return semantic_expression::analyze_lambda_expression(*this, scope, node);
  }

  ExprInfo analyze_this_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_this_expression(*this, scope, node);
  }

  ExprInfo make_implicit_member_expression(Scope & scope,
                                           const MemberValueLookupResult & member)
  {
    return semantic_expression::make_implicit_member_expression(*this, scope, member);
  }

  ExprInfo make_address_of_expr(const ExprInfo & operand) const
  {
    ExprInfo result;
    TypePtr pointee = remove_reference_type(operand.type);
    if(!pointee) {
      pointee = operand.type;
    }
    result.type = make_pointer(pointee);
    result.category = VC_PRVALUE;
    result.node = make_dump_node(CallSemKind::unary_expression, "&");
    result.node.has_token = true;
    result.node.token_type = OP_AMP;
    set_expr_metadata(result.node, result.type, result.category);
    result.node.children.push_back(operand.node);
    return result;
  }

  CppAstNode synthetic_identifier_node(const string & name) const
  {
    CppAstNode id;
    id.kind = CppAstKind::id_expression;
    id.value = name;
    return id;
  }

  ExprInfo make_field_expr(const ExprInfo & base, const FieldInfo & field)
  {
    if(field.is_bit_field) {
      throw logic_error("bit-field lowering unsupported");
    }
    ExprInfo result;
    result.type = field.type;
    result.category = VC_LVALUE;
    result.node = make_dump_node(CallSemKind::member_expression, field.name);
    set_expr_metadata(result.node, result.type, result.category);
    set_callsem_uint_value(result.node, field.offset);
    result.node.is_reference_storage = is_reference_type(field.type);
    result.node.children.push_back(base.node);
    return result;
  }

  ExprInfo make_subscript_expr(const ExprInfo & base,
                               size_t index,
                               const TypePtr & element_type)
  {
    ExprInfo result;
    result.type = element_type;
    result.category = VC_LVALUE;
    result.node = make_subscript_expr_node(base.node,
                                           make_integer_literal_node(static_cast<long long>(index)),
                                           element_type);
    return result;
  }

  ExprInfo make_base_expr(const ExprInfo & base,
                          const ClassInfo & class_info,
                          size_t offset = 0) const
  {
    ExprInfo result;
    result.type = class_info.type;
    result.category = VC_LVALUE;
    result.node = make_dump_node(CallSemKind::member_expression, class_info.name);
    set_callsem_resolved_name(result.node, class_info.qualified_name);
    set_expr_metadata(result.node, result.type, result.category);
    set_callsem_uint_value(result.node, offset);
    result.node.is_base_subobject = true;
    result.node.children.push_back(base.node);
    return result;
  }

  ExprInfo make_base_pointer_expr(const ExprInfo & base_ptr,
                                  const ClassInfo & class_info,
                                  size_t offset = 0) const
  {
    return make_address_of_expr(make_base_expr(base_ptr, class_info, offset));
  }

  FunctionBinding * select_constructor_from_exprs(Scope & scope,
                                                  ClassInfo & info,
                                                  const vector<ExprInfo> & source_args,
                                                  vector<ExprInfo> & args_out,
                                                  vector<ConversionRank> * ranks_out = nullptr,
                                                  const ConstructorSelectionOptions & options =
                                                      ConstructorSelectionOptions())
  {
    const ConstructorSelectionOptions effective_options =
        semantic_policy::apply_analysis_policy(analysis_policy_, options);
    constructor_lifecycle_service::ConstructorSelectionResult result =
        constructor_lifecycle_service::select_constructor_from_exprs(*this,
                                                                     scope,
                                                                     info,
                                                                     source_args,
                                                                     effective_options);
    args_out = result.converted_args;
    if(ranks_out) {
      *ranks_out = result.ranks;
    }
    return result.ctor;
  }

  FunctionBinding * select_constructor(Scope & scope,
                                       ClassInfo & info,
                                       const vector<const CppAstNode *> & arg_nodes,
                                       vector<ExprInfo> & args_out,
                                       const ConstructorSelectionOptions & options =
                                           ConstructorSelectionOptions())
  {
    constructor_lifecycle_service::ConstructorSelectionResult result =
        constructor_lifecycle_service::select_constructor(*this,
                                                          scope,
                                                          info,
                                                          arg_nodes,
                                                          semantic_policy::apply_analysis_policy(
                                                              analysis_policy_,
                                                              options));
    args_out = result.converted_args;
    return result.ctor;
  }

  FunctionBinding * select_constructor_for_direct_braced_init(
      Scope & scope,
      ClassInfo & info,
      const CppAstNode & direct_braced_init,
      vector<ExprInfo> & args_out,
      const ConstructorSelectionOptions & options = ConstructorSelectionOptions())
  {
    constructor_lifecycle_service::ConstructorSelectionResult result =
        constructor_lifecycle_service::select_constructor_for_direct_braced_init(*this,
                                                                                 scope,
                                                                                 info,
                                                                                 direct_braced_init,
                                                                                 semantic_policy::apply_analysis_policy(
                                                                                     analysis_policy_,
                                                                                     options));
    args_out = result.converted_args;
    return result.ctor;
  }

  FunctionBinding * destructor_for(ClassInfo & info)
  {
    map<string, vector<FunctionBinding *> >::iterator found =
        info.methods.find(string("~") + info.name);
    if(found == info.methods.end()) {
      return nullptr;
    }
    for(size_t i = 0; i < found->second.size(); ++i) {
      if(found->second[i]->is_destructor) {
        return found->second[i];
      }
    }
    return nullptr;
  }

  FunctionBinding * copy_constructor_for(ClassInfo & info)
  {
    map<string, vector<FunctionBinding *> >::iterator found = info.methods.find(info.name);
    if(found == info.methods.end()) {
      return nullptr;
    }
    for(size_t i = 0; i < found->second.size(); ++i) {
      if(found->second[i]->is_copy_constructor) {
        return found->second[i];
      }
    }
    return nullptr;
  }

  FunctionBinding * copy_assignment_for(ClassInfo & info)
  {
    map<string, vector<FunctionBinding *> >::iterator found = info.methods.find("operator=");
    if(found == info.methods.end()) {
      return nullptr;
    }
    for(size_t i = 0; i < found->second.size(); ++i) {
      if(found->second[i]->is_copy_assignment) {
        return found->second[i];
      }
    }
    return nullptr;
  }

  FunctionBinding * move_assignment_for(ClassInfo & info)
  {
    map<string, vector<FunctionBinding *> >::iterator found = info.methods.find("operator=");
    if(found == info.methods.end()) {
      return nullptr;
    }
    for(size_t i = 0; i < found->second.size(); ++i) {
      if(found->second[i]->is_move_assignment) {
        return found->second[i];
      }
    }
    return nullptr;
  }

  bool declared_parameter_types_for_abi_output(FunctionBinding & binding,
                                               vector<TypePtr> & out)
  {
    out.clear();
    const CppAstNode * declaration =
        binding.definition_node ? binding.definition_node : binding.declaration_node;
    if(!declaration) {
      return false;
    }

    const CppAstNode * declarator = find_child_kind(*declaration, CppAstKind::declarator);
    if(!declarator) {
      return false;
    }

    const CppAstNode * parameter_clause =
        find_child_kind(*declarator, CppAstKind::parameter_clause);
    if(!parameter_clause) {
      return true;
    }

    Scope * parse_scope = binding.declaration_scope;
    if(!parse_scope && binding.owner_class) {
      parse_scope = binding.owner_class->member_scope.get();
    }
    if(!parse_scope) {
      return false;
    }

    vector<pair<string, TypePtr> > params;
    if(!parse_parameter_clause(*parse_scope, *parameter_clause, params, nullptr, true)) {
      return false;
    }

    out.reserve(params.size());
    for(size_t i = 0; i < params.size(); ++i) {
      out.push_back(params[i].second);
    }
    return true;
  }

  void require_function_parameter_abi_output(FunctionBinding * binding)
  {
    if(!binding) {
      return;
    }

    vector<TypePtr> declared_param_types;
    const bool have_declared_param_types =
        declared_parameter_types_for_abi_output(*binding, declared_param_types);
    const size_t explicit_param_offset = function_binding_explicit_parameter_offset(*binding);

    for(size_t i = 0; i < binding->params.size(); ++i) {
      if(binding->params[i].first == "this") {
        continue;
      }

      TypePtr param_type = binding->params[i].second;
      const size_t explicit_index = i - explicit_param_offset;
      if(have_declared_param_types && explicit_index < declared_param_types.size()) {
        param_type = declared_param_types[explicit_index];
      }
      if(!param_type || is_reference_type(param_type)) {
        continue;
      }

      ClassInfo * info = complete_class_type(param_type);
      if(!info || !info->complete) {
        continue;
      }

      if(FunctionBinding * copy = copy_constructor_for(*info)) {
        if(!copy->synthesized) {
          require_function_definition(copy, OutputReason::RuntimeDependency);
        }
      }
    }
  }

  bool node_designates_base_subobject(const CallSemNode & node) const
  {
    if(node.is_base_subobject) {
      return true;
    }
    for(size_t i = 0; i < node.children.size(); ++i) {
      if(node_designates_base_subobject(node.children[i])) {
        return true;
      }
    }
    return false;
  }

  const CallSemNode * peel_base_subobject_root(const CallSemNode & node) const
  {
    const CallSemNode * current = &node;
    while(current) {
      if(current->kind == CallSemKind::unary_expression &&
         current->children.size() == 1 &&
         (callsem_has_token(*current, OP_AMP) ||
          callsem_has_token(*current, OP_STAR))) {
        current = &current->children[0];
        continue;
      }
      if(current->kind == CallSemKind::member_expression &&
         current->is_base_subobject &&
         current->children.size() == 1) {
        current = &current->children[0];
        continue;
      }
      return current;
    }
    return nullptr;
  }

  symbol_linkage::SpecialMemberEntryPointKind direct_call_entry_point_kind(
      const FunctionBinding & target,
      const vector<ExprInfo> & args) const
  {
    if(!(target.is_constructor || target.is_destructor) ||
       !target.owner_class ||
       !semantic_class_model::class_has_virtual_bases(*target.owner_class) ||
       args.empty()) {
      return symbol_linkage::SMEK_COMPLETE;
    }
    return node_designates_base_subobject(args[0].node) ?
               symbol_linkage::SMEK_BASE :
               symbol_linkage::SMEK_COMPLETE;
  }

  symbol_linkage::SymbolIdentity direct_call_symbol(
      const FunctionBinding & target,
      symbol_linkage::SpecialMemberEntryPointKind entry_point_kind)
  {
    if(!target.is_constructor && !target.is_destructor) {
      return target.symbol;
    }
    if(entry_point_kind == symbol_linkage::SMEK_COMPLETE &&
       !target.symbol.object_symbol.empty()) {
      return target.symbol;
    }
    std::string entry_object_symbol;
    if(!target.symbol.object_symbol.empty() &&
       symbol_linkage::special_member_entry_point_object_symbol_from_complete_object_symbol(
           target.symbol.object_symbol,
           target.is_constructor,
           entry_point_kind,
           entry_object_symbol)) {
      symbol_linkage::SymbolIdentity updated = target.symbol;
      updated.object_symbol = entry_object_symbol;
      const string base_internal_symbol =
          target.symbol.internal_symbol.empty() ?
              symbol_linkage::internal_symbol_from_name(target.name) :
              target.symbol.internal_symbol;
      updated.internal_symbol =
          entry_point_kind == symbol_linkage::SMEK_BASE ?
              base_internal_symbol + "__base_entry" :
          entry_point_kind == symbol_linkage::SMEK_DELETING ?
              base_internal_symbol + "__deleting_entry" :
              base_internal_symbol;
      updated.keep_internal_alias = target.symbol.keep_internal_alias;
      updated.prefer_local_object_binding = target.symbol.prefer_local_object_binding;
      return updated;
    }
    symbol_linkage::FunctionSymbolOptions options;
    populate_function_symbol_options(options,
                                     target.is_const_method,
                                     target.is_volatile_method,
                                     target.ref_qualifier,
                                     target.is_constructor,
                                     target.is_destructor,
                                     function_template_registration_identity(target),
                                     target.owner_class,
                                     target.declaration_scope,
                                     target.declaration_node,
                                     target.definition_node);
    if(!target.instantiation_pack_sizes.empty()) {
      options.template_argument_pack_sizes = &target.instantiation_pack_sizes;
    }
    options.abi_tags = function_binding_abi_tags(target);
    options.special_member_entry_point_kind = entry_point_kind;
    const string qualified_name =
        function_binding_qualified_name_for_symbol(target);
    QualifiedName qualified_name_syntax;
    const bool has_qualified_name_syntax =
        function_binding_qualified_name_syntax_for_symbol(target,
                                                          qualified_name_syntax);
    if(!has_qualified_name_syntax && !target.is_c_linkage) {
      throw logic_error("missing qualified-name syntax for function entry symbol " +
                        qualified_name);
    }
    const string object_symbol_key =
        function_object_symbol_key(
            qualified_name,
            target.type,
            target.is_const_method,
            target.is_volatile_method,
            target.ref_qualifier,
            target.is_constructor,
            target.is_destructor,
            function_template_registration_identity(target),
            target.owner_class);
    symbol_linkage::SymbolIdentity updated =
        has_qualified_name_syntax ?
            symbol_linkage::make_function_symbol_identity(
                qualified_name_syntax,
                function_binding_display_name_for_symbol(target),
                target.is_c_linkage,
                target.type,
                options,
                object_symbol_key,
                target.symbol.linkage) :
            symbol_linkage::make_c_function_symbol_identity(
                function_binding_display_name_for_symbol(target),
                target.symbol.linkage);
    const string base_internal_symbol =
        target.symbol.internal_symbol.empty() ?
            symbol_linkage::internal_symbol_from_name(target.name) :
            target.symbol.internal_symbol;
    updated.internal_symbol =
        entry_point_kind == symbol_linkage::SMEK_BASE ?
            base_internal_symbol + "__base_entry" :
            base_internal_symbol;
    updated.keep_internal_alias = target.symbol.keep_internal_alias;
    updated.prefer_local_object_binding = target.symbol.prefer_local_object_binding;
    return updated;
  }

  void append_callee_virtual_base_layout(DumpNode & callee,
                                         const FunctionBinding & target,
                                         const vector<ExprInfo> & args) const
  {
    if(!target.owner_class) {
      return;
    }

    const auto find_actual_subobject_offset =
        [&](const ClassInfo & object_class,
            const ClassInfo * target_class,
            size_t & out_offset) -> bool
        {
          if(!target_class) {
            return false;
          }

          bool found = false;
          for(size_t i = 0; i < object_class.complete_subobjects.size(); ++i) {
            const SubobjectInfo & subobject = object_class.complete_subobjects[i];
            if(!subobject.type) {
              continue;
            }
            if(subobject.type != target_class &&
               subobject.type->qualified_name != target_class->qualified_name) {
              continue;
            }
            if(found && out_offset != subobject.offset) {
              throw logic_error("ambiguous complete subobject offset for " +
                                target_class->qualified_name);
            }
            out_offset = subobject.offset;
            found = true;
          }
          if(found) {
            return true;
          }

          MemberAccess access = MA_PUBLIC;
          return find_unique_base_path(object_class, target_class, out_offset, access);
        };

    ClassInfo * object_class = nullptr;
    if(target.is_method &&
       !target.is_constructor &&
       !target.is_destructor &&
       !args.empty()) {
      const CallSemNode * root_object = peel_base_subobject_root(args[0].node);
      TypePtr root_object_type =
          (root_object && root_object->semantic_type) ? root_object->semantic_type : args[0].type;
      TypePtr object_type =
          root_object_type ? strip_top_level_cv(remove_reference_type(root_object_type)) : TypePtr();
      if(object_type && object_type->kind == Type::TK_POINTER) {
        object_type = strip_top_level_cv(object_type->inner);
      }
      if(object_type) {
        object_class = class_info_for_type(object_type);
      }
    }

    for(size_t i = 0; i < target.owner_class->virtual_base_subobjects.size(); ++i) {
      const SubobjectInfo & subobject = target.owner_class->virtual_base_subobjects[i];
      if(!subobject.type) {
        continue;
      }

      size_t actual_offset = subobject.offset;
      if(object_class) {
        size_t actual_path_offset = 0;
        if(find_actual_subobject_offset(*object_class, subobject.type, actual_path_offset)) {
          actual_offset = actual_path_offset;
        }
      }

      mutable_callsem_virtual_base_layout(callee).push_back(
          make_pair(subobject.type->qualified_name, actual_offset));
    }
  }

  DumpNode build_direct_call_node(FunctionBinding & emitted_function,
                                  const vector<ExprInfo> & args,
                                  TypePtr result_type,
                                  ValueCategory category)
  {
    const symbol_linkage::SpecialMemberEntryPointKind entry_point_kind =
        direct_call_entry_point_kind(emitted_function, args);
    const symbol_linkage::SymbolIdentity call_symbol =
        direct_call_symbol(emitted_function, entry_point_kind);
    DumpNode call = make_dump_node(CallSemKind::call_expression);
    set_expr_metadata(call, expression_type_for_function_result(result_type), category);
    call.semantic_type = result_type;

    DumpNode callee = make_dump_node(CallSemKind::callee, emitted_function.name);
    set_callsem_resolved_name(callee, function_output_name(emitted_function));
    callee.semantic_type = emitted_function.type;
    callee.is_c_linkage = emitted_function.is_c_linkage;
    if(emitted_function.is_constructor || emitted_function.is_destructor) {
      callee.has_special_member_entry_point_kind = true;
      callee.special_member_entry_point_kind = entry_point_kind;
    }
    append_callee_virtual_base_layout(callee, emitted_function, args);
    set_callsem_runtime_bridge_symbol(
        callee,
        runtime_bridge_symbol_for_bound_function(
            emitted_function.name,
            emitted_function.owner_class ? emitted_function.owner_class->qualified_name : "",
            emitted_function.type));
    set_dump_symbol(callee, call_symbol);
    TypePtr result_base = strip_top_level_cv(remove_reference_type(result_type));
    ClassInfo * result_class_info = result_base ? class_info_for_type(result_base) : nullptr;
    const string result_class =
        result_class_info ? result_class_info->qualified_name : "";
    if(!callsem_virtual_base_layout(callee).empty() &&
       emitted_function.owner_class &&
       !result_class.empty() &&
       result_class == emitted_function.owner_class->qualified_name) {
      set_callsem_virtual_base_layout(call, callsem_virtual_base_layout(callee));
    }
    if(callsem_virtual_base_layout(call).empty() &&
       !result_class.empty() &&
       emitted_function.type &&
       emitted_function.type->kind == Type::TK_FUNCTION) {
      const auto try_copy_layout_from_arg =
          [&](const ExprInfo & arg) -> bool
          {
            TypePtr arg_base = strip_top_level_cv(remove_reference_type(arg.type));
            if(arg_base && arg_base->kind == Type::TK_POINTER) {
              arg_base = strip_top_level_cv(arg_base->inner);
            }
            ClassInfo * arg_class_info =
                arg_base ? class_info_for_type(arg_base) : nullptr;
            if(!arg_class_info ||
               arg_class_info->qualified_name != result_class) {
              return false;
            }

            if(!callsem_virtual_base_layout(arg.node).empty()) {
              set_callsem_virtual_base_layout(call, callsem_virtual_base_layout(arg.node));
              return true;
            }

            const CallSemNode * root = peel_base_subobject_root(arg.node);
            if(root && !callsem_virtual_base_layout(*root).empty()) {
              set_callsem_virtual_base_layout(call, callsem_virtual_base_layout(*root));
              return true;
            }
            return false;
          };

      for(size_t i = 0; i < args.size(); ++i) {
        if(try_copy_layout_from_arg(args[i])) {
          break;
        }
      }
    }
    call.children.push_back(callee);
    for(size_t i = 0; i < args.size(); ++i) {
      call.children.push_back(args[i].node);
    }
    return call;
  }

  DumpNode make_direct_call_node(FunctionBinding & function,
                                 const vector<ExprInfo> & args)
  {
    CallableEmissionDecision emission =
        decide_callable_emission(&function, OutputReason::DirectCall);
    require_function_definition(&function,
                                OutputReason::DirectCall,
                                emission.mark_output_required_now);
    FunctionBinding * resolved_function =
        semantic_output::resolve_output_function_binding(*this, &function);
    FunctionBinding & emitted_function = resolved_function ? *resolved_function : function;
    TypePtr function_type = strip_top_level_cv(emitted_function.type);
    ValueCategory category = VC_PRVALUE;
    if(!result_value_category_for_function_result(function_type->inner, category)) {
      throw logic_error("invalid direct call result");
    }
    return build_direct_call_node(emitted_function, args, function_type->inner, category);
  }

  ExprInfo make_direct_call_expr(FunctionBinding & function,
                                 const vector<ExprInfo> & args,
                                 bool mark_output_required = true)
  {
    ExprInfo result;
    TypePtr function_type = strip_top_level_cv(function.type);
    if(!function_type || function_type->kind != Type::TK_FUNCTION) {
      throw logic_error("direct call expression requires function type");
    }
    if(!result_value_category_for_function_result(function_type->inner, result.category)) {
      throw logic_error("invalid direct call expression result");
    }
    result.type = expression_type_for_function_result(function_type->inner);
    if(!mark_output_required) {
      FunctionBinding * resolved_function =
          semantic_output::resolve_output_function_binding(*this, &function);
      FunctionBinding & emitted_function = resolved_function ? *resolved_function : function;
      TypePtr emitted_function_type = strip_top_level_cv(emitted_function.type);
      result.node =
          build_direct_call_node(emitted_function,
                                 args,
                                 emitted_function_type->inner,
                                 result.category);
      return result;
    }
    result.node = make_direct_call_node(function, args);
    return result;
  }

  void append_wrapped_action(DumpNode & out,
                             CallSemKind kind,
                             const string & text,
                             const DumpNode & call_node)
  {
    DumpNode wrapper = make_dump_node(kind, text);
    wrapper.children.push_back(call_node);
    out.children.push_back(wrapper);
  }

  void append_constructor_generated_statements(Scope & scope,
                                               FunctionBinding & binding,
                                               symbol_linkage::SpecialMemberEntryPointKind entry_point_kind,
                                               DumpNode & function_node)
  {
    semantic_lifetime::append_constructor_generated_statements(*this,
                                                               scope,
                                                               binding,
                                                               entry_point_kind,
                                                               function_node);
  }

  void append_copy_assignment_generated_statements(Scope & scope,
                                                   FunctionBinding & binding,
                                                   DumpNode & function_node)
  {
    semantic_lifetime::append_copy_assignment_generated_statements(*this,
                                                                   scope,
                                                                   binding,
                                                                   function_node);
  }

  void append_destructor_generated_statements(Scope & scope,
                                              FunctionBinding & binding,
                                              symbol_linkage::SpecialMemberEntryPointKind entry_point_kind,
                                              DumpNode & function_node)
  {
    semantic_lifetime::append_destructor_generated_statements(*this,
                                                              scope,
                                                              binding,
                                                              entry_point_kind,
                                                              function_node);
  }

  ExprInfo analyze_member_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_member_expression(*this, scope, node);
  }

  ExprInfo analyze_literal(const CppAstNode & node)
  {
    return semantic_expression::analyze_literal(*this, node);
  }

  ExprInfo analyze_id_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_id_expression(*this, scope, node);
  }

  ExprInfo analyze_unary_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_unary_expression(*this, scope, node);
  }

  ExprInfo analyze_postfix_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_postfix_expression(*this, scope, node);
  }

  ExprInfo analyze_conditional_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_conditional_expression(*this, scope, node);
  }

  ExprInfo analyze_binary_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_binary_expression(*this, scope, node);
  }

  ExprInfo analyze_subscript_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_subscript_expression(*this, scope, node);
  }

  ExprInfo analyze_sizeof_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_sizeof_expression(*this, scope, node);
  }

  ExprInfo analyze_sizeof_pack_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_sizeof_pack_expression(*this, scope, node);
  }

  bool try_analyze_declval_call_expression(Scope & scope,
                                           const CppAstNode & node,
                                           const std::string & use_location,
                                           ExprInfo & out)
  {
    if(node.children.empty() || node.children[0].kind != CppAstKind::id_expression) {
      return false;
    }

    const CppAstNode * argument_list = cpp_decl::find_child(node, CppAstKind::argument_list);
    if(!argument_list) {
      argument_list = cpp_decl::find_child(node, CppAstKind::paren_argument_list);
    }
    if(!argument_list || !argument_list->children.empty()) {
      return false;
    }

    const TemplateIdSyntax * template_id = cppast_template_id_syntax(node.children[0]);
    if(!template_id ||
       template_id->name.name != "declval" ||
       template_id->arguments.size() != 1) {
      return false;
    }

    TypePtr declval_type;
    if(!template_api::type::resolve_type_argument_text(*this,
                                                      scope,
                                                      template_id->arguments[0],
                                                      declval_type) ||
       !declval_type) {
      return false;
    }

    if(witness::enabled(template_witness_context()) &&
       template_api::template_witness_declval_call_source_capture_enabled() &&
       !type_depends_on_template_parameter(declval_type)) {
      std::string public_location =
          template_api::normalize_template_witness_source_location(use_location);
      const std::vector<WitnessUseLocationSearch> declval_location_searches = {
        WitnessUseLocationSearch(WITNESS_USE_LOCATION_NAME_IN_SUBTREE, &node)
      };
      const std::string node_declval_location =
          select_witness_use_location(declval_location_searches, "declval");
      if(!node_declval_location.empty() &&
         source_location_identifier_followed_by(node_declval_location, "declval", '<')) {
        public_location = node_declval_location;
      }
      if(!public_location.empty() &&
         !source_location_points_at_identifier(public_location, "declval")) {
        const std::string same_line_declval_location =
            template_api::template_witness_detail::
                source_location_for_identifier_token_on_or_after(
                    template_witness_context(),
                    public_location,
                    "declval",
                    true);
        if(!same_line_declval_location.empty()) {
          public_location =
              template_api::normalize_template_witness_source_location(
                  same_line_declval_location);
        }
      }
      if(public_location.empty()) {
        public_location = template_api::normalize_template_witness_source_location(
            template_public_use_location_or(std::string()));
      }
      if(!source_location_points_at_identifier(public_location, "declval")) {
        public_location.clear();
      }
      if(!source_location_identifier_followed_by(public_location, "declval", '<')) {
        public_location.clear();
      }
      if(!public_location.empty()) {
        witness::FunctionCallSourceDecision decision;
        decision.origin = witness::FunctionCallEmissionOrigin::DeclvalCall;
        witness::set_use_anchor(decision.location,
                                decision.use_anchor,
                                public_location);
        decision.template_name = "declval";
        decision.selected = "declval";
        decision.selection = witness::SourceSelectionKind::Instantiation;

        witness::TemplateWitnessSourceBinding binding;
        binding.param = "$1";
        binding.arg = template_api::type::lookup_text_for_type_argument(*this,
                                                                        declval_type);
        if(binding.arg.empty()) {
          binding.arg = template_id->arguments[0];
        }
        binding.source = "explicit";
        binding.type_like = true;
        decision.bindings.push_back(binding);

        witness::emit_function_call(template_witness_context(),
                                    decision);
      }
    }

    TypePtr result_type = is_void_type(strip_top_level_cv(declval_type)) ?
                              declval_type :
                              collapse_rvalue_reference_type(declval_type);
    ValueCategory category = VC_PRVALUE;
    if(!result_value_category_for_function_result(result_type, category)) {
      category = VC_PRVALUE;
    }

    out = ExprInfo();
    out.node = make_dump_node(CallSemKind::call_expression);
    set_expr_info_metadata(out, result_type, category);

    CallSemNode callee = make_dump_node(CallSemKind::callee, node.children[0].value);
    callee.semantic_type = make_function(result_type, vector<TypePtr>(), false);
    callee.is_declval_callee = true;
    set_expr_metadata(callee, callee.semantic_type, VC_PRVALUE);
    out.node.children.push_back(callee);
    return true;
  }

  ExprInfo analyze_assignment_expression(Scope & scope, const CppAstNode & node)
  {
    return semantic_expression::analyze_assignment_expression(*this, scope, node);
  }

  ExprInfo analyze_call_expression(Scope & scope,
                                   const CppAstNode & node,
                                   const semantic_overload::CallAnalysisOptions & options =
                                       semantic_overload::CallAnalysisOptions())
  {
    const semantic_overload::CallAnalysisOptions effective_options =
        semantic_policy::apply_analysis_policy(analysis_policy_, options);
    ExprInfo declval_expr;
    std::string declval_use_location =
        effective_options.hints ? effective_options.hints->use_location : string();
    if(declval_use_location.empty()) {
      declval_use_location = template_public_use_location_or(std::string());
    }
    if(try_analyze_declval_call_expression(scope,
                                           node,
                                           declval_use_location,
                                           declval_expr)) {
      return declval_expr;
    }
    return semantic_overload::analyze_call_expression(*this,
                                                      scope,
                                                      node,
                                                      effective_options);
  }

  void analyze_initializer(Scope & scope,
                           const TypePtr & type,
                           const CppAstNode & node,
                           DumpNode & out)
  {
    semantic_lifetime::analyze_initializer(*this, scope, type, node, out);
  }

  DumpNode make_integer_literal_node(long long value)
  {
    DumpNode node = make_dump_node(CallSemKind::literal, to_string(value));
    node.semantic_type = make_fundamental(FT_INT);
    node.value_category = CVC_PRVALUE;
    if(value >= 0) {
      set_callsem_uint_value(node, static_cast<unsigned long long>(value));
    }
    return node;
  }

  ExprInfo make_value_initialized_expr(const TypePtr & type)
  {
    return semantic_expression::make_value_initialized_expr(type);
  }

  DumpNode make_id_expr_node(const string & name, const TypePtr & type)
  {
    DumpNode node = make_dump_node(CallSemKind::id_expression, name);
    set_expr_metadata(node, type, VC_LVALUE);
    return node;
  }

  DumpNode make_binary_expr_node(ETokenType token_type,
                                 const string & text,
                                 const TypePtr & type,
                                 const DumpNode & lhs,
                                 const DumpNode & rhs)
  {
    DumpNode node = make_dump_node(CallSemKind::binary_expression, text);
    node.has_token = true;
    node.token_type = token_type;
    set_expr_metadata(node, type, VC_PRVALUE);
    node.children.push_back(lhs);
    node.children.push_back(rhs);
    return node;
  }

  DumpNode make_postfix_expr_node(ETokenType token_type,
                                  const string & text,
                                  const TypePtr & type,
                                  const DumpNode & child)
  {
    DumpNode node = make_dump_node(CallSemKind::postfix_expression, text);
    node.has_token = true;
    node.token_type = token_type;
    set_expr_metadata(node, type, VC_PRVALUE);
    node.children.push_back(child);
    return node;
  }

  DumpNode make_subscript_expr_node(const DumpNode & base,
                                    const DumpNode & index,
                                    const TypePtr & element_type)
  {
    DumpNode node = make_dump_node(CallSemKind::subscript_expression);
    set_expr_metadata(node, element_type, VC_LVALUE);
    node.children.push_back(base);
    node.children.push_back(index);
    return node;
  }

  void analyze_range_for_statement(Scope & scope,
                                   const TypePtr & return_type,
                                   const CppAstNode & node,
                                   DumpNode & out)
  {
    if(node.kind != CppAstKind::range_for_statement || node.children.size() != 3) {
      throw logic_error("malformed range-for statement");
    }

    const CppAstNode & range_decl = node.children[0];
    const CppAstNode & range_init = node.children[1];
    const CppAstNode & body_ast = node.children[2];
    const CppAstNode * specifiers = find_child_kind(range_decl, CppAstKind::decl_specifier_seq);
    const CppAstNode * declarator = find_child_kind(range_decl, CppAstKind::declarator);
    if(!specifiers || !declarator || range_init.children.size() != 1) {
      throw logic_error("unsupported range-for declaration");
    }

    ExprInfo range_expr = range_init.children[0].kind == CppAstKind::braced_init_list ?
        analyze_braced_init_list_expression(scope, range_init.children[0]) :
        analyze_expression(scope, range_init.children[0]);
    TypePtr range_type = strip_top_level_cv(range_expr.type);
    TypePtr element_type;
    ClassInfo * range_initlist_info = nullptr;
    bool range_is_initializer_list =
        is_initializer_list_type(range_expr.type, &element_type, &range_initlist_info);
    if(!range_type ||
       ((range_type->kind != Type::TK_ARRAY || !range_type->has_bound) &&
        !range_is_initializer_list)) {
      throw logic_error("range-for requires bounded array or std::initializer_list");
    }

    Scope body_scope(&scope);
    DumpNode outer = make_dump_node(CallSemKind::compound_statement);
    DumpNode range_base = range_expr.node;
    if(range_init.children[0].kind == CppAstKind::braced_init_list ||
       range_expr.category != VC_LVALUE ||
       range_expr.node.kind != CallSemKind::id_expression) {
      const string hidden_range = next_synthetic_local_name("range");
      body_scope.values[hidden_range] =
          ValueBinding(ValueBinding::VK_VARIABLE, hidden_range, range_expr.type);
      DumpNode hidden_var = make_dump_node(CallSemKind::variable, hidden_range);
      hidden_var.semantic_type = range_expr.type;
      hidden_var.children.push_back(range_expr.node);
      DumpNode hidden_decl = make_dump_node(CallSemKind::simple_declaration);
      hidden_decl.children.push_back(hidden_var);
      outer.children.push_back(hidden_decl);
      range_base = make_id_expr_node(hidden_range, range_expr.type);
    }

    string loop_name;
    TypePtr loop_type;
    bool is_typedef = false;
    if(!range_is_initializer_list) {
      element_type = range_type->inner;
    }
    if(decl_spec_contains_auto(*specifiers)) {
      ExprInfo pseudo_element;
      pseudo_element.type = element_type;
      pseudo_element.category = VC_LVALUE;
      if(!parse_auto_declaration_type_from_expr(body_scope,
                                                *specifiers,
                                                *declarator,
                                                pseudo_element,
                                                loop_name,
                                                loop_type)) {
        throw logic_error("unsupported range declaration");
      }
    } else if(!parse_variable_declaration_type(body_scope, *specifiers, *declarator, nullptr,
                                               false, loop_name, loop_type, is_typedef) ||
              is_typedef) {
      throw logic_error("unsupported range declaration");
    }

    if(loop_name.empty()) {
      throw logic_error("unsupported range declaration type");
    }
    body_scope.values[loop_name] = ValueBinding(ValueBinding::VK_VARIABLE, loop_name, loop_type);

    const string index_name = next_synthetic_local_name("idx");
    DumpNode idx_decl = make_dump_node(CallSemKind::simple_declaration);
    DumpNode idx_var = make_dump_node(CallSemKind::variable, index_name);
    idx_var.semantic_type = make_fundamental(FT_INT);
    idx_var.children.push_back(make_integer_literal_node(0));
    idx_decl.children.push_back(idx_var);

    DumpNode for_node = make_dump_node(CallSemKind::for_statement);
    DumpNode init_node = make_dump_node(CallSemKind::for_init_statement);
    init_node.children.push_back(idx_decl);
    for_node.children.push_back(init_node);

    DumpNode idx_id = make_id_expr_node(index_name, make_fundamental(FT_INT));
    DumpNode bound_expr;
    if(range_is_initializer_list) {
      ExprInfo range_base_expr;
      range_base_expr.type = range_expr.type;
      range_base_expr.category = VC_LVALUE;
      range_base_expr.node = range_base;
      bound_expr = make_field_expr(range_base_expr, range_initlist_info->fields[1]).node;
    } else {
      bound_expr =
          make_integer_literal_node(static_cast<long long>(range_type->bound));
    }
    DumpNode cond_expr =
        make_binary_expr_node(OP_LT, "<", make_fundamental(FT_INT), idx_id, bound_expr);
    DumpNode cond_node = make_dump_node(CallSemKind::condition);
    cond_node.children.push_back(cond_expr);
    for_node.children.push_back(cond_node);

    DumpNode iter_node = make_dump_node(CallSemKind::iteration);
    iter_node.children.push_back(
        make_postfix_expr_node(OP_INC, "++", make_fundamental(FT_INT), idx_id));
    for_node.children.push_back(iter_node);

    DumpNode body_node = make_dump_node(CallSemKind::compound_statement);
    DumpNode loop_var_decl = make_dump_node(CallSemKind::simple_declaration);
    DumpNode loop_var = make_dump_node(CallSemKind::variable, loop_name);
    loop_var.semantic_type = loop_type;
    DumpNode element_base = range_base;
    if(range_is_initializer_list) {
      ExprInfo range_base_expr;
      range_base_expr.type = range_expr.type;
      range_base_expr.category = VC_LVALUE;
      range_base_expr.node = range_base;
      element_base = make_field_expr(range_base_expr, range_initlist_info->fields[0]).node;
    }
    loop_var.children.push_back(
        make_subscript_expr_node(element_base,
                                 make_id_expr_node(index_name, make_fundamental(FT_INT)),
                                 element_type));
    loop_var_decl.children.push_back(loop_var);
    body_node.children.push_back(loop_var_decl);
    analyze_statement(body_scope, return_type, body_ast, body_node);
    for_node.children.push_back(body_node);

    outer.children.push_back(for_node);
    out.children.push_back(outer);
  }

  ClassInfo * complete_class_type(const TypePtr & type)
  {
    note_performance_counter(&semantic_metrics::AnalyzerCounters::complete_class_type_calls);
    TypePtr base_type = strip_top_level_cv(type);
    if(!base_type || base_type->kind != Type::TK_NAMED) {
      note_performance_counter(&semantic_metrics::AnalyzerCounters::complete_class_type_no_class);
      return nullptr;
    }
    if(named_type_has_dependent_semantic(base_type) ||
       named_type_key_contains_dependent_semantic(base_type) ||
       base_type->named_key.compare(0, 5, "enum ") == 0) {
      note_performance_counter(&semantic_metrics::AnalyzerCounters::complete_class_type_no_class);
      return nullptr;
    }
    const auto sync_query_type_layout =
        [&base_type](const ClassInfo * completed)
    {
      if(!completed ||
         !completed->type ||
         !base_type ||
         base_type->kind != Type::TK_NAMED ||
         base_type->named_key != completed->type->named_key) {
        return;
      }
      base_type->named_complete = completed->type->named_complete;
      base_type->named_has_layout = completed->type->named_has_layout;
      base_type->named_alignment = completed->type->named_alignment;
      base_type->named_size = completed->type->named_size;
      base_type->named_is_empty = completed->type->named_is_empty;
      base_type->named_host_abi_chunks = completed->type->named_host_abi_chunks;
      base_type->named_itanium_abi_encoding =
          completed->type->named_itanium_abi_encoding;
      base_type->named_itanium_abi_substitution_keys =
          completed->type->named_itanium_abi_substitution_keys;
      base_type->named_itanium_abi_context_encoding =
          completed->type->named_itanium_abi_context_encoding;
      base_type->named_itanium_abi_context_substitution_keys =
          completed->type->named_itanium_abi_context_substitution_keys;
      base_type->named_itanium_abi_signature_parameter_types =
          completed->type->named_itanium_abi_signature_parameter_types;
      base_type->named_itanium_abi_discriminator =
          completed->type->named_itanium_abi_discriminator;
      base_type->named_class_template_specialization_mangle_info =
          completed->type->named_class_template_specialization_mangle_info;
    };

    ClassInfo * info = class_info_for_type(type);
    if(semantic_hotspot::enabled()) {
      std::ostringstream query;
      query << (type ? describe_type(type) : string("<null-type>"));
      if(info) {
        query << " qualified=" << info->qualified_name
              << " complete=" << (info->complete ? "yes" : "no")
              << " ref=" << (info->reference_members_collected ? "yes" : "no")
              << " in_progress=" << (info->template_instantiation_in_progress ? "yes" : "no")
              << " full_collect="
              << (info->full_member_collection_in_progress ? "yes" : "no")
              << " ref_collect="
              << (info->reference_member_collection_in_progress ? "yes" : "no");
      } else {
        query << " qualified=<none>";
      }
      semantic_hotspot::note_semantic_query("complete_class_type", query.str());
    }
    if(!info) {
      note_performance_counter(&semantic_metrics::AnalyzerCounters::complete_class_type_no_class);
      return nullptr;
    }
    if(info->complete) {
      note_performance_counter(
          &semantic_metrics::AnalyzerCounters::complete_class_type_already_complete);
      sync_query_type_layout(info);
      return info;
    }
    DIAG_CONTEXT("complete_class_type [" +
                 (type ? describe_type(type) : string("<null-type>")) + "]");
    if(info->full_member_collection_in_progress ||
       info->reference_member_collection_in_progress) {
      note_performance_counter(
          &semantic_metrics::AnalyzerCounters::complete_class_type_in_progress);
      return nullptr;
    }
    if(!info->source_template &&
       info->enclosing_scope &&
       info->enclosing_scope->class_info &&
       info->enclosing_scope->class_info->source_template) {
      ClassInfo * owner = info->enclosing_scope->class_info;
      const string nested_name = info->name;
      if(owner) {
        ClassInfo * completed_owner = owner->complete ? owner : nullptr;
        if(!completed_owner &&
           !owner->template_instantiation_in_progress &&
           !owner->full_member_collection_in_progress &&
           !owner->reference_member_collection_in_progress &&
           owner->type) {
          completed_owner = complete_class_type(owner->type);
        }
        if(completed_owner && completed_owner->member_scope) {
          ClassInfo * refreshed = class_info_for_type(type);
          if((!refreshed || refreshed == info) && !nested_name.empty()) {
            map<string, TypePtr>::const_iterator found =
                completed_owner->member_scope->named_types.find(nested_name);
            if(found != completed_owner->member_scope->named_types.end()) {
              refreshed = class_info_for_type(found->second);
            }
          }
          if(refreshed && refreshed != info) {
            info = refreshed;
            if(info->complete) {
              note_performance_counter(
                  &semantic_metrics::AnalyzerCounters::complete_class_type_already_complete);
              sync_query_type_layout(info);
              return info;
            }
            if(info->full_member_collection_in_progress ||
               info->reference_member_collection_in_progress) {
              note_performance_counter(
                  &semantic_metrics::AnalyzerCounters::complete_class_type_in_progress);
              return nullptr;
            }
          }
        }
      }
    }
    template_api::refresh_forward_class_template_selection(*this, *info);
    if(template_api::class_template_completion_has_owner_definition(*info)) {
      template_api::TemplateNestedMemberClassCompletionRequest request;
      request.nested_info = info;
      const template_api::TemplateNestedMemberClassCompletionResult result =
          template_api::complete_nested_member_class_from_owner_definition(*this,
                                                                          request);
      if(result.completed) {
        note_performance_counter(
            &semantic_metrics::AnalyzerCounters::complete_class_type_materializations);
        return info;
      }
    }
    if(!template_api::class_has_source_template_identity(info) &&
       info->class_node &&
       info->class_node->kind != CppAstKind::class_forward_declaration) {
      note_performance_counter(
          &semantic_metrics::AnalyzerCounters::complete_class_type_materializations);
      populate_class_info(*info, *info->class_node);
      if(info->complete &&
         template_api::class_template_completion_has_owner_definition(*info)) {
        template_api::finalize_nested_member_class_instantiation_from_owner(*this,
                                                                           info,
                                                                           true);
      }
      sync_query_type_layout(info);
      return info->complete ? info : nullptr;
    }
    const bool incomplete_instantiation_has_member_state =
        info->reference_members_collected ||
        info->implicit_special_members_ensured ||
        !info->fields.empty() ||
        !info->methods.empty() ||
        !info->method_declaration_order.empty() ||
        !info->bases.empty() ||
        !info->anonymous_member_classes.empty() ||
        (info->member_scope &&
         (!info->member_scope->values.empty() ||
          !info->member_scope->function_sets.empty() ||
          !info->member_scope->function_templates.empty() ||
          !info->member_scope->alias_templates.empty() ||
          !info->member_scope->variable_templates.empty()));
    if(incomplete_instantiation_has_member_state &&
       !info->complete &&
       !info->dependent_instantiation &&
       info->member_scope) {
      template_api::ClassTemplateUseInfo use_info;
      if(template_api::class_template_use_info_for_class(*this,
                                                         *info->member_scope,
                                                         info,
                                                         use_info,
                                                         true) &&
         use_info.has_selection &&
         use_info.selection.class_node &&
         use_info.selection.parameters) {
        reset_instantiated_class_info(*info,
                                      use_info.origin ? use_info.origin->name : info->name,
                                      use_info.selection.class_node);
        info->is_explicit_specialization =
            use_info.selection.kind == template_api::MS_EXPLICIT_SPECIALIZATION;
        template_api::binding::bind_template_arguments_into_scope(
            *this,
            *info->member_scope,
            *use_info.selection.parameters,
            use_info.selection.arguments,
            &use_info.selection.pack_sizes);
      }
    }
    const template_api::ClassTemplateCompletionPlan completion =
        template_api::class_template_completion_plan(*info);
    if(!completion.ready) {
      if(completion.in_progress) {
        note_performance_counter(
            &semantic_metrics::AnalyzerCounters::complete_class_type_in_progress);
      }
      template_audit::log_upgrade_fail(type ? describe_type(type) : string("<null-type>"),
                                       info,
                                       "complete_class_type");
      return nullptr;
    }

    track_instantiated_class(info);
    promote_referenced_class_template_instantiation(*info);

    template_audit::log_upgrade(*info, "complete_class_type");
    info->template_instantiation_in_progress = true;
    note_performance_counter(
        &semantic_metrics::AnalyzerCounters::complete_class_type_materializations);
    try {
      semantic_trace::append_template_context(
          [&]()
          {
            populate_class_info(*info, *completion.output_node);
          },
          [&]() -> string
          {
            ostringstream out;
            out << " [completing referenced class " << info->qualified_name << "]";
            out << completion.trace_suffix;
            return out.str();
          });
      finalize_instantiated_class_materialization(*completion.origin,
                                                  *info,
                                                  *completion.arguments);
    } catch(const TemplateSubstitutionFailure &) {
      info->template_instantiation_in_progress = false;
      template_audit::log_upgrade_fail(type ? describe_type(type) : string("<null-type>"),
                                       info,
                                       "complete_class_type");
      throw;
    } catch(const logic_error &) {
      info->template_instantiation_in_progress = false;
      template_audit::log_upgrade_fail(type ? describe_type(type) : string("<null-type>"),
                                       info,
                                       "complete_class_type");
      throw;
    }
    info->template_instantiation_in_progress = false;
    if(!info->complete) {
      template_audit::log_upgrade_fail(type ? describe_type(type) : string("<null-type>"),
                                       info,
                                       "complete_class_type");
    }
    sync_query_type_layout(info);
    return info->complete ? info : nullptr;
  }

  void analyze_object_lifetime_actions(Scope & scope,
                                       const string & name,
                                       const TypePtr & type,
                                       const CppAstNode * initializer,
                                       DumpNode & out)
  {
    semantic_lifetime::analyze_object_lifetime_actions(*this, scope, name, type, initializer, out);
  }

  void analyze_statement(Scope & scope,
                         const TypePtr & return_type,
                         const CppAstNode & node,
                         DumpNode & out)
  {
    semantic_statement::analyze_statement(*this, scope, return_type, node, out);
  }

};

}  // namespace

CallSemNode analyze_calls_translation_unit(IRecogTokenSequence & tokens,
                                           bool expand_output_closure,
                                           bool emit_all_source_function_definitions,
                                           witness::TemplateWitnessSession * witness_session)
{
  DiagnosticContext::clear();
  try {
    CppAstParser parser(tokens);
    CppAstNode ast;
    if(!parser.parse_translation_unit(ast)) {
      throw logic_error(parser.error());
    }

    Analyzer analyzer(ast,
                      tokens,
                      expand_output_closure,
                      emit_all_source_function_definitions,
                      witness_session);
    CallSemNode result = analyzer.analyze();
    DiagnosticContext::clear();
    return result;
  } catch(const TemplateSubstitutionFailure & e) {
    throw TemplateSubstitutionFailure(
        append_diagnostic_detail(e.diagnostic(),
                                 append_diagnostic_context_message(std::string())));
  } catch(const logic_error & e) {
    throw logic_error(append_diagnostic_context_message(e.what()));
  }
}

CallSemNode analyze_calls_translation_unit(const vector<RecogToken> & tokens,
                                           bool expand_output_closure,
                                           bool emit_all_source_function_definitions,
                                           witness::TemplateWitnessSession * witness_session)
{
  DiagnosticContext::clear();
  try {
    CppAstParser parser(tokens);
    CppAstNode ast;
    if(!parser.parse_translation_unit(ast)) {
      throw logic_error(parser.error());
    }

    Analyzer analyzer(ast,
                      tokens,
                      expand_output_closure,
                      emit_all_source_function_definitions,
                      witness_session);
    CallSemNode result = analyzer.analyze();
    DiagnosticContext::clear();
    return result;
  } catch(const TemplateSubstitutionFailure & e) {
    throw TemplateSubstitutionFailure(
        append_diagnostic_detail(e.diagnostic(),
                                 append_diagnostic_context_message(std::string())));
  } catch(const logic_error & e) {
    throw logic_error(append_diagnostic_context_message(e.what()));
  }
}

string describe_calls_translation_unit(IRecogTokenSequence & tokens)
{
  return callsem_dump_tree(analyze_calls_translation_unit(tokens));
}

string describe_calls_translation_unit(const vector<RecogToken> & tokens)
{
  return callsem_dump_tree(analyze_calls_translation_unit(tokens));
}
