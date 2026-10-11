#pragma once

#include "callsem_output.h"
#include "cpp_decl_model.h"
#include "semantic_context_facets.h"
#include "semantic_model.h"

class SemanticContext;

namespace semantic_conversion {

using namespace semantic_model;

enum ValueCategory : int
{
  VC_LVALUE,
  VC_PRVALUE,
  VC_XVALUE
};

struct ExprInfo
{
  ExprInfo();
  ExprInfo(const ExprInfo & other);
  ExprInfo & operator=(const ExprInfo & other);
  ExprInfo(ExprInfo && other);
  ExprInfo & operator=(ExprInfo && other);

  cpp_decl::TypePtr type;
  ValueCategory category = VC_PRVALUE;
  CallSemNode node;
  bool null_pointer_constant = false;
};

enum ConversionRank : int
{
  CR_EXACT = 0,
  CR_PROMOTION = 1,
  CR_CONVERSION = 2,
  CR_USER_DEFINED = 3,
  CR_ELLIPSIS = 4,
  CR_BAD = 5
};

cpp_decl::TypePtr value_conversion_type(const ExprInfo & expr);
ConversionRank standard_conversion_rank_non_reference(const cpp_decl::TypePtr & target,
                                                      const ExprInfo & expr);
ConversionRank standard_conversion_rank(const cpp_decl::TypePtr & target,
                                        const ExprInfo & expr);
bool try_semantic_exact_reference_binding(SemanticContext & ctx,
                                          const cpp_decl::TypePtr & target,
                                          const ExprInfo & expr,
                                          ExprInfo & out,
                                          ConversionRank & rank);
bool ref_qualifier_accepts_implicit_object(semantic_model::RefQualifier ref_qualifier,
                                           const cpp_decl::TypePtr & implicit_object_parameter,
                                           ValueCategory category);
bool member_pointer_ref_qualifier_rejects_object(
    cpp_decl::FunctionTypeRefQualifier qualifier,
    ValueCategory category);
void apply_standard_conversion_result_metadata(SemanticContext & ctx,
                                               const cpp_decl::TypePtr & target,
                                               const ExprInfo & expr,
                                               ExprInfo & out);
ConversionRank inheritance_conversion_rank(SemanticContext & ctx,
                                           const cpp_decl::TypePtr & target,
                                           const ExprInfo & arg);
ConversionRank conversion_rank(SemanticContext & ctx,
                               const cpp_decl::TypePtr & target,
                               const ExprInfo & arg);
bool can_copy_initialize(SemanticContext & ctx,
                         const cpp_decl::TypePtr & target,
                         const ExprInfo & expr);
bool supports_non_reference_explicit_cast(SemanticContext & ctx,
                                          const cpp_decl::TypePtr & target,
                                          const ExprInfo & expr,
                                          bool allow_reinterpret_like);
bool top_level_cv_flags(const cpp_decl::TypePtr & type,
                        cpp_decl::TypePtr & base,
                        bool & cv_const,
                        bool & cv_volatile);
bool same_type_with_compatible_top_cv(const cpp_decl::TypePtr & target,
                                      const cpp_decl::TypePtr & source);
bool class_value_transfer_prefers_nonconst_move(
    const cpp_decl::TypePtr & target,
    const cpp_decl::TypePtr & source,
    CallValueCategory source_category);
bool is_const_object_type(const cpp_decl::TypePtr & type);
bool is_unscoped_enum_type(const cpp_decl::TypePtr & type);
bool is_integral_or_unscoped_enum_type(const cpp_decl::TypePtr & type);
cpp_decl::TypePtr promoted_integral_type(const cpp_decl::TypePtr & type);
bool is_condition_test_type(const cpp_decl::TypePtr & type);
bool try_condition_test_conversion(SemanticContext & ctx,
                                   semantic_model::Scope & scope,
                                   ExprInfo & expr);
int compare_reference_binding_preference(const cpp_decl::TypePtr & lhs_param,
                                         const ExprInfo & lhs_arg,
                                         const cpp_decl::TypePtr & rhs_param,
                                         const ExprInfo & rhs_arg);
int compare_standard_conversion_preference(const cpp_decl::TypePtr & lhs_param,
                                           const ExprInfo & lhs_arg,
                                           const cpp_decl::TypePtr & rhs_param,
                                           const ExprInfo & rhs_arg);
int compare_parameter_qualification_preference(const cpp_decl::TypePtr & lhs,
                                               const cpp_decl::TypePtr & rhs);
int compare_qualification_conversion_preference(const cpp_decl::TypePtr & lhs_param,
                                                const ExprInfo & lhs_arg,
                                                const cpp_decl::TypePtr & rhs_param,
                                                const ExprInfo & rhs_arg);
bool pointer_equality_operands_compatible(const cpp_decl::TypePtr & lhs,
                                          const cpp_decl::TypePtr & rhs);
bool pointer_subtraction_operands_compatible(const cpp_decl::TypePtr & lhs,
                                             const cpp_decl::TypePtr & rhs);
cpp_decl::TypePtr promoted_integral_result_type(const cpp_decl::TypePtr & type);
cpp_decl::TypePtr common_integral_result_type(const cpp_decl::TypePtr & lhs,
                                              const cpp_decl::TypePtr & rhs);
cpp_decl::TypePtr common_arithmetic_result_type(const cpp_decl::TypePtr & lhs,
                                                const cpp_decl::TypePtr & rhs);
bool try_apply_inheritance_conversion(SemanticContext & ctx,
                                      const cpp_decl::TypePtr & target,
                                      const ExprInfo & expr,
                                      ExprInfo & out);
bool try_apply_unmaterialized_inheritance_conversion(SemanticContext & ctx,
                                                     const cpp_decl::TypePtr & target,
                                                     const ExprInfo & expr,
                                                     ExprInfo & out);
bool try_argument_conversion(SemanticContext & ctx,
                             Scope & scope,
                             const cpp_decl::TypePtr & target,
                             const ExprInfo & expr,
                             ExprInfo & out,
                             ConversionRank & rank,
                             const ArgumentConversionOptions & options =
                                 ArgumentConversionOptions());
bool try_builtin_pointer_operand_conversion(
    SemanticContext & ctx,
    Scope & scope,
    const ExprInfo & expr,
    ExprInfo & out,
    cpp_decl::TypePtr & pointer_type,
    const ArgumentConversionOptions & options = ArgumentConversionOptions());
bool is_modifiable_lvalue(const ExprInfo & expr);
bool result_value_category_for_function_result(const cpp_decl::TypePtr & result_type,
                                               ValueCategory & out);
cpp_decl::TypePtr expression_type_for_function_result(
    const cpp_decl::TypePtr & result_type);
ConversionRank implicit_object_conversion_rank(SemanticContext & ctx,
                                               const cpp_decl::TypePtr & target,
                                               const ExprInfo & arg);

}  // namespace semantic_conversion
