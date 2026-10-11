#pragma once

#include <string>
#include <vector>

#include "cpp_decl_model.h"
#include "cppast_ast.h"
#include "semantic_model.h"
#include "template_model.h"

class SemanticContext;

namespace semantic_lookup {

using namespace semantic_model;

struct MemberValueLookupResult
{
  const ValueBinding * binding = nullptr;
  const ClassInfo * declared_in = nullptr;
  MemberAccess path_access = MA_PUBLIC;
  std::size_t path_offset = 0;
};

struct MemberFunctionLookupResult
{
  std::vector<FunctionBinding *> functions;
  const ClassInfo * declared_in = nullptr;
  MemberAccess path_access = MA_PUBLIC;
  std::size_t path_offset = 0;
};

struct MemberFunctionTemplateLookupResult
{
  std::vector<FunctionTemplateDecl *> templates;
  const ClassInfo * declared_in = nullptr;
  MemberAccess path_access = MA_PUBLIC;
  std::size_t path_offset = 0;
};

struct MemberCallableLookupResult
{
  std::vector<FunctionBinding *> functions;
  std::vector<FunctionTemplateDecl *> templates;
  const ClassInfo * declared_in = nullptr;
  MemberAccess path_access = MA_PUBLIC;
  std::size_t path_offset = 0;
};

struct MemberClassTemplateLookupResult
{
  ClassTemplateDecl * class_template = nullptr;
  const ClassInfo * declared_in = nullptr;
  MemberAccess path_access = MA_PUBLIC;
  std::size_t path_offset = 0;
};

struct MemberAliasTemplateLookupResult
{
  AliasTemplateDecl * alias_template = nullptr;
  const ClassInfo * declared_in = nullptr;
  MemberAccess path_access = MA_PUBLIC;
  std::size_t path_offset = 0;
};

struct MemberVariableTemplateLookupResult
{
  VariableTemplateDecl * variable_template = nullptr;
  const ClassInfo * declared_in = nullptr;
  MemberAccess path_access = MA_PUBLIC;
  std::size_t path_offset = 0;
};

struct QualifiedMemberTarget
{
  bool qualified = false;
  ClassInfo * target_class = nullptr;
  std::string lookup_name;
  MemberAccess path_access = MA_PUBLIC;
  std::size_t path_offset = 0;
};

struct MemberTypeLookupResult
{
  cpp_decl::TypePtr type;
  const ClassInfo * declared_in = nullptr;
  MemberAccess path_access = MA_PUBLIC;
  std::size_t path_offset = 0;
};

Scope * resolve_direct_namespace(Scope & scope, const std::string & name);
cpp_decl::TypePtr resolve_direct_type_qualifier(SemanticContext & ctx,
                                                Scope & scope,
                                                Scope & lookup_scope,
                                                const std::string & name,
                                                const std::vector<cpp_decl::TemplateArgumentSyntax> *
                                                    arg_syntaxes = nullptr,
                                                const cpp_decl::TemplateIdSyntax *
                                                    template_id_syntax = nullptr);
cpp_decl::TypePtr resolve_qualified_owner_type_node(
    SemanticContext & ctx,
    Scope & scope,
    const cpp_decl::QualifiedName & name,
    const CppAstNode & node);
const ValueBinding * lookup_direct_value(Scope & scope, const std::string & name);
bool same_value_binding_entity(const ValueBinding * lhs, const ValueBinding * rhs);
bool resolve_qualified_namespace_entity_target(SemanticContext & ctx,
                                               Scope & scope,
                                               const cpp_decl::QualifiedName & name,
                                               Scope *& out_scope,
                                               std::string & out_name);
Scope * resolve_qualified_variable_parse_scope(SemanticContext & ctx,
                                               Scope & scope,
                                               const CppAstNode & declarator);
std::string scope_qualified_name(const Scope & scope, const std::string & name);
cpp_decl::QualifiedName scope_qualified_name_syntax(const Scope & scope,
                                                    const std::string & name);
std::string scope_symbol_qualified_name(const Scope & scope, const std::string & name);
cpp_decl::QualifiedName scope_symbol_qualified_name_syntax(const Scope & scope,
                                                           const std::string & name);
std::string inline_namespace_collapsed_scope_name(const Scope * scope);
Scope * unqualified_friend_entity_scope(const ClassInfo & info);
bool same_inline_namespace_function_template_entity(const FunctionTemplateDecl * lhs,
                                                    const FunctionTemplateDecl * rhs);
bool same_inline_namespace_class_template_entity(const ClassTemplateDecl * lhs,
                                                 const ClassTemplateDecl * rhs);
bool class_has_friend_class_access(const ClassInfo * current_class,
                                   const ClassInfo * declared_in);
bool same_function_template_entity_type(
    const cpp_decl::TypePtr & lhs_type,
    const std::vector<template_model::TemplateParameterInfo> & lhs_parameters,
    const cpp_decl::TypePtr & rhs_type,
    const std::vector<template_model::TemplateParameterInfo> & rhs_parameters);
bool same_function_template_entity_result_pattern(
    const CppAstNode & lhs,
    const std::vector<template_model::TemplateParameterInfo> & lhs_parameters,
    const CppAstNode & rhs,
    const std::vector<template_model::TemplateParameterInfo> & rhs_parameters);
bool same_inline_namespace_function_entity(const FunctionBinding & lhs,
                                           const FunctionBinding & rhs);
std::string canonical_function_lookup_name(const std::string & name);
std::vector<FunctionBinding *> & direct_function_set_slot(Scope & scope,
                                                          const std::string & name);
void set_direct_function_access_override(Scope & scope,
                                         const std::string & name,
                                         const FunctionBinding * binding,
                                         MemberAccess access);
MemberAccess effective_direct_function_access(const Scope & scope,
                                              const std::string & name,
                                              const FunctionBinding & binding);
std::vector<FunctionTemplateDecl *> & direct_function_template_slot(
    Scope & scope,
    const std::string & name);
const std::vector<FunctionBinding *> * find_direct_function_set(const Scope & scope,
                                                                const std::string & name);
const std::vector<FunctionTemplateDecl *> * find_direct_function_template_set(
    const Scope & scope,
    const std::string & name);

FunctionBinding * current_function_scope(Scope & scope);
ClassInfo * current_class_scope(Scope & scope);
bool is_named_enum_type(SemanticContext & ctx, const cpp_decl::TypePtr & type);

MemberAccess combine_member_access(MemberAccess inherited, MemberAccess edge);
bool is_same_or_derived(const ClassInfo * current, const ClassInfo * target);
bool find_unique_base_path(const ClassInfo & current,
                           const ClassInfo * target,
                           std::size_t & out_offset,
                           MemberAccess & out_access);

MemberValueLookupResult lookup_member_value(ClassInfo & info, const std::string & name);
MemberFunctionLookupResult lookup_member_functions(ClassInfo & info, const std::string & name);
MemberFunctionLookupResult lookup_class_scoped_functions(ClassInfo & info,
                                                         const std::string & name);
MemberFunctionLookupResult lookup_visible_member_functions(ClassInfo & info,
                                                           const std::string & name);
MemberFunctionTemplateLookupResult lookup_visible_member_function_templates(
    ClassInfo & info,
    const std::string & name);
MemberCallableLookupResult lookup_visible_member_callables(ClassInfo & info,
                                                           const std::string & name);
void remove_hidden_using_base_member_function_candidates(
    std::vector<FunctionBinding *> & functions,
    const ClassInfo & current);
MemberClassTemplateLookupResult lookup_member_class_template(SemanticContext & ctx,
                                                             ClassInfo & info,
                                                             const std::string & name);
MemberAliasTemplateLookupResult lookup_member_alias_template(SemanticContext & ctx,
                                                             ClassInfo & info,
                                                             const std::string & name);
MemberVariableTemplateLookupResult lookup_member_variable_template(
    SemanticContext & ctx,
    ClassInfo & info,
    const std::string & name);
MemberTypeLookupResult lookup_member_type(SemanticContext & ctx,
                                          ClassInfo & info,
                                          const std::string & name,
                                          bool ensure_current_reference_members = true,
                                          Scope * lexical_scope = nullptr);
bool resolve_qualified_member_target(SemanticContext & ctx,
                                     Scope & scope,
                                     ClassInfo & object_class,
                                     const cpp_decl::QualifiedName & member_name,
                                     QualifiedMemberTarget & out,
                                     bool allow_dependent_class_qualifiers = true,
                                     const CppAstNode * member_name_node = nullptr);

bool function_has_friend_access(const FunctionBinding * current_function,
                                const ClassInfo * declared_in);
bool member_access_allowed(const Scope * lexical_scope,
                           const ClassInfo * current_class,
                           const FunctionBinding * current_function,
                           const ClassInfo * declared_in,
                           MemberAccess member_access,
                           MemberAccess path_access);
bool member_access_allowed_through_object(const Scope * lexical_scope,
                                          const ClassInfo * current_class,
                                          const FunctionBinding * current_function,
                                          const ClassInfo * object_class,
                                          const ClassInfo * declared_in,
                                          MemberAccess member_access,
                                          MemberAccess path_access);

std::vector<FunctionBinding *> lookup_direct_functions(Scope & scope, const std::string & name);
ClassTemplateDecl * lookup_direct_class_template(Scope & scope, const std::string & name);
AliasTemplateDecl * lookup_direct_alias_template(Scope & scope, const std::string & name);
VariableTemplateDecl * lookup_direct_variable_template(Scope & scope, const std::string & name);
std::vector<FunctionTemplateDecl *> lookup_direct_function_templates(
    Scope & scope,
    const std::string & name);
void collect_direct_function_templates(Scope & scope,
                                       const std::string & name,
                                       std::vector<FunctionTemplateDecl *> & out);

void append_unique_functions(std::vector<FunctionBinding *> & out,
                             const std::vector<FunctionBinding *> & in);
void append_unique_function_templates(std::vector<FunctionTemplateDecl *> & out,
                                      const std::vector<FunctionTemplateDecl *> & in);
void append_unique_scopes(std::vector<Scope *> & out, Scope * scope);

void lookup_functions_from_using_directives(Scope & scope,
                                            const std::string & name,
                                            std::set<const Scope *> & visited,
                                            std::vector<FunctionBinding *> & out);
void lookup_function_templates_from_using_directives(
    Scope & scope,
    const std::string & name,
    std::set<const Scope *> & visited,
    std::vector<FunctionTemplateDecl *> & out);
struct ValueLookupFromUsingDirectivesResult
{
  const ValueBinding * binding;
  bool ambiguous;

