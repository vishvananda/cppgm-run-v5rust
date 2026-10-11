#pragma once

#include "semantic_context.h"

namespace semantic_lookup {
struct MemberValueLookupResult;
}

namespace semantic_expression {

semantic_conversion::ExprInfo analyze_expression(SemanticContext & ctx,
                                                 semantic_model::Scope & scope,
                                                 const CppAstNode & node);

semantic_conversion::ExprInfo analyze_expression_for_target(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node,
    const cpp_decl::TypePtr & target);

semantic_conversion::ExprInfo make_initializer_list_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const cpp_decl::TypePtr & initlist_type,
    const CppAstNode & node);

bool try_analyze_array_braced_init_list_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const cpp_decl::TypePtr & expr_type,
    const CppAstNode & node,
    semantic_conversion::ExprInfo & out);

semantic_conversion::ExprInfo analyze_this_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_member_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_literal(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_id_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

bool try_analyze_reference_binding_source_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node,
    semantic_conversion::ExprInfo & out);

semantic_conversion::ExprInfo make_implicit_member_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const semantic_lookup::MemberValueLookupResult & member);

semantic_conversion::ExprInfo analyze_unary_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_postfix_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_conditional_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_binary_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_subscript_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_sizeof_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_sizeof_pack_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_braced_init_list_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_type_trait_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_cast_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_lambda_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_lambda_expression_as_closure(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo analyze_assignment_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node);

semantic_conversion::ExprInfo make_value_initialized_expr(
    const cpp_decl::TypePtr & type);

}  // namespace semantic_expression
