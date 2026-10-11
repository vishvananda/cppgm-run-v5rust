#pragma once

#include <cstddef>
#include <map>
#include <set>
#include <string>
#include <tuple>
#include <utility>
#include <vector>

#include "function_template_identity.h"
#include "semantic_conversion.h"
#include "template_instantiation_api.h"
#include "template_service_interfaces.h"
#include "witness_api.h"

class SemanticContext;
namespace symbol_linkage {
struct FunctionSymbolOptions;
}

namespace template_api {

enum NonTypeArgumentStatus
{
  NT_ARG_PARSE_FAILED,
  NT_ARG_DEPENDENT,
  NT_ARG_EVAL_FAILED,
  NT_ARG_EVALUATED
};

enum MatchKind
{
  MS_PRIMARY,
  MS_EXPLICIT_SPECIALIZATION,
  MS_PARTIAL_SPECIALIZATION
};

struct ClassSpecializationSelection
{
  const CppAstNode * class_node = nullptr;
  semantic_model::Scope * binding_scope = nullptr;
  const std::vector<template_model::TemplateParameterInfo> * parameters = nullptr;
  std::vector<template_model::TemplateArgument> arguments;
  const std::vector<cpp_decl::TemplateArgumentSyntax> * argument_syntaxes = nullptr;
  std::map<std::string, std::size_t> pack_sizes;
  std::string selection_key;
  MatchKind kind = MS_PRIMARY;
  bool reentrant_primary = false;
};

struct ClassTemplateCompletionPlan
{
  bool ready = false;
  bool in_progress = false;
  semantic_model::ClassTemplateDecl * origin = nullptr;
  const CppAstNode * output_node = nullptr;
  const std::vector<template_model::TemplateArgument> * arguments = nullptr;
  std::string trace_suffix;
};

struct ClassTemplateUseInfo
{
  semantic_model::ClassInfo * instance = nullptr;
  semantic_model::ClassTemplateDecl * origin = nullptr;
  const std::vector<template_model::TemplateParameterInfo> * parameters = nullptr;
  const std::vector<template_model::TemplateArgument> * arguments = nullptr;
  std::string key;
  std::string template_name;
  std::string unqualified_name;
  const std::vector<std::string> * canonical_texts = nullptr;
  bool dependent_arguments = false;
  bool explicit_case = false;
  bool selects_specialized_definition = false;
  bool has_stored_key = false;
  bool has_selection = false;
  ClassSpecializationSelection selection;
};

struct VariableSpecializationSelection
{
  semantic_model::Scope * binding_scope = nullptr;
  const std::vector<template_model::TemplateParameterInfo> * parameters = nullptr;
  std::vector<template_model::TemplateArgument> arguments;
  std::map<std::string, std::size_t> pack_sizes;
  const CppAstNode * specifiers = nullptr;
  const CppAstNode * declarator = nullptr;
  const CppAstNode * initializer = nullptr;
  std::string selection_key;
  MatchKind kind = MS_PRIMARY;
};

struct ParsedFunctionTemplateSignature
{
  std::string name;
  cpp_decl::TypePtr type;
  std::vector<std::pair<std::string, cpp_decl::TypePtr> > params;
  std::vector<const CppAstNode *> default_arguments;
  std::vector<const CppAstNode *> parameter_declarations;
  CppAstNode result_type_pattern;
  CppAstNode effective_declarator;
};

enum class FunctionTemplateSignatureParseStatus
{
  Ok,
  UnsupportedDeclSpecifiers,
  UnsupportedDeclarator,
  UnsupportedParameterClause,
};

struct FunctionTemplateSignatureParseResult
{
  FunctionTemplateSignatureParseStatus status =
      FunctionTemplateSignatureParseStatus::Ok;
  ParsedFunctionTemplateSignature signature;
  std::string diagnostic;