  ValueLookupFromUsingDirectivesResult()
    : binding(nullptr), ambiguous(false)
  {}

  explicit ValueLookupFromUsingDirectivesResult(const ValueBinding * binding)
    : binding(binding), ambiguous(false)
  {}

  static ValueLookupFromUsingDirectivesResult make_ambiguous()
  {
    ValueLookupFromUsingDirectivesResult result;
    result.ambiguous = true;
    return result;
  }
};

ValueLookupFromUsingDirectivesResult lookup_value_from_using_directives(
    Scope & scope,
    const std::string & name,
    std::set<const Scope *> & visited);
void lookup_functions_in_scopes(const std::vector<Scope *> & scopes,
                                const std::string & name,
                                std::vector<FunctionBinding *> & out);
void lookup_function_templates_in_scopes(const std::vector<Scope *> & scopes,
                                         const std::string & name,
                                         std::vector<FunctionTemplateDecl *> & out);
void lookup_adl_functions_in_scopes(const std::vector<Scope *> & scopes,
                                    const std::string & name,
                                    std::vector<FunctionBinding *> & out);
void lookup_adl_function_templates_in_scopes(
    const std::vector<Scope *> & scopes,
    const std::string & name,
    std::vector<FunctionTemplateDecl *> & out);
Scope * lookup_namespace_name(Scope & scope, const cpp_decl::QualifiedName & qualified);
void collect_associated_namespace_scopes_for_type(SemanticContext & ctx,
                                                  const cpp_decl::TypePtr & type,
                                                  std::vector<Scope *> & out);
void lookup_associated_friend_functions_for_type(SemanticContext & ctx,
                                                 const cpp_decl::TypePtr & type,
                                                 const std::string & name,
                                                 std::vector<FunctionBinding *> & out);
void lookup_associated_friend_function_templates_for_type(
    SemanticContext & ctx,
    const cpp_decl::TypePtr & type,
    const std::string & name,
    std::vector<FunctionTemplateDecl *> & out);
ClassTemplateDecl * lookup_unqualified_class_template(Scope & scope,
                                                      const std::string & name);
AliasTemplateDecl * lookup_unqualified_alias_template(Scope & scope,
                                                      const std::string & name);
ClassTemplateDecl * lookup_class_template(SemanticContext & ctx,
                                          Scope & scope,
                                          const std::string & name);
ClassTemplateDecl * lookup_class_template(SemanticContext & ctx,
                                          Scope & scope,
                                          const cpp_decl::QualifiedName & name);
ClassTemplateDecl * lookup_class_template_node(
    SemanticContext & ctx,
    Scope & scope,
    const cpp_decl::QualifiedName & name,
    const CppAstNode & node);
AliasTemplateDecl * lookup_alias_template(SemanticContext & ctx,
                                          Scope & scope,
                                          const std::string & name);
AliasTemplateDecl * lookup_alias_template(SemanticContext & ctx,
                                          Scope & scope,
                                          const cpp_decl::QualifiedName & name);
AliasTemplateDecl * lookup_alias_template_node(
    SemanticContext & ctx,
    Scope & scope,
    const cpp_decl::QualifiedName & name,
    const CppAstNode & node);
std::vector<FunctionTemplateDecl *> lookup_function_templates(SemanticContext & ctx,
                                                              Scope & scope,
                                                              const std::string & name);
void collect_function_templates(SemanticContext & ctx,
                                Scope & scope,
                                const std::string & name,
                                std::vector<FunctionTemplateDecl *> & out);
void collect_function_templates(SemanticContext & ctx,
                                Scope & scope,
                                const cpp_decl::QualifiedName & qualified,
                                std::vector<FunctionTemplateDecl *> & out);
VariableTemplateDecl * lookup_variable_template(SemanticContext & ctx,
                                                Scope & scope,
                                                const std::string & name);
VariableTemplateDecl * lookup_variable_template(SemanticContext & ctx,
                                                Scope & scope,
                                                const cpp_decl::QualifiedName & qualified);
VariableTemplateDecl * lookup_variable_template_node(
    SemanticContext & ctx,
    Scope & scope,
    const cpp_decl::QualifiedName & qualified,
    const CppAstNode & node);
Scope * resolve_qualified_scope_for_class_or_namespace(SemanticContext & ctx,
                                                       Scope & scope,
                                                       const cpp_decl::QualifiedName & qualified,
                                                       bool allow_dependent_class_qualifiers = false);
CppAstNode make_value_qualifier_type_lookup_node(const CppAstNode & node,
                                                 const cpp_decl::QualifiedName & qualified,
                                                 const std::string & qualifier_name);
const ValueBinding * lookup_qualified_value_binding(SemanticContext & ctx,
                                                    Scope & scope,
                                                    const cpp_decl::QualifiedName & qualified);
const ValueBinding * lookup_qualified_value_binding_node(
    SemanticContext & ctx,
    Scope & scope,
    const cpp_decl::QualifiedName & qualified,
    const CppAstNode & node,
    cpp_decl::TypePtr * qualifier_type_out = nullptr);

}  // namespace semantic_lookup
