#pragma once

#include <functional>
#include <set>
#include <string>
#include <vector>

#include "builtin_type_transforms.h"
#include "callsem_output.h"
#include "cpp_decl_model.h"
#include "semantic_conversion.h"
#include "semantic_model.h"

class SemanticContext;

namespace semantic_builtins {

struct RegistrationHooks
{
  std::function<void(semantic_model::Scope &,
                     const std::string &,
                     const cpp_decl::TypePtr &,
                     const std::vector<cpp_decl::TypePtr> &,
                     bool)> register_function;
};

void register_builtin_types(semantic_model::Scope & scope);
void register_builtin_functions(semantic_model::Scope & scope,
                                const RegistrationHooks & hooks);

cpp_decl::TypePtr gnu_complex_type_for_component(const cpp_decl::TypePtr & component_type);
bool is_gnu_complex_type(const cpp_decl::TypePtr & type,
                         cpp_decl::TypePtr * component_type = nullptr);
bool is_builtin_va_list_type(const cpp_decl::TypePtr & type);

bool expression_is_nothrow(SemanticContext & ctx,
                           semantic_model::Scope & scope,
                           const CppAstNode & expr,
                           bool & out);

bool evaluate_builtin_type_trait(SemanticContext & ctx,
                                 semantic_model::Scope & scope,
                                 const std::string & name,
                                 const cpp_decl::TypePtr & type,
                                 long long & out);
bool evaluate_builtin_binary_type_trait(SemanticContext & ctx,
                                        semantic_model::Scope & scope,
                                        const std::string & name,
                                        const cpp_decl::TypePtr & lhs,
                                        const cpp_decl::TypePtr & rhs,
                                        long long & out);
bool evaluate_builtin_type_trait(SemanticContext & ctx,
                                 semantic_model::Scope & scope,
                                 const std::string & name,
                                 const std::vector<cpp_decl::TypePtr> & types,
                                 long long & out);
bool is_supported_builtin_type_trait_name(const std::string & name);
cpp_decl::TypePtr builtin_type_trait_result_type(const std::string & name);
bool is_supported_builtin_type_transform_name(const std::string & name);

cpp_decl::TypePtr make_dependent_builtin_type_transform_type(
    const std::string & builtin_name,
    const std::string & arg_text,
    const cpp_decl::TypePtr & arg_type);
bool describe_dependent_builtin_type_transform(const cpp_decl::TypePtr & type,
                                               std::string & builtin_name,
                                               cpp_decl::TypePtr & arg_type);
bool is_dependent_builtin_type_transform_type(const cpp_decl::TypePtr & type);
bool try_parse_builtin_type_trait_call_arg(SemanticContext & ctx,
                                           semantic_model::Scope & scope,
                                           const CppAstNode & arg,
                                           cpp_decl::TypePtr & type);
bool try_parse_builtin_type_trait_call(SemanticContext & ctx,
                                       semantic_model::Scope & scope,
                                       const CppAstNode & node,
                                       std::string & trait_name,
                                       std::vector<cpp_decl::TypePtr> & types);
bool try_parse_builtin_type_trait_expression(SemanticContext & ctx,
                                             semantic_model::Scope & scope,
                                             const CppAstNode & node,
                                             std::string & trait_name,
                                             std::vector<cpp_decl::TypePtr> & types);
bool apply_builtin_type_transform(const std::string & name,
                                  const cpp_decl::TypePtr & arg_type,
                                  cpp_decl::TypePtr & out);
bool apply_builtin_type_transform_kind(builtin_type_transforms::Kind kind,
                                       const cpp_decl::TypePtr & arg_type,
                                       cpp_decl::TypePtr & out);
}  // namespace semantic_builtins