  bool ok() const
  {
    return status == FunctionTemplateSignatureParseStatus::Ok;
  }
};

struct TemplateFunctionDeductionRequest
{
  semantic_model::FunctionTemplateDecl * decl = nullptr;
  const std::vector<semantic_conversion::ExprInfo> * args = nullptr;
  semantic_model::Scope * use_scope = nullptr;
  semantic_model::Scope * resolution_scope = nullptr;
  const std::vector<template_model::TemplateArgument> * explicit_arguments = nullptr;
  cpp_decl::TypePtr target_type;
};

struct TemplateFunctionDeductionResult
{
  std::vector<template_model::TemplateArgument> arguments;
  std::map<std::string, std::size_t> pack_sizes;
};

struct TemplateSelectedClassInstantiationRequest
{
  semantic_model::ClassTemplateDecl * decl = nullptr;
  TemplateEnvironmentHandle use_scope;
  std::vector<template_model::TemplateArgument> arguments;
  ClassSpecializationSelection specialization;
};

struct TemplateNestedMemberClassFinalizationRequest
{
  semantic_model::ClassTemplateDecl * owner_decl = nullptr;
  semantic_model::ClassInfo * nested_info = nullptr;
  std::vector<template_model::TemplateArgument> owner_arguments;
  bool emit_track_instantiation = true;
};

struct TemplateNestedMemberClassCompletionRequest
{
  semantic_model::ClassInfo * nested_info = nullptr;
};

struct TemplateNestedMemberClassCompletionResult
{
  semantic_model::ClassInfo * nested_info = nullptr;
  bool attempted = false;
  bool completed = false;
};

using witness::AliasUseSourceDecision;
using witness::ClassUseSourceDecision;
using witness::FunctionCallSourceDecision;
using witness::SourceDropSet;
using witness::SourceSelectionKind;
using witness::TemplateWitnessSourceBinding;
using witness::TemplateWitnessSourceDrop;
using witness::VariableUseSourceDecision;
using TemplateWitnessSourceDropSet = witness::SourceDropSet;

class ScopedTemplateArgumentSourceLocations
{
public:
  ScopedTemplateArgumentSourceLocations(
      const std::vector<std::string> & texts,
      const std::vector<std::string> & locations);
  ~ScopedTemplateArgumentSourceLocations();

  ScopedTemplateArgumentSourceLocations(
      const ScopedTemplateArgumentSourceLocations &) = delete;
  ScopedTemplateArgumentSourceLocations & operator=(
      const ScopedTemplateArgumentSourceLocations &) = delete;

private:
  bool active_ = false;
};

bool current_template_argument_source_locations_active();
std::string current_template_argument_source_location(
    const std::string & text,
    std::size_t index);

class ScopedTemplateIdSourceArguments
{
public:
  ScopedTemplateIdSourceArguments(
      const std::string & location,
      const std::string & template_name,
      std::vector<std::string> arg_texts);
  ~ScopedTemplateIdSourceArguments();

