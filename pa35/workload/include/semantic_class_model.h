#pragma once

#include <string>
#include <utility>
#include <vector>

#include "cpp_decl_model.h"
#include "semantic_model.h"

class SemanticContext;

namespace semantic_class_model {

using ClassFunctionOptions = semantic_model::FunctionSemanticFlags;

struct MethodSyntaxInfo
{
  CppAstNode filtered_specifiers;
  CppAstNode filtered_declarator;
  bool decl_static = false;
  bool decl_virtual = false;
  bool decl_explicit = false;
  bool is_override = false;
  bool is_final = false;
  bool is_const_method = false;
  bool is_volatile_method = false;
  bool is_variadic = false;
  semantic_model::RefQualifier ref_qualifier = semantic_model::RQ_NONE;
  const CppAstNode * function_qualifier = nullptr;
};

struct PreparedMethodParseContext
{
  bool has_method_syntax = false;
  bool uses_filtered_parse = false;
  MethodSyntaxInfo syntax;
  const CppAstNode * source_specifiers = nullptr;
  const CppAstNode * source_declarator = nullptr;

  void set_parse_sources(const CppAstNode * specifiers,
                         const CppAstNode * declarator)
  {
    source_specifiers = specifiers;
    source_declarator = declarator;
  }

  const CppAstNode * parse_specifiers_node() const
  {
    return uses_filtered_parse ? (source_specifiers ? &syntax.filtered_specifiers : nullptr)
                               : source_specifiers;
  }

