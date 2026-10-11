#pragma once

#include <map>
#include <string>
#include <vector>

#include "analysis_policy.h"
#include "cpp_decl_model.h"
#include "cppast_ast.h"
#include "symbol_linkage.h"

namespace semantic_model {
struct Scope;
struct ClassInfo;
struct FunctionBinding;
struct FunctionTemplateDecl;
enum MemberAccess : int;
}  // namespace semantic_model

namespace template_model {
struct TemplateArgument;
}  // namespace template_model

namespace semantic_conversion {
struct ExprInfo;
enum ConversionRank : int;
}  // namespace semantic_conversion

namespace semantic_overload {
struct CallAnalysisHints
{
  const semantic_conversion::ExprInfo * explicit_member_base = nullptr;
  std::vector<const semantic_conversion::ExprInfo *> args;
  std::vector<semantic_conversion::ConversionRank> * selected_ranks_out = nullptr;
  std::size_t explicit_member_arg_prefix = 0;
  std::string use_location;
  const semantic_model::ClassInfo * explicit_member_declared_in = nullptr;
  semantic_model::MemberAccess explicit_member_path_access =
      static_cast<semantic_model::MemberAccess>(0);
  bool suppress_user_defined_output_materialization = false;
  bool adl_candidates_precollected = false;
};

struct CallAnalysisOptions
{
  CallAnalysisOptions(bool instantiate_bodies_in = true,
                      const CallAnalysisHints * hints_in = nullptr)
      : instantiate_bodies(instantiate_bodies_in),
        hints(hints_in)
  {}

  bool instantiate_bodies;
  const CallAnalysisHints * hints;
};
}  // namespace semantic_overload

enum class OutputReason
{
  DirectCall,
  ConstructorUse,
  FunctionIdUse,
  NewExpression,
  VTableSlot,
  TemplateUpgrade,
  SyntheticDependency,
  RuntimeDependency
};

enum class InstantiatedFunctionOutputMode
{
  TrackOnly,
  RequireDefinition
};

struct CallableEmissionDecision
{
  bool should_seed_definition = false;
  bool mark_output_required_now = false;
};

struct ArgumentConversionOptions
{
  ArgumentConversionOptions(bool allow_user_defined_in = true,
                            bool instantiate_user_defined_bodies_in = true,
                            bool materialize_user_defined_output_in = true,
                            bool allow_explicit_in = false)
      : allow_user_defined(allow_user_defined_in),
        instantiate_user_defined_bodies(instantiate_user_defined_bodies_in),
        materialize_user_defined_output(materialize_user_defined_output_in),
        materialize_standard_adjustments(true),
        allow_explicit(allow_explicit_in)
  {}

  bool allow_user_defined;
  bool instantiate_user_defined_bodies;
  bool materialize_user_defined_output;
  bool materialize_standard_adjustments;
  bool allow_explicit;
  std::string source_use_location;
};

struct ConstructorSelectionOptions
{
  const char * context = "";
  std::string use_location;
  std::string source_witness_location;
  bool allow_user_defined = true;
  bool instantiate_bodies = true;
  bool allow_partial_aggregate = false;
  bool allow_explicit = true;
  bool initializer_list_only = false;
  bool synthesize_implicit_copy_move = true;
  bool source_witness_direct_construction = false;
};

struct PreparedDeclarationSpecifiers
{
  CppAstNode resolved_specifiers;
  bool has_auto = false;
  bool parsed_decl_spec = false;
  bool declaration_is_typedef = false;
  cpp_decl::TypePtr base;
};