  ScopedTemplateIdSourceArguments(
      const ScopedTemplateIdSourceArguments &) = delete;
  ScopedTemplateIdSourceArguments & operator=(
      const ScopedTemplateIdSourceArguments &) = delete;

private:
  bool active_ = false;
};

bool current_template_id_source_arguments(
    const std::string & location,
    const std::string & template_name,
    std::vector<std::string> & arg_texts);
const std::vector<std::string> * current_template_id_source_arguments_ptr(
    const std::string & location,
    const std::string & template_name);

bool resolve_template_arguments(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::vector<template_model::TemplateParameterInfo> & parameters,
    const std::vector<std::string> & texts,
    const std::vector<cpp_decl::TemplateArgumentSyntax> * syntaxes,
    std::vector<template_model::TemplateArgument> & out,
    semantic_model::Scope * default_argument_declaring_scope = nullptr);

bool resolve_template_arguments(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::vector<template_model::TemplateParameterInfo> & parameters,
    const std::vector<std::string> & texts,
    std::vector<template_model::TemplateArgument> & out,
    semantic_model::Scope * default_argument_declaring_scope = nullptr);

bool deduce_function_template(
    SemanticContext & ctx,
    const TemplateFunctionDeductionRequest & request,
    TemplateFunctionDeductionResult & out);

TemplateInstantiationResult acquire_function_instantiation(
    SemanticContext & ctx,
    const TemplateFunctionInstantiationRequest & request);

TemplateInstantiationResult acquire_class_instantiation(
    SemanticContext & ctx,
    const TemplateClassInstantiationRequest & request);

TemplateInstantiationResult acquire_selected_class_instantiation(
    SemanticContext & ctx,
    const TemplateSelectedClassInstantiationRequest & request);

TemplateInstantiationResult finalize_class_instantiation(
    SemanticContext & ctx,
    const TemplateClassFinalizationRequest & request);
bool build_class_finalization_request(
    semantic_model::ClassInfo & info,
    TemplateClassFinalizationRequest & out);

TemplateInstantiationResult finalize_nested_member_class_instantiation(
    SemanticContext & ctx,
    const TemplateNestedMemberClassFinalizationRequest & request);

bool prepare_nested_member_class_reference_from_owner_definition(
    SemanticContext & ctx,
    semantic_model::ClassInfo * nested,
    std::size_t incoming_template_parameter_count,
    std::size_t & owner_template_parameter_count);

TemplateNestedMemberClassCompletionResult complete_nested_member_class_from_owner_definition(
    SemanticContext & ctx,
    const TemplateNestedMemberClassCompletionRequest & request);

TemplateInstantiationResult finalize_nested_member_class_instantiation_from_owner(
    SemanticContext & ctx,
    semantic_model::ClassInfo * nested_info,
    bool emit_track_instantiation);

TemplateInstantiationResult acquire_function_binding(
    SemanticContext & ctx,
    const TemplateFunctionBindingAcquisitionRequest & request);
bool build_function_definition_upgrade_request(
    const semantic_model::FunctionBinding & binding,
    semantic_model::Scope & use_scope,
    TemplateFunctionInstantiationRequest & out);

TemplateInstantiationResult acquire_variable_instantiation(
    SemanticContext & ctx,
    const TemplateVariableInstantiationRequest & request);

bool class_has_template_identity(const semantic_model::ClassInfo * info);
const std::string & class_template_instantiation_key(
    const semantic_model::ClassInfo & info);
std::string class_template_effective_instantiation_key(
    SemanticContext & ctx,
    const semantic_model::ClassInfo & info);
std::string template_argument_identity_key(
    SemanticContext & ctx,
    const std::vector<template_model::TemplateArgument> & arguments);
std::vector<std::string> canonical_template_argument_texts(
    SemanticContext & ctx,
    const std::vector<template_model::TemplateArgument> & arguments);
bool template_arguments_are_dependent(
    SemanticContext & ctx,
    const std::vector<template_model::TemplateArgument> & arguments);
void canonicalize_simple_dependent_argument_texts(
    SemanticContext & ctx,
    std::vector<template_model::TemplateArgument> & arguments);
std::string specialization_name_for_instantiation(
    SemanticContext & ctx,
    const std::string & name,
    const std::vector<template_model::TemplateArgument> & arguments);
std::string display_specialization_name_for_instantiation(
    SemanticContext & ctx,
    const std::string & name,
    const std::vector<template_model::TemplateArgument> & arguments);
bool record_class_template_instantiation_state(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info,
    const std::string & key,
    const std::vector<template_model::TemplateArgument> & arguments,
    bool is_explicit_specialization,
    bool suppress_implicit_instantiation_definition,
    bool dependent_arguments,
    const std::vector<std::string> * dependent_argument_texts = nullptr,
    const std::vector<cpp_decl::TemplateArgumentSyntax> * dependent_argument_syntaxes = nullptr,
    const std::vector<template_model::TemplateParameterInfo> *
        dependent_argument_mangle_parameters = nullptr,
    const std::vector<template_model::TemplateArgument> *
        dependent_argument_mangle_arguments = nullptr,
    const std::map<std::string, std::size_t> *
        dependent_argument_mangle_pack_sizes = nullptr);
bool refresh_forward_class_template_selection(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info);
bool class_template_completion_has_owner_definition(
    const semantic_model::ClassInfo & info);
ClassTemplateCompletionPlan class_template_completion_plan(
    const semantic_model::ClassInfo & info);
bool apply_out_of_class_static_member_definitions_to_reference(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info);
bool apply_out_of_class_member_function_abi_metadata(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info);
bool class_template_use_info_for_type(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const cpp_decl::TypePtr & type,
    ClassTemplateUseInfo & out,
    bool select_specialization = true);
bool class_template_use_info_for_class(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo * info,
    ClassTemplateUseInfo & out,
    bool select_specialization = true);
bool template_id_matches_class_template_origin(
    const cpp_decl::QualifiedName & template_id,
    const ClassTemplateUseInfo & info);
void append_class_template_type_arguments(
    const semantic_model::ClassInfo * info,
    std::vector<cpp_decl::TypePtr> & out);
bool class_template_instantiation_depends_on_template_parameter(
    SemanticContext & ctx,
    const semantic_model::ClassInfo & info);
bool class_has_source_template_identity(const semantic_model::ClassInfo * info);
bool class_source_template_identity_matches(
    const semantic_model::ClassInfo * info,
    const semantic_model::ClassTemplateDecl * decl);
bool class_has_non_dependent_source_template_identity(
    const semantic_model::ClassInfo * info);
bool class_is_explicit_specialization(const semantic_model::ClassInfo * info);
bool class_owner_scope_has_template_identity(const semantic_model::ClassInfo * info);
bool scope_has_template_owner_identity(const semantic_model::Scope * scope);
bool scope_has_linkage_template_owner_identity(const semantic_model::Scope * scope);
bool function_binding_has_template_identity(const semantic_model::FunctionBinding * binding);
bool function_or_owner_has_template_identity(const semantic_model::FunctionBinding * binding);
bool function_binding_has_linkage_template_identity(
    const semantic_model::FunctionBinding * binding);
bool value_or_owner_has_template_identity(const semantic_model::ValueBinding * binding);
bool function_binding_has_source_template_identity(
    const semantic_model::FunctionBinding * binding);
bool function_binding_has_parameter_name_syntax_source(
    const semantic_model::FunctionBinding & binding);
const CppAstNode * function_binding_source_template_declarator(
    const semantic_model::FunctionBinding & binding);
const void * function_binding_source_template_debug_identity(
    const semantic_model::FunctionBinding * binding);
bool function_binding_has_template_or_body_definition_source(
    const semantic_model::FunctionBinding & binding);
bool function_binding_is_declaration_only_template(
    const semantic_model::FunctionBinding & binding);
bool function_binding_has_empty_template_identity(
    const semantic_model::FunctionBinding & binding);
FunctionTemplateRegistrationIdentity function_binding_registration_identity(
    const semantic_model::FunctionBinding & binding);
bool function_binding_matches_instantiation_identity(
    const semantic_model::FunctionBinding & binding,
    const FunctionTemplateRegistrationIdentity & identity);
bool function_binding_matches_materialized_owner_template_identity(
    const semantic_model::FunctionBinding & binding,
    const FunctionTemplateRegistrationIdentity & identity);
bool should_preserve_owner_prefixed_template_identity(
    const semantic_model::FunctionBinding & original,
    const semantic_model::FunctionBinding & materialized,
    bool types_match);
void record_function_template_identity(
    semantic_model::FunctionBinding & binding,
    const FunctionTemplateRegistrationIdentity & identity);
void record_function_template_arguments_preserving_pack_sizes(
    semantic_model::FunctionBinding & binding,
    const std::vector<template_model::TemplateArgument> & arguments,
    bool has_arguments);
void adopt_materialized_owner_template_identity(
    semantic_model::FunctionBinding & binding,
    const FunctionTemplateRegistrationIdentity & identity,
    semantic_model::Scope * declaration_scope);
void adopt_function_template_identity_from_materialized(
    semantic_model::FunctionBinding & original,
    const semantic_model::FunctionBinding & materialized);
bool class_suppresses_implicit_instantiation_definition(
    const semantic_model::ClassInfo * info);
bool function_template_decl_is_member_function_template(
    const semantic_model::FunctionTemplateDecl & decl);
bool function_binding_is_member_function_template(
    const semantic_model::FunctionBinding & binding);
bool function_binding_owner_class_suppresses_implicit_instantiation_definition(
    const semantic_model::FunctionBinding & binding);
bool function_binding_excluded_from_explicit_instantiation(
    const semantic_model::FunctionBinding & binding);
bool function_binding_bypasses_explicit_instantiation_suppression(
    const semantic_model::FunctionBinding & binding,
    bool explicit_instantiation_suppressed);
bool function_binding_bypasses_explicit_instantiation_suppression(
    const semantic_model::FunctionBinding & binding);
bool function_binding_output_suppressed_by_explicit_instantiation(
    const semantic_model::FunctionBinding & binding);
bool value_binding_owner_class_suppresses_implicit_instantiation_definition(
    const semantic_model::ValueBinding & binding);
bool value_binding_output_suppressed_by_explicit_instantiation(
    const semantic_model::ValueBinding & binding);
void apply_function_template_symbol_options(
    semantic_model::FunctionTemplateDecl * source_template,
    const std::vector<template_model::TemplateArgument> * instantiation_arguments,
    bool has_instantiation_arguments,
    const semantic_model::ClassInfo * owner_class,
    bool is_constructor,
    bool is_destructor,
    symbol_linkage::FunctionSymbolOptions & options);
void apply_function_binding_template_symbol_options(
    const semantic_model::FunctionBinding & binding,
    symbol_linkage::FunctionSymbolOptions & options);

struct TemplateClassOutputReadiness
{
  bool templated_context = false;
  bool suppress_implicit_definition = false;
  bool non_dependent = true;
  bool complete = true;
  bool output_blocked_by_placeholders = false;
};

const semantic_model::ClassInfo * effective_instantiated_class_output_owner(
    const semantic_model::ClassInfo & info);
TemplateClassOutputReadiness compute_instantiated_class_output_readiness(
    SemanticContext & ctx,
    const semantic_model::ClassInfo & info);
bool instantiated_class_arguments_non_dependent(
    SemanticContext & ctx,
    const semantic_model::ClassInfo & info);
bool function_binding_signature_mentions_source_template_parameter(
    const semantic_model::FunctionBinding & binding,
    const std::string & signature_text);
bool function_binding_instantiation_arguments_dependent(
    SemanticContext & ctx,
    const semantic_model::FunctionBinding & binding);
bool function_binding_instantiation_arguments_complete(
    SemanticContext & ctx,
    const semantic_model::FunctionBinding & binding);
void bump_scope_template_binding_fingerprint_epoch(semantic_model::Scope & scope);
semantic_model::FunctionBinding * find_defined_class_function_matching_template_identity(
    SemanticContext & ctx,
    semantic_model::ClassInfo & owner,
    const std::string & lookup_name,
    const semantic_model::FunctionBinding & binding);
semantic_model::FunctionBinding * find_defined_function_matching_template_identity(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::string & lookup_name,
    const semantic_model::FunctionBinding & binding);

std::string function_binding_witness_entity(const semantic_model::FunctionBinding * binding);
std::string function_binding_witness_decl_location(
    SemanticContext & ctx,
    const semantic_model::FunctionBinding * binding);
std::string function_binding_template_trace_key(
    const semantic_model::FunctionBinding * binding);
std::string alias_template_witness_entity(
    const semantic_model::AliasTemplateDecl * decl);

struct TemplateFunctionDefinitionClosureState
{
  semantic_model::ClassInfo * template_owner = nullptr;
  std::string decl_location;
  bool template_owned_binding = false;
  bool closure_trigger_differs = false;
};

TemplateFunctionDefinitionClosureState function_definition_closure_state(
    SemanticContext & ctx,
    const semantic_model::FunctionBinding * binding);
bool function_definition_materialized_by_enclosing_closure(
    const semantic_model::FunctionBinding * binding);
void mark_function_definition_materialized_by_enclosing_closure(
    semantic_model::FunctionBinding * binding);
void note_closure_owner_class_instantiation_if_needed(
    SemanticContext & ctx,
    semantic_model::ClassInfo * owner,
    const TemplateFunctionDefinitionClosureState & state);
void note_function_definition_ensure_requested(
    SemanticContext & ctx,
    const semantic_model::FunctionBinding * binding,
    const TemplateFunctionDefinitionClosureState & state);
void note_function_definition_materialized_by_closure(
    SemanticContext & ctx,
    const semantic_model::FunctionBinding * binding,
    const TemplateFunctionDefinitionClosureState & state);
void note_function_declaration_instantiated_by_closure(
    SemanticContext & ctx,
    const semantic_model::FunctionBinding * binding,
    const TemplateFunctionDefinitionClosureState & state);

TemplateWitnessEntryContext make_function_binding_closure_entry_context(
    SemanticContext & ctx,
    TemplateClosureReason reason,
    const semantic_model::FunctionBinding * binding);
TemplateWitnessEntryContext make_class_closure_entry_context(
    SemanticContext & ctx,
    TemplateClosureReason reason,
    const semantic_model::ClassInfo * info);
TemplateWitnessEntryContext make_value_binding_closure_entry_context(
    SemanticContext & ctx,
    TemplateClosureReason reason,
    const semantic_model::ValueBinding * binding);
ScopedTemplateWitnessEntryContext maybe_enter_function_binding_closure_context(
    SemanticContext & ctx,
    TemplateClosureReason reason,
    const semantic_model::FunctionBinding * binding);
ScopedTemplateWitnessEntryContext maybe_enter_function_body_materialization_context(
    SemanticContext & ctx,
    const semantic_model::FunctionBinding * binding);
ScopedTemplateWitnessEntryContext maybe_enter_class_closure_context(
    SemanticContext & ctx,
    TemplateClosureReason reason,
    const semantic_model::ClassInfo * info);
ScopedTemplateWitnessEntryContext maybe_enter_class_tracking_context(
    SemanticContext & ctx,
    const semantic_model::ClassInfo * info);
ScopedTemplateWitnessEntryContext maybe_enter_class_finalization_context(
    SemanticContext & ctx,
    const semantic_model::ClassInfo * info);
ScopedTemplateWitnessEntryContext maybe_enter_value_binding_closure_context(
    SemanticContext & ctx,
    TemplateClosureReason reason,
    const semantic_model::ValueBinding * binding);

void note_function_binding_closure_event(
    SemanticContext & ctx,
    TemplateWitnessLogEventKind kind,
    const std::string & location,
    const semantic_model::FunctionBinding * binding,
    const std::string & detail = std::string(),
    TemplateLifecycleCause cause = TemplateLifecycleCause::None);
void note_class_closure_event(
    SemanticContext & ctx,
    TemplateWitnessLogEventKind kind,
    const std::string & location,
    const semantic_model::ClassInfo * info,
    const std::string & detail = std::string(),
    TemplateLifecycleCause cause = TemplateLifecycleCause::None);
void note_value_binding_closure_event(
    SemanticContext & ctx,
    TemplateWitnessLogEventKind kind,
    const std::string & location,
    const semantic_model::ValueBinding * binding,
    const std::string & detail = std::string(),
    TemplateLifecycleCause cause = TemplateLifecycleCause::None);
bool class_instantiation_log_emitted(const semantic_model::ClassInfo * info);
void note_output_tracked_class_instantiation_if_needed(
    SemanticContext & ctx,
    semantic_model::ClassInfo * info,
    bool already_logged);
void note_anonymous_member_class_events_if_owner_logged(
    SemanticContext & ctx,
    const semantic_model::ClassInfo & info);
void note_template_member_value_instantiation_if_needed(
    SemanticContext & ctx,
    const semantic_model::ValueBinding & binding);
void note_nested_member_class_instantiation_completed_if_needed(
    SemanticContext & ctx,
    semantic_model::ClassInfo * info,
    const CppAstNode * preferred_decl_node,
    const CppAstNode * fallback_decl_node);
void note_nested_member_class_track_instantiation(
    SemanticContext & ctx,
    const semantic_model::ClassInfo & info,
    const std::string & decl_location);

using witness::append_source_drop;
using witness::append_unique_source_drop;
using witness::note_alias_use_source_decision;
using witness::note_class_use_source_decision;
using witness::note_function_call_source_decision;
using witness::note_source_owned_class_use_source_decision;
using witness::note_variable_use_source_decision;

void append_function_template_witness_bindings(
    SemanticContext & ctx,
    const semantic_model::FunctionBinding * binding,
    std::size_t explicit_arg_count,
    std::vector<TemplateWitnessSourceBinding> & out);

void append_class_template_witness_bindings(
    SemanticContext & ctx,
    const semantic_model::ClassInfo * info,
    std::vector<TemplateWitnessSourceBinding> & out);

std::string class_witness_output_qualified_name(
    SemanticContext & ctx,
    const semantic_model::ClassInfo & info);

std::string template_witness_source_argument_text(
    SemanticContext & ctx,
    const template_model::TemplateArgument & arg);

enum class TemplateWitnessSourceBindingPolicy
{
  FixedSource,
  DeducedWithDefaultedTrailingDefaults
};

void append_template_witness_source_bindings(
    SemanticContext & ctx,
    std::vector<TemplateWitnessSourceBinding> & out,
    const std::vector<template_model::TemplateParameterInfo> & parameters,
    const std::vector<template_model::TemplateArgument> & arguments,
    const std::string & source,
    TemplateWitnessSourceBindingPolicy policy =
        TemplateWitnessSourceBindingPolicy::FixedSource);

void append_template_witness_source_bindings(
    SemanticContext & ctx,
    std::vector<TemplateWitnessSourceBinding> & out,
    const std::vector<template_model::TemplateParameterInfo> & parameters,
    const std::vector<template_model::TemplateArgument> & arguments,
    const std::vector<std::string> & explicit_argument_texts,
    const std::string & explicit_source,
    const std::string & defaulted_source,
    bool treat_explicit_defaults_as_defaulted = true);

namespace binding {

void bind_named_type(semantic_model::Scope & scope,
                     const std::string & name,
                     const cpp_decl::TypePtr & type);

void overlay_direct_scope_bindings(semantic_model::Scope & target,
                                   const semantic_model::Scope & source);

void overlay_ancestor_scope_bindings(
    semantic_model::Scope & target,
    const semantic_model::Scope & source,
    const semantic_model::Scope * stop_before = nullptr);

void overlay_instantiation_use_scope_bindings(semantic_model::Scope & target,
                                              const semantic_model::Scope & use_scope,
                                              const semantic_model::Scope * declaring_scope);

void overlay_instantiation_use_scope_bindings(
    semantic_model::Scope & target,
    const semantic_model::Scope & use_scope,
    const semantic_model::Scope * declaring_scope,
    const std::set<std::string> & excluded_names);

void overlay_instantiation_local_named_types(
    SemanticContext & ctx,
    semantic_model::Scope & target,
    const semantic_model::Scope & use_scope,
    const semantic_model::Scope * declaring_scope,
    const std::vector<template_model::TemplateArgument> & arguments,
    const std::set<std::string> * excluded_names = nullptr);

void bind_template_arguments_into_scope(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::vector<template_model::TemplateParameterInfo> & parameters,
    const std::vector<template_model::TemplateArgument> & arguments,
    const std::map<std::string, std::size_t> * pack_sizes = nullptr);

semantic_model::Scope & bind_template_arguments(
    SemanticContext & ctx,
    semantic_model::Scope & declaring_scope,
    const std::vector<template_model::TemplateParameterInfo> & parameters,
    const std::vector<template_model::TemplateArgument> & arguments,
    const std::map<std::string, std::size_t> * pack_sizes = nullptr);

semantic_model::Scope & bind_template_arguments_for_instantiation(
    SemanticContext & ctx,
    semantic_model::Scope & declaring_scope,
    semantic_model::Scope & use_scope,
    const std::vector<template_model::TemplateParameterInfo> & parameters,
    const std::vector<template_model::TemplateArgument> & arguments,
    const std::map<std::string, std::size_t> * pack_sizes = nullptr,
    semantic_model::ClassInfo * active_owner = nullptr);

semantic_model::Scope & bind_class_template_arguments_for_instantiation(
    SemanticContext & ctx,
    semantic_model::Scope & declaring_scope,
    semantic_model::Scope & use_scope,
    const std::vector<template_model::TemplateParameterInfo> & parameters,
    const std::vector<template_model::TemplateArgument> & arguments,
    const std::map<std::string, std::size_t> * pack_sizes = nullptr);

}  // namespace binding

std::size_t scope_template_binding_fingerprint(
    const semantic_model::Scope & scope);

std::size_t scope_template_instance_fingerprint(
    const semantic_model::Scope & scope);

namespace type {

// template-boundary-audit: begin text_recovery_bridge
std::string lookup_text_for_type_argument(SemanticContext & ctx,
                                          const cpp_decl::TypePtr & type);

bool substitute_type(const cpp_decl::TypePtr & type,
                     const std::vector<template_model::TemplateParameterInfo> & parameters,
                     const std::vector<template_model::TemplateArgument> & arguments,
                     cpp_decl::TypePtr & out);

bool resolve_instantiated_dependent_type(SemanticContext & ctx,
                                         semantic_model::Scope & scope,
                                         const cpp_decl::TypePtr & type,
                                         cpp_decl::TypePtr & out);

bool resolve_type_argument_input(SemanticContext & ctx,
                                 semantic_model::Scope & scope,
                                 const cpp_decl::TemplateArgumentSyntax * syntax,
                                 bool reference_class_templates_only,
                                 cpp_decl::TypePtr & out);

bool text_mentions_template_placeholders(SemanticContext & ctx,
                                         semantic_model::Scope & scope,
                                         const std::string & text);

bool text_mentions_dependent_non_namespace_binding_names(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::string & text);

bool text_mentions_non_namespace_binding_names(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::string & text);

bool text_mentions_current_specialization_names(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::string & text);

bool should_defer_unresolved_type_lookup(SemanticContext & ctx,
                                         semantic_model::Scope & scope,
                                         const std::string & text);

std::vector<std::string> expand_bound_type_pack_texts(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::vector<std::string> & texts);

std::vector<std::string> expand_bound_expression_pack_texts(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::string & text);

bool parse_type_id_node_for_templates(SemanticContext & ctx,
                                      semantic_model::Scope & scope,
                                      const CppAstNode & type_id,
                                      cpp_decl::TypePtr & out,
                                      bool reference_class_templates_only = false);

bool parse_decltype_or_typeof_node(SemanticContext & ctx,
                                   semantic_model::Scope & scope,
                                   const CppAstNode & node,
                                   cpp_decl::TypePtr & out);

bool resolve_template_id_syntax_type(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const cpp_decl::TemplateIdSyntax & syntax,
    bool reference_class_templates_only,
    const std::string & source_location,
    cpp_decl::TypePtr & out,
    semantic_model::Scope * argument_scope = nullptr,
    ClassTemplateSourceUseMode source_use_mode =
        ClassTemplateSourceUseMode::EmitClassUse);
// template-boundary-audit: end text_recovery_bridge

bool scope_has_type_parameter_pack_name(const semantic_model::Scope & scope,
                                        const std::string & name);

void append_structured_bool_value_dependencies_in_expression_ast(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node,
    std::vector<template_model::TemplateValueDependency> & out);

void append_base_template_value_dependencies(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & base_name,
    const cpp_decl::TemplateIdSyntax * base_template_syntax,
    const std::vector<cpp_decl::TemplateArgumentSyntax> * base_arg_syntaxes,
    std::vector<template_model::TemplateValueDependency> & out);

bool note_constant_value_member_instantiations_in_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & expr);

bool template_id_names_alias_template(SemanticContext & ctx,
                                      semantic_model::Scope & scope,
                                      const cpp_decl::TemplateIdSyntax & syntax);

bool template_id_names_class_template(SemanticContext & ctx,
                                      semantic_model::Scope & scope,
                                      const cpp_decl::TemplateIdSyntax & syntax);

bool resolve_non_type_template_parameter_type(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const template_model::TemplateParameterInfo & parameter,
    cpp_decl::TypePtr & out);

}  // namespace type