  const CppAstNode & parse_declarator_node() const
  {
    return uses_filtered_parse ? syntax.filtered_declarator : *source_declarator;
  }
};

struct PreparedClassMemberDeclarationContext
{
  CppAstNode resolved_specifiers;
  CppAstNode filtered_specifiers;
  bool parsed_decl_spec = false;
  bool declaration_is_typedef = false;
  cpp_decl::TypePtr base;
};

struct PreparedClassMemberFunctionDefinition
{
  const CppAstNode * specifiers = nullptr;
  const CppAstNode * declarator = nullptr;
  const CppAstNode * body = nullptr;
  PreparedMethodParseContext method;
  std::string name;
  cpp_decl::TypePtr base;
  cpp_decl::TypePtr declared_type;
  bool is_static_member = false;
  bool is_constexpr_member = false;
  bool is_inline_member = false;
};

semantic_model::MemberAccess default_access_for_class_kind(const std::string & class_kind);
bool class_member_specifiers_supported(const CppAstNode & specifiers,
                                       bool allow_inline_virtual);
bool declarator_is_const_method(const CppAstNode & declarator);
bool declarator_is_volatile_method(const CppAstNode & declarator);
semantic_model::RefQualifier declarator_ref_qualifier(const CppAstNode & declarator);
const CppAstNode * declarator_function_qualifier(const CppAstNode & declarator);
CppAstNode filtered_class_member_decl_specifiers(const CppAstNode & specifiers);
void analyze_method_syntax(const CppAstNode * specifiers,
                           const CppAstNode & declarator,
                           MethodSyntaxInfo & out);
void prepare_method_parse_context(
    const CppAstNode * specifiers,
    const CppAstNode & declarator,
    PreparedMethodParseContext & out,
    bool has_method_syntax = true,
    bool require_parameter_clause_for_filtered_parse = false);
bool prepare_class_member_declaration_context(
    SemanticContext & ctx,
    semantic_model::Scope & member_scope,
    const CppAstNode & specifiers,
    const CppAstNode * declarators,
    bool collect_embedded_types,
    bool collect_named_forward_declarations,
    bool reference_class_templates_only,
    PreparedClassMemberDeclarationContext & out);
bool prepare_class_member_function_definition(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info,
    const CppAstNode & node,
    bool reference_class_templates_only,
    PreparedClassMemberFunctionDefinition & out);
ClassFunctionOptions class_function_options(semantic_model::MemberAccess access,
                                            const MethodSyntaxInfo * syntax = nullptr,
                                            bool is_constructor = false,
                                            bool is_destructor = false,
                                            bool is_constexpr = false,
                                            bool is_defaulted = false,
                                            bool is_inline = false);
void validate_method_virtual_syntax(const MethodSyntaxInfo & info);
cpp_decl::TypePtr method_function_type(const cpp_decl::TypePtr & class_type,
                                       bool is_const_method,
                                       bool is_volatile_method,
                                       const cpp_decl::TypePtr & declared_type);
bool try_parse_conversion_operator_result_type(SemanticContext & ctx,
                                               semantic_model::Scope & scope,
                                               const CppAstNode & declarator,
                                               cpp_decl::TypePtr & out);
bool parse_conversion_operator_signature(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & node,
    std::string & member_name,
    cpp_decl::TypePtr & declared_type,
    std::vector<std::pair<std::string, cpp_decl::TypePtr> > & params,
    std::vector<const CppAstNode *> * default_args = nullptr,
    MethodSyntaxInfo * syntax_out = nullptr);
cpp_decl::TypePtr resolve_instantiated_member_alias_type(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const cpp_decl::TypePtr & type,
    semantic_model::ClassInfo * current_info = nullptr);

void reset_instantiated_class_info(semantic_model::ClassInfo & info,
                                   const std::string & template_name,
                                   const CppAstNode * output_node);
void invalidate_forward_class_reference_members(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info);

void finalize_class_virtuals(SemanticContext & ctx,
                             semantic_model::ClassInfo & info);
bool class_has_virtual_bases(const semantic_model::ClassInfo & info);
bool class_uses_extended_virtual_abi(const semantic_model::ClassInfo & info);
bool class_needs_vtt(const semantic_model::ClassInfo & info);
std::string construction_vtable_key(const semantic_model::ClassInfo & dynamic_class,
                                    const semantic_model::ClassInfo & base_class,
                                    std::size_t base_offset);
void collect_construction_vtables(SemanticContext & ctx,
                                  semantic_model::ClassInfo & dynamic_class,
                                  std::vector<semantic_model::VTableInfo> & out);
bool find_vtt_direct_base_slice_offset(SemanticContext & ctx,
                                       semantic_model::ClassInfo & dynamic_class,
                                       semantic_model::ClassInfo & base_class,
                                       std::size_t & out_byte_offset);
bool find_vtt_self_table_index(SemanticContext & ctx,
                               semantic_model::ClassInfo & info,
                               const std::string & table_key,
                               std::size_t & out_index);
void collect_vtt_entries(SemanticContext & ctx,
                         semantic_model::ClassInfo & info,
                         std::vector<std::pair<std::string, unsigned long long> > & out);
void finalize_class_layout(SemanticContext & ctx,
                           semantic_model::ClassInfo & info);
void populate_class_info(SemanticContext & ctx,
                         semantic_model::ClassInfo & info,
                         const CppAstNode & node);
void collect_class_simple_declaration(SemanticContext & ctx,
                                      semantic_model::ClassInfo & info,
                                      const CppAstNode & node,
                                      semantic_model::MemberAccess access);
void collect_dependent_class_simple_declaration(SemanticContext & ctx,
                                                semantic_model::ClassInfo & info,
                                                const CppAstNode & node,
                                                semantic_model::MemberAccess access);
void collect_class_bit_field_declaration(SemanticContext & ctx,
                                         semantic_model::ClassInfo & info,
                                         const CppAstNode & node,
                                         semantic_model::MemberAccess access);
void collect_dependent_class_bit_field_declaration(SemanticContext & ctx,
                                                   semantic_model::ClassInfo & info,
                                                   const CppAstNode & node,
                                                   semantic_model::MemberAccess access);
void collect_class_method_definition(SemanticContext & ctx,
                                     semantic_model::ClassInfo & info,
                                     const CppAstNode & node,
                                     semantic_model::MemberAccess access);
void collect_special_member(SemanticContext & ctx,
                            semantic_model::ClassInfo & info,
                            const CppAstNode & node,
                            semantic_model::MemberAccess access);
void collect_conversion_operator_member(SemanticContext & ctx,
                                        semantic_model::ClassInfo & info,
                                        const CppAstNode & node,
                                        semantic_model::MemberAccess access);
void finalize_class_constant_members(SemanticContext & ctx,
                                     semantic_model::ClassInfo & info);
void ensure_class_reference_members(SemanticContext & ctx,
                                    semantic_model::ClassInfo & info);
bool resolve_deferred_class_alias(SemanticContext & ctx,
                                  semantic_model::ClassInfo & info,
                                  const std::string & alias_name,
                                  cpp_decl::TypePtr & out);
void collect_class_declaration(SemanticContext & ctx,
                               semantic_model::Scope & scope,
                               const CppAstNode & node);
bool is_anonymous_union_specifier(const CppAstNode & node);
std::string scope_anonymous_union_type_name(const CppAstNode & node);
std::string scope_anonymous_union_storage_name(const CppAstNode & node);
bool synthesize_anonymous_union_storage_declaration(const CppAstNode & node,
                                                    CppAstNode & out_decl,
                                                    std::string & out_type_name,
                                                    std::string & out_storage_name);
void inject_anonymous_union_variable_bindings(semantic_model::Scope & scope,
                                              semantic_model::ClassInfo & storage_info,
                                              const std::string & storage_name);

void ensure_implicit_special_members(SemanticContext & ctx,
                                     semantic_model::ClassInfo & info);
bool is_trivially_default_constructible_type_for_host_abi(
    SemanticContext & ctx,
    const cpp_decl::TypePtr & type);
bool is_trivially_destructible_type_for_host_abi(
    SemanticContext & ctx,
    const cpp_decl::TypePtr & type);
bool is_trivially_copy_constructible_type_for_host_abi(
    SemanticContext & ctx,
    const cpp_decl::TypePtr & type);
bool is_trivially_move_constructible_type_for_host_abi(
    SemanticContext & ctx,
    const cpp_decl::TypePtr & type);
semantic_model::FunctionBinding * ensure_implicit_copy_constructor(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info);
semantic_model::FunctionBinding * ensure_implicit_move_constructor(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info);
semantic_model::FunctionBinding * ensure_implicit_move_assignment(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info);
bool can_synthesize_aggregate_constructor(const semantic_model::ClassInfo & info);
bool class_has_bit_fields(const semantic_model::ClassInfo & info);
std::size_t aggregate_element_count(const semantic_model::ClassInfo & info);
const semantic_model::FieldInfo * first_aggregate_field(
    const semantic_model::ClassInfo & info);
semantic_model::ClassInfo * anonymous_storage_union_info(
    SemanticContext & ctx,
    const semantic_model::FieldInfo & field);
const semantic_model::FieldInfo * aggregate_input_field(
    SemanticContext & ctx,
    const semantic_model::FieldInfo & field);
cpp_decl::TypePtr aggregate_constructor_parameter_type(
    SemanticContext & ctx,
    const semantic_model::FieldInfo & field);
semantic_model::FunctionBinding * ensure_implicit_aggregate_constructor(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info);
semantic_model::FunctionBinding * ensure_implicit_aggregate_constructor(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info,
    std::size_t explicit_arg_count);
semantic_model::FunctionBinding * ensure_implicit_copy_assignment(
    SemanticContext & ctx,
    semantic_model::ClassInfo & info);

}  // namespace semantic_class_model
