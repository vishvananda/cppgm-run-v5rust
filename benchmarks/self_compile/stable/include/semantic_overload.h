#pragma once

#include <cstddef>
#include <string>
#include <vector>

#include "cpp_decl_model.h"
#include "semantic_context.h"

namespace semantic_overload {

struct NonmemberOperatorCandidateSet
{
  std::vector<semantic_model::FunctionBinding *> functions;
  std::vector<semantic_model::FunctionTemplateDecl *> templates;
  std::vector<semantic_model::Scope *> associated_scopes;
};

enum AssociatedFriendTemplateMergeOrder
{
  MERGE_FRIEND_TEMPLATES_BEFORE_ASSOCIATED_SCOPES,
  MERGE_FRIEND_TEMPLATES_AFTER_ASSOCIATED_SCOPES
};

void collect_nonmember_operator_candidates(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::string & operator_name,
    const std::vector<cpp_decl::TypePtr> & operand_types,
    std::size_t argument_count,
    NonmemberOperatorCandidateSet & out,
    AssociatedFriendTemplateMergeOrder friend_template_order =
        MERGE_FRIEND_TEMPLATES_AFTER_ASSOCIATED_SCOPES);

void initialize_operator_candidate_scope(
    semantic_model::Scope & operator_scope,
    semantic_model::Scope & source_scope,
    const std::string & operator_name,
    const std::vector<semantic_model::Scope *> & associated_scopes,
    const std::vector<semantic_model::FunctionBinding *> & functions,
    const std::vector<semantic_model::FunctionTemplateDecl *> & templates);

bool resolve_function_id_for_target(SemanticContext & ctx,
                                    semantic_model::Scope & scope,
                                    const std::string & name,
                                    const cpp_decl::TypePtr & target,
                                    semantic_conversion::ExprInfo & out,
                                    const cpp_decl::QualifiedName * name_syntax = nullptr,
                                    const cpp_decl::TemplateIdSyntax * name_template_id_syntax = nullptr,
                                    const CppAstNode * name_node = nullptr,
                                    bool require_output_definition = true);

bool resolve_member_function_id_for_target(SemanticContext & ctx,
                                           semantic_model::Scope & scope,
                                           const CppAstNode & unary_node,
                                           const cpp_decl::TypePtr & target,
                                           semantic_conversion::ExprInfo & out);

semantic_model::FunctionBinding * select_constructor_from_exprs(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo & info,
    const std::vector<semantic_conversion::ExprInfo> & source_args,
    std::vector<semantic_conversion::ExprInfo> & args_out,
    std::vector<semantic_conversion::ConversionRank> * ranks_out = nullptr,
    const ConstructorSelectionOptions & options = ConstructorSelectionOptions());

semantic_model::FunctionBinding * select_constructor(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo & info,
    const std::vector<const CppAstNode *> & arg_nodes,
    std::vector<semantic_conversion::ExprInfo> & args_out,
    const ConstructorSelectionOptions & options = ConstructorSelectionOptions());

semantic_model::FunctionBinding * select_constructor_for_direct_braced_init(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo & info,
    const CppAstNode & direct_braced_init,
    std::vector<semantic_conversion::ExprInfo> & args_out,
    const ConstructorSelectionOptions & options = ConstructorSelectionOptions());

semantic_conversion::ExprInfo analyze_functional_cast(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const cpp_decl::TypePtr & callee_type,
    const std::vector<const CppAstNode *> & arg_nodes,
    const CppAstNode * direct_braced_init,
    const CallAnalysisOptions & options = CallAnalysisOptions());

semantic_conversion::ExprInfo analyze_adl_only_call_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const std::string & name,
    const std::vector<const CppAstNode *> & arg_nodes,
    const CallAnalysisOptions & options = CallAnalysisOptions());

semantic_conversion::ExprInfo analyze_call_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node,
    const CallAnalysisOptions & options = CallAnalysisOptions());

semantic_conversion::ExprInfo analyze_overloaded_assignment_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node,
    const semantic_conversion::ExprInfo & lhs);

}  // namespace semantic_overload