namespace specialization {

using ClassSpecializationSelection = template_api::ClassSpecializationSelection;
using VariableSpecializationSelection = template_api::VariableSpecializationSelection;

ClassSpecializationSelection select_class_specialization(
    SemanticContext & ctx,
    semantic_model::ClassTemplateDecl & decl,
    semantic_model::Scope & use_scope,
    const std::string & key,
    const std::vector<template_model::TemplateArgument> & arguments,
    const std::vector<std::string> * dependent_source_argument_texts = nullptr);

}  // namespace specialization

namespace signature {

using ParsedFunctionTemplateSignature = template_api::ParsedFunctionTemplateSignature;

std::string normalize_special_member_template_name(SemanticContext & ctx,
                                                   const std::string & name,
                                                   bool is_constructor,
                                                   bool is_destructor);

void parse_function_template_parameter_clause(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::string & template_name,
    const CppAstNode & parameter_clause,
    std::vector<std::pair<std::string, cpp_decl::TypePtr> > & params,
    std::vector<const CppAstNode *> & default_arguments);

bool expand_parameter_clause_pack_patterns(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node,
    CppAstNode & expanded_clause,
    std::vector<const CppAstNode *> * default_args_out = nullptr);

CppAstNode build_function_result_type_pattern(
    const CppAstNode & parse_specifiers,
    const CppAstNode & parse_declarator);

ParsedFunctionTemplateSignature parse_function_template_signature(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::string & template_name,
    const CppAstNode & raw_declarator,
    const CppAstNode & parse_specifiers,
    const CppAstNode & parse_declarator,
    bool filter_nonmember_declarator);

FunctionTemplateSignatureParseResult try_parse_function_template_signature(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::string & template_name,
    const CppAstNode & raw_declarator,
    const CppAstNode & parse_specifiers,
    const CppAstNode & parse_declarator,
    bool filter_nonmember_declarator);

}  // namespace signature

