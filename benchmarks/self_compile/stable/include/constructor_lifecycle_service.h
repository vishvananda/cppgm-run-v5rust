#pragma once

#include <vector>

#include "semantic_conversion.h"
#include "semantic_context.h"
#include "semantic_model.h"

namespace constructor_lifecycle_service {

enum class ConstructorIntent
{
  DirectInitialization,
  NoUserDefinedConstructors,
  ImplicitDefaultConstructorViability,
  UserDefinedConversionConstructorProbe,
  NonExplicitConstruction,
  CopyListInitialization,
  AggregateConstruction,
  AggregatePartialMatch,
};

struct ConstructorSelectionProfile
{
  ConstructorIntent intent = ConstructorIntent::DirectInitialization;
  const char * context = "";
  bool instantiate_bodies = true;
  bool allow_partial_aggregate = false;
};

struct ConstructorSelectionResult
{
  semantic_model::FunctionBinding * ctor = nullptr;
  std::vector<semantic_conversion::ExprInfo> converted_args;
  std::vector<semantic_conversion::ConversionRank> ranks;
};

struct ConstructorActionResult
{
  semantic_model::FunctionBinding * ctor = nullptr;
  std::vector<semantic_conversion::ExprInfo> call_args;
  semantic_conversion::ExprInfo call_expr;
  bool trivial_lifecycle = false;
  CallableEmissionDecision emission;
};

ConstructorSelectionProfile direct_initialization_profile(const char * context = "");
ConstructorSelectionProfile no_user_defined_constructor_profile(const char * context);
ConstructorSelectionProfile implicit_default_constructor_viability_profile(
    const char * context);
ConstructorSelectionProfile user_defined_conversion_constructor_probe_profile(
    const char * context,
    bool instantiate_bodies);
ConstructorSelectionProfile non_explicit_construction_profile(
    const char * context);
ConstructorSelectionProfile copy_list_initialization_profile(
    const char * context);
ConstructorSelectionProfile aggregate_construction_profile(
    const char * context,
    bool allow_partial_aggregate = true);
ConstructorSelectionProfile aggregate_partial_match_profile(
    const char * context,
    bool allow_partial_aggregate = true);
ConstructorSelectionOptions selection_options_for(
    const ConstructorSelectionProfile & profile);
void apply_selection_profile(
    ConstructorSelectionOptions & options,
    const ConstructorSelectionProfile & profile);

ConstructorSelectionResult select_constructor_from_exprs(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo & info,
    const std::vector<semantic_conversion::ExprInfo> & source_args,
    const ConstructorSelectionOptions & options = ConstructorSelectionOptions());
void select_constructor_from_exprs_into(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo & info,
    const std::vector<semantic_conversion::ExprInfo> & source_args,
    ConstructorSelectionResult & out,
    const ConstructorSelectionOptions & options = ConstructorSelectionOptions());

ConstructorSelectionResult select_constructor(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo & info,
    const std::vector<const CppAstNode *> & arg_nodes,
    const ConstructorSelectionOptions & options = ConstructorSelectionOptions());
void select_constructor_into(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo & info,
    const std::vector<const CppAstNode *> & arg_nodes,
    ConstructorSelectionResult & out,
    const ConstructorSelectionOptions & options = ConstructorSelectionOptions());

ConstructorSelectionResult select_constructor_for_direct_braced_init(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo & info,
    const CppAstNode & direct_braced_init,
    const ConstructorSelectionOptions & options = ConstructorSelectionOptions());
void select_constructor_for_direct_braced_init_into(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo & info,
    const CppAstNode & direct_braced_init,
    ConstructorSelectionResult & out,
    const ConstructorSelectionOptions & options = ConstructorSelectionOptions());

CallableEmissionDecision require_selected_constructor(
    OutputRequirementContext & ctx,
    const ConstructorSelectionResult & result,
    OutputReason reason,
    bool mark_output_required = true);

CallableEmissionDecision require_lifecycle_function(
    OutputRequirementContext & ctx,
    semantic_model::FunctionBinding * binding,
    OutputReason reason,
    bool mark_output_required = true);

ConstructorActionResult prepare_selected_constructor_action(
    SemanticContext & ctx,
    const semantic_conversion::ExprInfo & object_ptr,
    const ConstructorSelectionResult & selection,
    bool trivial_lifecycle,
    OutputReason reason,
    bool mark_output_required = true);
void prepare_selected_constructor_action_into(
    SemanticContext & ctx,
    const semantic_conversion::ExprInfo & object_ptr,
    const ConstructorSelectionResult & selection,
    bool trivial_lifecycle,
    OutputReason reason,
    ConstructorActionResult & out,
    bool mark_output_required = true);

ConstructorActionResult prepare_lifecycle_call(
    SemanticContext & ctx,
    semantic_model::FunctionBinding * binding,
    const std::vector<semantic_conversion::ExprInfo> & call_args,
    bool trivial_lifecycle,
    OutputReason reason,
    bool mark_output_required = true);
void prepare_lifecycle_call_into(
    SemanticContext & ctx,
    semantic_model::FunctionBinding * binding,
    const std::vector<semantic_conversion::ExprInfo> & call_args,
    bool trivial_lifecycle,
    OutputReason reason,
    ConstructorActionResult & out,
    bool mark_output_required = true);

bool value_initialization_requires_zero_init(
    const semantic_model::FunctionBinding & binding);

}  // namespace constructor_lifecycle_service