namespace semantic_policy {

inline semantic_overload::CallAnalysisOptions call_analysis(
    bool instantiate_bodies,
    const semantic_overload::CallAnalysisHints * hints = nullptr)
{
  return semantic_overload::CallAnalysisOptions(instantiate_bodies, hints);
}

inline semantic_overload::CallAnalysisOptions default_call_analysis(
    const semantic_overload::CallAnalysisHints * hints = nullptr)
{
  return call_analysis(true, hints);
}

inline semantic_overload::CallAnalysisOptions without_body_instantiation(
    const semantic_overload::CallAnalysisHints * hints = nullptr)
{
  return call_analysis(false, hints);
}

inline ArgumentConversionOptions default_argument_conversion()
{
  return ArgumentConversionOptions();
}

inline ArgumentConversionOptions allow_explicit_argument_conversion()
{
  return ArgumentConversionOptions(true, true, true, true);
}

inline ArgumentConversionOptions no_output_materialization_argument_conversion()
{
  return ArgumentConversionOptions(true, true, false);
}

inline ArgumentConversionOptions without_user_defined_body_instantiation(
    bool allow_explicit = false)
{
  return ArgumentConversionOptions(true, false, true, allow_explicit);
}

inline ArgumentConversionOptions rematerialization_conversion(
    const ConstructorSelectionOptions & options)
{
  return ArgumentConversionOptions(options.allow_user_defined,
                                   options.instantiate_bodies,
                                   true,
                                   options.allow_explicit);
}

inline ArgumentConversionOptions rematerialization_conversion(
    const semantic_overload::CallAnalysisOptions & options)
{
  return ArgumentConversionOptions(true, options.instantiate_bodies);
}

inline semantic_overload::CallAnalysisOptions apply_analysis_policy(
    const AnalysisPolicy & policy,
    const semantic_overload::CallAnalysisOptions & options)
{
  semantic_overload::CallAnalysisOptions effective = options;
  effective.instantiate_bodies =
      effective.instantiate_bodies && policy.instantiate_function_bodies;
  return effective;
}

inline ArgumentConversionOptions apply_analysis_policy(
    const AnalysisPolicy & policy,
    const ArgumentConversionOptions & options)
{
  ArgumentConversionOptions effective = options;
  effective.allow_user_defined =
      effective.allow_user_defined && policy.allow_user_defined_conversions;
  effective.instantiate_user_defined_bodies =
      effective.instantiate_user_defined_bodies && policy.instantiate_function_bodies;
  effective.materialize_user_defined_output =
      effective.materialize_user_defined_output &&
      policy.materialize_user_defined_output;
  return effective;
}

inline ConstructorSelectionOptions apply_analysis_policy(
    const AnalysisPolicy & policy,
    const ConstructorSelectionOptions & options)
{
  ConstructorSelectionOptions effective = options;
  effective.allow_user_defined =
      effective.allow_user_defined && policy.allow_user_defined_conversions;
  effective.instantiate_bodies =
      effective.instantiate_bodies && policy.instantiate_function_bodies;
  return effective;
}

}  // namespace semantic_policy

class OutputRequirementContext
{
public:
  virtual ~OutputRequirementContext() {}

  virtual CallableEmissionDecision decide_callable_emission(
      semantic_model::FunctionBinding * binding,
      OutputReason reason,
      bool request_definition = true) const = 0;
  virtual semantic_model::FunctionBinding * refresh_required_function_definition(
      semantic_model::FunctionBinding * binding,
      bool insert_if_missing = true) = 0;
  virtual void require_function_definition(semantic_model::FunctionBinding * binding,
                                           OutputReason reason,
                                           bool enabled = true) = 0;
  virtual void upgrade_function_symbol_linkage(
      semantic_model::FunctionBinding * binding,
      symbol_linkage::SymbolLinkage linkage) = 0;
  virtual void note_instantiated_function_output(
      semantic_model::FunctionBinding * binding,
      InstantiatedFunctionOutputMode mode = InstantiatedFunctionOutputMode::TrackOnly) = 0;
};

class TemplateInstantiationContext : public OutputRequirementContext
{
public:
  ~TemplateInstantiationContext() override {}

  virtual semantic_model::FunctionBinding * ensure_function_template_definition(
      semantic_model::FunctionBinding * binding,
      semantic_model::Scope & use_scope) = 0;
};