namespace resolution {

bool resolve_template_argument(SemanticContext & ctx,
                               semantic_model::Scope & argument_scope,
                               semantic_model::Scope & parameter_scope,
                               const template_model::TemplateParameterInfo & parameter,
                               const std::string & text,
                               const cpp_decl::TemplateArgumentSyntax * syntax,
                               template_model::TemplateArgument & out);

bool resolve_template_argument(SemanticContext & ctx,
                               semantic_model::Scope & argument_scope,
                               semantic_model::Scope & parameter_scope,
                               const template_model::TemplateParameterInfo & parameter,
                               const std::string & text,
                               template_model::TemplateArgument & out);

bool trailing_pack_accepts_argument_count(
    const std::vector<template_model::TemplateParameterInfo> & parameters,
    std::size_t argument_count);

bool deduce_template_argument(SemanticContext & ctx,
                              const std::vector<template_model::TemplateParameterInfo> & parameters,
                              const cpp_decl::TypePtr & pattern,
                              const cpp_decl::TypePtr & actual,
                              std::map<std::string, cpp_decl::TypePtr> & deduced,
                              semantic_model::Scope * deduction_scope = nullptr,
                              bool partial_top_level_cv_deduction = false,
                              semantic_model::Scope * actual_lookup_scope = nullptr);

bool explicit_function_template_arguments_determine_signature(
    SemanticContext & ctx,
    semantic_model::FunctionTemplateDecl & decl,
    std::size_t explicit_argument_count);

bool resolve_function_explicit_template_arguments(
    SemanticContext & ctx,
    semantic_model::FunctionTemplateDecl & decl,
    semantic_model::Scope & resolution_scope,
    const std::vector<std::string> & explicit_arg_texts,
    std::vector<template_model::TemplateArgument> & out,
    const std::vector<cpp_decl::TemplateArgumentSyntax> * explicit_arg_syntaxes = nullptr);

}  // namespace resolution

}  // namespace template_api
