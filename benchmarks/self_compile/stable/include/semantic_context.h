#pragma once

#include <map>
#include <set>
#include <string>
#include <utility>
#include <vector>

#include "analysis_policy.h"
#include "cpp_decl_model.h"
#include "cppast_ast.h"
#include "function_template_identity.h"
#include "semantic_context_facets.h"
#include "semantic_model.h"
#include "semantic_source_use.h"
#include "symbol_linkage.h"
#include "template_service_interfaces.h"

namespace semantic_model {
struct Scope;
struct ClassInfo;
struct FunctionBinding;
struct ValueBinding;
struct FieldInfo;
struct ClassTemplateDecl;
struct AliasTemplateDecl;
struct FunctionTemplateDecl;
struct VariableTemplateDecl;
struct PartialClassTemplateSpecializationDecl;
struct VariableTemplateSpecializationDecl;
enum MemberAccess : int;
enum RefQualifier : int;
}  // namespace semantic_model

namespace template_model {
struct TemplateParameterInfo;
struct TemplateArgument;
}  // namespace template_model

namespace template_api {
struct ClassSpecializationSelection;
}  // namespace template_api

namespace semantic_overload {
struct CallAnalysisOptions;
}  // namespace semantic_overload

namespace semantic_conversion {
struct ExprInfo;
enum ValueCategory : int;
enum ConversionRank : int;
}  // namespace semantic_conversion

namespace constant_eval {
struct ConstexprValue;
struct LocalDeclaration;
class Evaluator;
}  // namespace constant_eval

namespace semantic_class_model {
using ClassFunctionOptions = semantic_model::FunctionSemanticFlags;
}  // namespace semantic_class_model

namespace semantic_metrics {
struct AnalyzerCounters;
}  // namespace semantic_metrics

namespace semantic_consteval {
struct OffsetofFieldInfo;
}  // namespace semantic_consteval

namespace template_api {
struct TemplateWitnessContext;
}  // namespace template_api

struct CallSemNode;

struct FunctionRegistrationRequest
{
  semantic_model::Scope * scope = nullptr;
  semantic_model::ClassInfo * owner_class = nullptr;
  std::string name;
  cpp_decl::TypePtr declared_type;
  std::vector<std::pair<std::string, cpp_decl::TypePtr> > params;
  std::vector<const CppAstNode *> default_arguments;
  const CppAstNode * body = nullptr;
  const CppAstNode * ctor_initializer = nullptr;
  const CppAstNode * declaration_node = nullptr;
  const CppAstNode * parameter_syntax_node = nullptr;
  semantic_model::FunctionSemanticFlags semantic_flags;
  bool is_static_member = false;
  bool is_c_linkage = false;
  semantic_model::Scope * declaration_scope = nullptr;
  const CppAstNode * function_qualifier = nullptr;
  bool is_constexpr = false;
  FunctionTemplateRegistrationIdentity template_identity;
  semantic_model::ClassInfo * lexical_access_class = nullptr;
  semantic_model::FunctionBinding * lexical_access_function = nullptr;
  bool hidden_friend_only = false;
};

struct ClassTemplateInfoCreationRequest
{
  semantic_model::Scope * scope = nullptr;
  std::string class_kind;
  std::string template_name;
  std::string specialization_name;
  std::string internal_specialization_name;
  semantic_model::ClassTemplateDecl * template_decl = nullptr;
  const CppAstNode * output_node = nullptr;
  bool track_output = true;
};

class SemanticContext : public TemplateInstantiationContext
{
public:
  ~SemanticContext() override {}

  virtual std::string source_location_for_node(const CppAstNode & node) const = 0;
  virtual std::string source_location_for_name_in_node(const CppAstNode & node,
                                                       const std::string & name,
                                                       bool prefer_last = false) const = 0;
  virtual template_api::TemplateWitnessContext template_witness_context() const = 0;
  virtual void emit_nested_class_use_source_events_from_location(
      semantic_model::Scope & scope,
      const std::string & location,
      semantic_source_use::SourceUseOwnership ownership) = 0;
  virtual void emit_class_use_source_events_after_location(
      semantic_model::Scope & scope,
      const std::string & location,
      semantic_source_use::SourceUseOwnership ownership) = 0;
  virtual void emit_nested_class_use_source_events_from_ast_node(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      semantic_source_use::SourceUseOwnership ownership,
      bool allow_source_template_header_replay = false) = 0;
  virtual void emit_nested_class_use_source_events_from_template_arguments(
      semantic_model::Scope & scope,
      const std::vector<cpp_decl::TemplateArgumentSyntax> & syntaxes,
      semantic_source_use::SourceUseOwnership ownership) = 0;
  virtual void emit_static_member_definition_class_use_source_events_from_ast_node(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      semantic_source_use::SourceUseOwnership ownership) = 0;
  virtual void record_deduced_class_use_for_resolved_alias_type(
      semantic_model::Scope & scope,
      const cpp_decl::TypePtr & type,
      const std::string & use_location,
      semantic_source_use::SourceUseRole role =
          semantic_source_use::SourceUseRole::TypeUse) = 0;
  virtual void record_class_use_for_resolved_type_node(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      const cpp_decl::TypePtr & type,
      const std::string & node_use_location,
      bool allow_concrete_dependent_argument_spelling = false) = 0;
  virtual void record_declaration_type_class_use_for_resolved_type_node(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      const cpp_decl::TypePtr & type,
      const std::string & node_use_location,
      bool allow_concrete_dependent_argument_spelling = false,
      bool clear_template_id_occurrence = false) = 0;
  virtual void record_primary_alias_base_source_uses(
      semantic_model::ClassTemplateDecl & decl) = 0;
  virtual bool node_comes_from_standard_include_path(
      const CppAstNode * node) const = 0;
  virtual bool definition_comes_from_standard_include_path(
      const CppAstNode * declaration_node,
      const CppAstNode * body,
      bool is_defaulted) const = 0;

  virtual cpp_decl::TypePtr lookup_type(semantic_model::Scope & scope,
                                        const std::string & name,
                                        bool reference_class_templates_only = false) = 0;
  virtual cpp_decl::TypePtr lookup_type_node(semantic_model::Scope & scope,
                                             const CppAstNode & node,
                                             const std::string & name,
                                             bool reference_class_templates_only = false) = 0;
  virtual semantic_model::ClassInfo * lookup_declared_class_info(
      semantic_model::Scope & scope,
      const std::string & text) = 0;
  virtual cpp_decl::TypePtr lookup_non_template_type_name(
      semantic_model::Scope & scope,
      const std::string & text) = 0;
  virtual cpp_decl::TypePtr lookup_non_template_type_name(
      semantic_model::Scope & scope,
      const cpp_decl::QualifiedName & name) = 0;
  virtual cpp_decl::TypePtr maybe_introduce_elaborated_type(
      semantic_model::Scope & scope,
      const std::string & class_kind,
      const cpp_decl::QualifiedName & name) = 0;
  virtual bool parse_type_id(semantic_model::Scope & scope,
                             const CppAstNode & node,
                             cpp_decl::TypePtr & type,
                             bool reference_class_templates_only = false,
                             bool record_class_template_use = true) = 0;
  virtual const std::vector<cpp_decl::TypePtr> * lookup_type_pack(
      semantic_model::Scope & scope,
      const std::string & name) = 0;
  virtual cpp_decl::TypePtr lookup_exact_local_type_name(
      semantic_model::Scope & scope,
      const std::string & name) const = 0;
  virtual cpp_decl::TypePtr lookup_exact_bound_type_name(
      semantic_model::Scope & scope,
      const std::string & name) const = 0;
  virtual bool scope_has_template_placeholders(semantic_model::Scope & scope) const = 0;
  virtual bool should_materialize_direct_call_output(
      const semantic_model::FunctionBinding & binding) const = 0;
  virtual const AnalysisPolicy & current_analysis_policy() const = 0;
  virtual void request_function_definition_semantic_validation(
      semantic_model::FunctionBinding * binding) = 0;
  virtual bool expand_output_closure_enabled() const = 0;
  virtual bool emit_all_source_function_definitions() const = 0;
  virtual semantic_metrics::AnalyzerCounters * performance_counters() = 0;
  virtual bool function_binding_is_live(
      const semantic_model::FunctionBinding * binding) const = 0;
  virtual const CppAstNode * materialize_lazy_function_body(const CppAstNode & body) = 0;
  virtual void note_late_required_class_method(
      semantic_model::FunctionBinding * binding) = 0;
  virtual semantic_conversion::ExprInfo analyze_expression(semantic_model::Scope & scope,
                                                           const CppAstNode & expr) = 0;
  virtual semantic_conversion::ExprInfo analyze_expression_without_output_materialization(
      semantic_model::Scope & scope,
      const CppAstNode & expr) = 0;
  virtual semantic_conversion::ExprInfo analyze_expression_for_target(
      semantic_model::Scope & scope,
      const CppAstNode & expr,
      const cpp_decl::TypePtr & target) = 0;
  virtual semantic_conversion::ExprInfo analyze_call_expression(
      semantic_model::Scope & scope,
      const CppAstNode & expr,
      const semantic_overload::CallAnalysisOptions & options =
          semantic_overload::CallAnalysisOptions()) = 0;
  virtual semantic_conversion::ExprInfo analyze_braced_init_list_expression(
      semantic_model::Scope & scope,
      const CppAstNode & node) = 0;
  virtual semantic_conversion::ExprInfo analyze_type_trait_expression(
      semantic_model::Scope & scope,
      const CppAstNode & node) = 0;
  virtual semantic_conversion::ExprInfo analyze_cast_expression(
      semantic_model::Scope & scope,
      const CppAstNode & node) = 0;
  virtual bool try_analyze_target_aware_expression(
      semantic_model::Scope & scope,
      const CppAstNode & expr,
      const cpp_decl::TypePtr & target,
      semantic_conversion::ExprInfo & out,
      const ConstructorSelectionOptions * ctor_options = nullptr) = 0;
  virtual semantic_conversion::ExprInfo analyze_this_expression(
      semantic_model::Scope & scope,
      const CppAstNode & node) = 0;
  virtual semantic_conversion::ExprInfo analyze_id_expression(
      semantic_model::Scope & scope,
      const CppAstNode & node) = 0;
  virtual semantic_conversion::ExprInfo analyze_lambda_expression(
      semantic_model::Scope & scope,
      const CppAstNode & node) = 0;
  virtual semantic_conversion::ExprInfo analyze_assignment_expression(
      semantic_model::Scope & scope,
      const CppAstNode & node) = 0;
  virtual semantic_conversion::ExprInfo make_value_initialized_expr(
      const cpp_decl::TypePtr & type) = 0;
  virtual semantic_model::FunctionBinding * lookup_synthetic_lambda_binding(
      semantic_model::Scope & scope,
      const CppAstNode & node) = 0;
  virtual semantic_model::ClassInfo * lookup_synthetic_lambda_closure(
      semantic_model::Scope & scope,
      const CppAstNode & node) = 0;
  virtual void register_synthetic_lambda_binding(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      semantic_model::FunctionBinding & binding) = 0;
  virtual void register_synthetic_lambda_closure(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      semantic_model::ClassInfo & info) = 0;
  virtual semantic_model::FunctionBinding * create_synthetic_lambda_function(
      semantic_model::Scope & scope,
      const cpp_decl::TypePtr & function_type,
      const std::vector<std::pair<std::string, cpp_decl::TypePtr> > & params,
      const std::vector<const CppAstNode *> & default_arguments,
      const CppAstNode * declarator,
      const CppAstNode * body) = 0;
  virtual semantic_model::ClassInfo * synthesize_lambda_closure_class(
      semantic_model::Scope & scope,
      const std::vector<std::pair<std::string, bool> > & captures,
      const std::vector<std::pair<std::string, cpp_decl::TypePtr> > & params,
      const std::vector<const CppAstNode *> & default_arguments,
      const cpp_decl::TypePtr & result_type,
      bool mutable_lambda,
      const CppAstNode * declarator,
      const CppAstNode * body,
      semantic_model::FunctionBinding *& call_operator) = 0;
  virtual bool uses_extended_virtual_abi() const = 0;
  virtual bool can_synthesize_aggregate_constructor(const semantic_model::ClassInfo & info) const = 0;
  virtual bool class_has_bit_fields(const semantic_model::ClassInfo & info) const = 0;
  virtual bool explicit_function_nothrow(semantic_model::FunctionBinding & binding,
                                         bool & out) = 0;
  virtual bool evaluate_explicit_function_nothrow_semantically(
      semantic_model::FunctionBinding & binding,
      bool & out) = 0;
  virtual bool function_binding_is_nothrow(semantic_model::FunctionBinding & binding) = 0;
  virtual bool callsem_node_can_throw(semantic_model::Scope & scope,
                                      const CallSemNode & node,
                                      std::set<semantic_model::FunctionBinding *> & visiting) = 0;
  virtual const semantic_model::ClassIndexMap &
  template_named_class_index() const = 0;
  virtual semantic_model::ClassInfo * class_info_for_type(const cpp_decl::TypePtr & type) const = 0;
  virtual semantic_model::Scope * scope_for_type(const cpp_decl::TypePtr & type) const = 0;
  virtual semantic_model::ClassInfo * complete_class_type(const cpp_decl::TypePtr & type) = 0;
  virtual bool is_empty_class_info(const semantic_model::ClassInfo * info) const = 0;
  virtual semantic_model::FunctionBinding * select_default_constructor_for_builtin_trait(
      semantic_model::Scope & scope,
      semantic_model::ClassInfo & info) = 0;
  virtual bool try_argument_conversion(semantic_model::Scope & scope,
                                       const cpp_decl::TypePtr & target,
                                       const semantic_conversion::ExprInfo & arg,
                                       semantic_conversion::ExprInfo & out,
                                       semantic_conversion::ConversionRank & rank,
                                       const ArgumentConversionOptions & options =
                                           ArgumentConversionOptions()) = 0;

  virtual void register_builtin_function(semantic_model::Scope & scope,
                                         const std::string & name,
                                         const cpp_decl::TypePtr & result_type,
                                         const std::vector<cpp_decl::TypePtr> & params,
                                         bool explicit_nothrow = false) = 0;

  virtual std::size_t evaluate_bit_field_width(semantic_model::ClassInfo & info,
                                               const semantic_model::FieldInfo & field) = 0;
  virtual semantic_model::FunctionBinding * find_exact_class_function(
      semantic_model::ClassInfo & info,
      const std::string & name,
      const cpp_decl::TypePtr & type,
      semantic_model::RefQualifier ref_qualifier =
          static_cast<semantic_model::RefQualifier>(0)) = 0;
  virtual semantic_model::FunctionBinding * find_equivalent_class_function(
      semantic_model::ClassInfo & info,
      const std::string & name,
      const cpp_decl::TypePtr & type,
      semantic_model::RefQualifier ref_qualifier =
          static_cast<semantic_model::RefQualifier>(0)) = 0;
  virtual semantic_model::FunctionBinding * register_function_entity(
      const FunctionRegistrationRequest & request) = 0;
  virtual bool parse_decl_spec(const CppAstNode & node,
                               semantic_model::Scope & scope,
                               bool & is_typedef,
                               cpp_decl::TypePtr & out,
                               bool reference_class_templates_only = false) = 0;
  virtual bool prepare_namespace_scope_specifiers(semantic_model::Scope & scope,
                                                  const CppAstNode & specifiers,
                                                  const CppAstNode * declarators,
                                                  bool collect_embedded_types,
                                                  bool collect_named_forward_declarations,
                                                  CppAstNode & out) = 0;
  virtual bool prepare_namespace_scope_declaration_specifiers(
      semantic_model::Scope & scope,
      const CppAstNode & specifiers,
      const CppAstNode * declarators,
      bool collect_embedded_types,
      bool collect_named_forward_declarations,
      PreparedDeclarationSpecifiers & out) = 0;
  virtual bool resolve_declared_class_scope_and_name(semantic_model::Scope & scope,
                                                     const cpp_decl::QualifiedName & name,
                                                     semantic_model::Scope *& target_scope,
                                                     std::string & class_name) = 0;
  virtual semantic_model::ClassInfo * create_class_info(semantic_model::Scope & scope,
                                                        const std::string & class_kind,
                                                        const std::string & name,
                                                        const CppAstNode * class_node = nullptr) = 0;
  virtual bool class_layout_depends_on_template_parameters(
      const semantic_model::ClassInfo & info) const = 0;
  virtual void finalize_dependent_class_shape(semantic_model::ClassInfo & info) = 0;

  virtual std::vector<semantic_model::FunctionTemplateDecl *> lookup_function_templates(
      semantic_model::Scope & scope,
      const std::string & name) = 0;
  virtual std::vector<semantic_model::FunctionTemplateDecl *> lookup_function_templates_node(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      const std::string & name) = 0;
  virtual semantic_model::AliasTemplateDecl * lookup_alias_template(
      semantic_model::Scope & scope,
      const std::string & name) = 0;
  virtual semantic_model::AliasTemplateDecl * lookup_alias_template(
      semantic_model::Scope & scope,
      const cpp_decl::QualifiedName & name) = 0;
  virtual semantic_model::ClassTemplateDecl * lookup_class_template(
      semantic_model::Scope & scope,
      const std::string & name) = 0;
  virtual semantic_model::ClassTemplateDecl * lookup_class_template(
      semantic_model::Scope & scope,
      const cpp_decl::QualifiedName & name) = 0;
  virtual semantic_model::VariableTemplateDecl * lookup_variable_template(
      semantic_model::Scope & scope,
      const std::string & name) = 0;
  virtual std::vector<semantic_model::FunctionBinding *> lookup_functions(
      semantic_model::Scope & scope,
      const std::string & name,
      const semantic_overload::CallAnalysisOptions & options =
          semantic_overload::CallAnalysisOptions()) = 0;
  virtual std::vector<semantic_model::FunctionBinding *> lookup_functions_node(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      const std::string & name,
      const semantic_overload::CallAnalysisOptions & options =
          semantic_overload::CallAnalysisOptions()) = 0;
  virtual semantic_model::Scope * resolve_qualified_scope_for_node(
      semantic_model::Scope & scope,
      const cpp_decl::QualifiedName & qualified,
      const CppAstNode & node,
      bool allow_dependent_class_qualifiers) = 0;
  virtual std::vector<semantic_model::FunctionBinding *> lookup_function_template_id(
      semantic_model::Scope & scope,
      const cpp_decl::TemplateIdSyntax & template_id,
      const semantic_overload::CallAnalysisOptions & options =
          semantic_overload::CallAnalysisOptions()) = 0;
  virtual std::vector<semantic_model::FunctionBinding *> lookup_function_template_id_node(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      const cpp_decl::TemplateIdSyntax & template_id,
      const semantic_overload::CallAnalysisOptions & options =
          semantic_overload::CallAnalysisOptions()) = 0;
  virtual const semantic_model::ValueBinding * lookup_value(
      semantic_model::Scope & scope,
      const std::string & name) = 0;
  virtual std::vector<std::string> expand_bound_type_pack_texts(
      semantic_model::Scope & scope,
      const std::vector<std::string> & texts) = 0;
  virtual std::vector<std::string> expand_bound_expression_pack_texts(
      semantic_model::Scope & scope,
      const std::string & text) = 0;
  virtual bool expand_pack_argument_node(semantic_model::Scope & scope,
                                         const CppAstNode & node,
                                         std::vector<CppAstNode> & out) = 0;
  virtual bool evaluate_builtin_type_trait(semantic_model::Scope & scope,
                                           const std::string & name,
                                           const cpp_decl::TypePtr & type,
                                           long long & value) = 0;
  virtual bool evaluate_builtin_binary_type_trait(semantic_model::Scope & scope,
                                                  const std::string & name,
                                                  const cpp_decl::TypePtr & lhs,
                                                  const cpp_decl::TypePtr & rhs,
                                                  long long & value) = 0;
  virtual bool evaluate_constant_expression(semantic_model::Scope & scope,
                                            const CppAstNode & node,
                                            long long & value) = 0;
  virtual bool evaluate_constant_expression_value(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      constant_eval::ConstexprValue & value) = 0;
  virtual bool text_mentions_template_placeholders(semantic_model::Scope & scope,
                                                   const std::string & text) const = 0;
  virtual bool text_mentions_dependent_non_namespace_binding_names(
      semantic_model::Scope & scope,
      const std::string & text) const = 0;
  virtual bool type_depends_on_template_parameter(const cpp_decl::TypePtr & type) const = 0;
  virtual bool sizeof_depends_on_template_parameters(
      const cpp_decl::TypePtr & type) const = 0;
  virtual bool should_defer_unresolved_type_lookup(semantic_model::Scope & scope,
                                                   const std::string & text) const = 0;
  virtual std::string instantiation_identity_text_for_type_argument(
      const cpp_decl::TypePtr & type) const = 0;
  virtual void bind_single_template_argument_into_scope(
      semantic_model::Scope & scope,
      const template_model::TemplateParameterInfo & parameter,
      const template_model::TemplateArgument & argument) = 0;

  virtual semantic_model::Scope & append_template_scope(semantic_model::Scope & parent) = 0;
  virtual bool evaluate_initializer_constant(semantic_model::Scope & scope,
                                             const CppAstNode & initializer,
                                             long long & value) = 0;
  virtual bool evaluate_initializer_constant_value(
      semantic_model::Scope & scope,
      const CppAstNode & initializer,
      constant_eval::ConstexprValue & value) = 0;
  virtual bool evaluate_initializer_constant_value(
      semantic_model::Scope & scope,
      const CppAstNode & initializer,
      const cpp_decl::TypePtr & target,
      constant_eval::ConstexprValue & value) = 0;
  virtual std::string make_template_specialization_name(
      const std::string & name,
      const std::vector<template_model::TemplateArgument> & arguments) const = 0;
  virtual bool is_builtin_initializer_list_template(semantic_model::ClassTemplateDecl & decl) const = 0;
  virtual semantic_model::ClassInfo * create_instantiated_class_info(
      const ClassTemplateInfoCreationRequest & request) = 0;
  virtual void reset_instantiated_class_info(semantic_model::ClassInfo & info,
                                             const std::string & template_name,
                                             const CppAstNode * output_node) = 0;
  virtual void discard_class_function_bindings_for_reset(
      semantic_model::ClassInfo & info) {}
  virtual void populate_class_info(semantic_model::ClassInfo & info,
                                   const CppAstNode & node) = 0;
  virtual void finalize_class_virtuals(semantic_model::ClassInfo & info) = 0;
  virtual void finalize_class_layout(semantic_model::ClassInfo & info) = 0;
  virtual void build_function_template_parse_view(
      const semantic_model::FunctionTemplateDecl & decl,
      CppAstNode & specifiers,
      CppAstNode & declarator) = 0;
  virtual CppAstNode filter_function_declarator(const CppAstNode & declarator) const = 0;
  virtual bool parse_trailing_return_base(semantic_model::Scope & scope,
                                          const CppAstNode & specifiers,
                                          const CppAstNode & declarator,
                                          bool & is_typedef,
                                          cpp_decl::TypePtr & base,
                                          bool reference_class_templates_only) = 0;
  virtual bool parse_function_definition_base(semantic_model::Scope & scope,
                                              const CppAstNode & specifiers,
                                              const CppAstNode & declarator,
                                              const CppAstNode & body,
                                              bool is_const_method,
                                              bool is_volatile_method,
                                              bool & is_typedef,
                                              cpp_decl::TypePtr & base,
                                              bool reference_class_templates_only = false) = 0;
  virtual bool parse_declarator(semantic_model::Scope & scope,
                                const CppAstNode & declarator,
                                const cpp_decl::TypePtr & base,
                                std::string & name,
                                cpp_decl::TypePtr & type,
                                bool reference_class_templates_only = false) = 0;
  virtual bool parse_parameter_clause(semantic_model::Scope & scope,
                                      const CppAstNode & node,
                                      std::vector<std::pair<std::string, cpp_decl::TypePtr> > & params,
                                      std::vector<const CppAstNode *> * default_args_out,
                                      bool reference_class_templates_only = false,
                                      std::vector<cpp_decl::TypePtr> * parameter_object_types_out = nullptr) = 0;
  virtual semantic_model::FunctionBinding * register_block_scope_function_declaration(
      semantic_model::Scope & block_scope,
      const std::string & name,
      const cpp_decl::TypePtr & type,
      const std::vector<std::pair<std::string, cpp_decl::TypePtr> > & params,
      const std::vector<const CppAstNode *> & default_args,
      const CppAstNode & declaration_node,
      const CppAstNode & declarator_node,
      const CppAstNode & specifiers) = 0;
  virtual bool parse_variable_declaration_type(semantic_model::Scope & scope,
                                               const CppAstNode & specifiers,
                                               const CppAstNode & declarator,
                                               const CppAstNode * initializer,
                                               bool allow_function_types,
                                               std::string & name,
                                               cpp_decl::TypePtr & type,
                                               bool & is_typedef,
                                               bool allow_unnamed = false) = 0;
  virtual bool parse_auto_declaration_type_from_expr(
      semantic_model::Scope & scope,
      const CppAstNode & specifiers,
      const CppAstNode & declarator,
      const semantic_conversion::ExprInfo & expr,
      std::string & name,
      cpp_decl::TypePtr & type,
      bool reference_class_templates_only = false) = 0;
  virtual bool declarator_is_plain_identifier(const CppAstNode & node,
                                              std::string & name) const = 0;
  virtual std::string next_synthetic_local_name(const std::string & prefix) = 0;
  virtual bool prepare_block_scope_specifiers(semantic_model::Scope & scope,
                                              const CppAstNode & specifiers,
                                              const CppAstNode * declarators,
                                              CppAstNode & out) = 0;
  virtual bool prepare_block_scope_declaration_specifiers(
      semantic_model::Scope & scope,
      const CppAstNode & specifiers,
      const CppAstNode * declarators,
      PreparedDeclarationSpecifiers & out) = 0;
  virtual cpp_decl::TypePtr apply_auto_cv_qualifiers(
      const CppAstNode & specifiers,
      const cpp_decl::TypePtr & deduced) const = 0;
  virtual bool is_initializer_list_type(
      const cpp_decl::TypePtr & type,
      cpp_decl::TypePtr * element_type,
      semantic_model::ClassInfo ** info_out) const = 0;
  virtual semantic_model::Scope & append_namespace_scope(semantic_model::Scope & parent,
                                                         const std::string & name) = 0;
  virtual semantic_model::Scope * find_named_namespace_child(
      semantic_model::Scope & scope,
      const std::string & name) = 0;
  virtual std::vector<semantic_model::FunctionBinding *> lookup_qualified_functions(
      semantic_model::Scope & scope,
      const cpp_decl::QualifiedName & name) = 0;
  virtual std::vector<semantic_model::FunctionTemplateDecl *> lookup_qualified_function_templates(
      semantic_model::Scope & scope,
      const cpp_decl::QualifiedName & name) = 0;
  virtual void collect_class_declaration(semantic_model::Scope & scope,
                                         const CppAstNode & node) = 0;
  virtual void collect_enum_declaration(semantic_model::Scope & scope,
                                        const CppAstNode & node) = 0;
  virtual void collect_simple_declaration(semantic_model::Scope & scope,
                                          const CppAstNode & node,
                                          bool is_c_linkage,
                                          bool linkage_has_braces) = 0;
  virtual void collect_explicit_instantiation(semantic_model::Scope & scope,
                                              const CppAstNode & node) = 0;
  virtual const CppAstNode * own_synthetic_ast(CppAstNode node) = 0;
  virtual semantic_model::FunctionTemplateDecl * register_inherited_constructor_template(
      semantic_model::ClassInfo & owner,
      semantic_model::FunctionTemplateDecl & base_template,
      const std::string & constructor_name,
      const CppAstNode & using_node,
      const CppAstNode * ctor_initializer,
      semantic_model::MemberAccess access) = 0;
  virtual void collect_deduction_guide_declaration(semantic_model::Scope & scope,
                                                   const CppAstNode & node) = 0;
  virtual void collect_template_declaration(
      semantic_model::Scope & scope,
      const CppAstNode & node,
      semantic_model::MemberAccess access = static_cast<semantic_model::MemberAccess>(0)) = 0;
  virtual void collect_function_definition(semantic_model::Scope & scope,
                                           const CppAstNode & node,
                                           bool is_c_linkage) = 0;
  virtual void collect_special_member_definition(semantic_model::Scope & scope,
                                                 const CppAstNode & node) = 0;
  virtual std::string describe_expression_for_diagnostic(const CppAstNode & node) const = 0;
  virtual std::string describe_scope_bindings_for_diagnostic(
      const semantic_model::Scope & scope) const = 0;
  virtual std::string describe_static_assert_lookup_for_diagnostic(
      semantic_model::Scope & scope,
      const CppAstNode & node) = 0;
  virtual semantic_model::FunctionBinding * find_exact_function(
      semantic_model::Scope & scope,
      const std::string & name,
      const cpp_decl::TypePtr & type) = 0;
  virtual semantic_model::FunctionBinding * find_defined_function(
      semantic_model::Scope & scope,
      const std::string & name,
      const cpp_decl::TypePtr & type,
      const FunctionTemplateRegistrationIdentity & template_identity =
          FunctionTemplateRegistrationIdentity()) = 0;
  virtual semantic_model::FunctionBinding * find_defined_class_function(
      semantic_model::ClassInfo & info,
      const std::string & name,
      const cpp_decl::TypePtr & type,
      const FunctionTemplateRegistrationIdentity & template_identity =
          FunctionTemplateRegistrationIdentity(),
      semantic_model::RefQualifier ref_qualifier =
          static_cast<semantic_model::RefQualifier>(0)) = 0;
  virtual semantic_model::FunctionBinding * find_function_by_symbol(
      const symbol_linkage::SymbolIdentity & symbol,
      const std::string & name,
      const cpp_decl::TypePtr & type) = 0;
  virtual semantic_model::Scope * resolve_qualified_function_parse_scope(
      semantic_model::Scope & scope,
      const CppAstNode & declarator) = 0;
  virtual bool resolve_out_of_class_method_binding(
      semantic_model::Scope & scope,
      const cpp_decl::QualifiedName & name,
      const cpp_decl::TypePtr & type,
      bool is_const_method,
      bool is_volatile_method,
      semantic_model::RefQualifier ref_qualifier,
      semantic_model::FunctionBinding *& out) = 0;
  virtual bool resolve_out_of_class_method_binding_from_declarator_syntax(
      semantic_model::Scope & scope,
      const cpp_decl::QualifiedName & name,
      const CppAstNode * function_identifier,
      const cpp_decl::TypePtr & type,
      bool is_const_method,
      bool is_volatile_method,
      semantic_model::RefQualifier ref_qualifier,
      semantic_model::FunctionBinding *& out) = 0;
  virtual bool resolve_out_of_class_special_member_binding(
      semantic_model::Scope & scope,
      const cpp_decl::QualifiedName & name,
      const std::vector<std::pair<std::string, cpp_decl::TypePtr> > & params,
      const CppAstNode * function_identifier,
      semantic_model::FunctionBinding *& out) = 0;
  // Record / retrieve the binding an out-of-class member definition node
  // resolved to during collection, so the output phase can reuse it instead of
  // re-resolving the owner class from the qualified-name text.
  virtual void note_out_of_class_definition_binding(
      const CppAstNode & node,
      semantic_model::FunctionBinding * binding) = 0;
  virtual semantic_model::FunctionBinding * out_of_class_definition_binding(
      const CppAstNode & node) const = 0;
  virtual void track_instantiated_class(semantic_model::ClassInfo * info) = 0;

  virtual std::string normalize_type_lookup_name(const std::string & text) const = 0;
  virtual cpp_decl::TypePtr instantiate_alias_template(
      semantic_model::AliasTemplateDecl & decl,
      semantic_model::Scope & use_scope,
      const std::vector<std::string> & arg_texts,
      bool reference_class_templates_only) = 0;
  virtual cpp_decl::TypePtr instantiate_alias_template_with_syntax(
      semantic_model::AliasTemplateDecl & decl,
      semantic_model::Scope & use_scope,
      const std::vector<std::string> & arg_texts,
      const std::vector<cpp_decl::TemplateArgumentSyntax> * arg_syntaxes,
      bool reference_class_templates_only,
      bool suppress_source_capture = false) = 0;
  virtual cpp_decl::TypePtr instantiate_resolved_alias_template(
      semantic_model::AliasTemplateDecl & decl,
      semantic_model::Scope & use_scope,
      const std::vector<template_model::TemplateArgument> & arguments,
      bool reference_class_templates_only) = 0;
  virtual semantic_model::ClassInfo * reference_class_template_instantiation(
      semantic_model::ClassTemplateDecl & decl,
      semantic_model::Scope & use_scope,
      const std::vector<std::string> & arg_texts) = 0;
  virtual semantic_model::ClassInfo * reference_class_template_instantiation_with_syntax(
      semantic_model::ClassTemplateDecl & decl,
      semantic_model::Scope & use_scope,
      const std::vector<std::string> & arg_texts,
      const std::vector<cpp_decl::TemplateArgumentSyntax> * arg_syntaxes,
      template_api::ClassTemplateSourceUseMode source_use_mode =
          template_api::ClassTemplateSourceUseMode::EmitClassUse) = 0;
  virtual semantic_model::ClassInfo * reference_selected_class_template_instantiation(
      semantic_model::ClassTemplateDecl & decl,
      semantic_model::Scope & use_scope,
      const std::vector<template_model::TemplateArgument> & arguments,
      const template_api::ClassSpecializationSelection & specialization,
      const std::vector<std::string> * source_arg_texts,
      template_api::ClassTemplateSourceUseMode source_use_mode =
          template_api::ClassTemplateSourceUseMode::EmitClassUse,
      const std::vector<cpp_decl::TemplateArgumentSyntax> * source_arg_syntaxes = nullptr,
      const std::string * precomputed_key = nullptr) = 0;
  virtual semantic_model::ClassInfo * instantiate_selected_class_template(
      semantic_model::ClassTemplateDecl & decl,
      semantic_model::Scope & use_scope,
      const std::vector<template_model::TemplateArgument> & arguments,
      const template_api::ClassSpecializationSelection & specialization) = 0;
  virtual semantic_model::FunctionBinding * ensure_implicit_copy_constructor(
      semantic_model::ClassInfo & info) = 0;
  virtual semantic_model::FunctionBinding * ensure_implicit_move_constructor(
      semantic_model::ClassInfo & info) = 0;
  virtual semantic_model::FunctionBinding * ensure_implicit_move_assignment(
      semantic_model::ClassInfo & info) = 0;
  virtual semantic_model::FunctionBinding * ensure_implicit_copy_assignment(
      semantic_model::ClassInfo & info) = 0;
  virtual void ensure_class_reference_members(semantic_model::ClassInfo & info) = 0;
  virtual semantic_model::FunctionBinding * select_constructor_from_exprs(
      semantic_model::Scope & scope,
      semantic_model::ClassInfo & info,
      const std::vector<semantic_conversion::ExprInfo> & source_args,
      std::vector<semantic_conversion::ExprInfo> & args_out,
      std::vector<semantic_conversion::ConversionRank> * ranks_out,
      const ConstructorSelectionOptions & options) = 0;
  virtual semantic_model::FunctionBinding * select_constructor(
      semantic_model::Scope & scope,
      semantic_model::ClassInfo & info,
      const std::vector<const CppAstNode *> & arg_nodes,
      std::vector<semantic_conversion::ExprInfo> & args_out,
      const ConstructorSelectionOptions & options) = 0;
  virtual semantic_model::FunctionBinding * select_constructor_for_direct_braced_init(
      semantic_model::Scope & scope,
      semantic_model::ClassInfo & info,
      const CppAstNode & direct_braced_init,
      std::vector<semantic_conversion::ExprInfo> & args_out,
      const ConstructorSelectionOptions & options) = 0;
  virtual semantic_conversion::ExprInfo make_constructor_conversion_expr(
      semantic_model::FunctionBinding & ctor,
      const cpp_decl::TypePtr & result_type,
      const std::vector<semantic_conversion::ExprInfo> & args,
      bool mark_output_required = true) = 0;
  virtual bool is_conversion_function_name(const std::string & name) const = 0;
  virtual semantic_conversion::ExprInfo make_address_of_expr(
      const semantic_conversion::ExprInfo & expr) const = 0;
  virtual semantic_conversion::ExprInfo make_field_expr(
      const semantic_conversion::ExprInfo & base,
      const semantic_model::FieldInfo & field) = 0;
  virtual semantic_conversion::ExprInfo make_subscript_expr(
      const semantic_conversion::ExprInfo & base,
      std::size_t index,
      const cpp_decl::TypePtr & element_type) = 0;
  virtual semantic_conversion::ExprInfo make_base_expr(
      const semantic_conversion::ExprInfo & base,
      const semantic_model::ClassInfo & class_info,
      std::size_t offset) const = 0;
  virtual semantic_conversion::ExprInfo make_base_pointer_expr(
      const semantic_conversion::ExprInfo & base_ptr,
      const semantic_model::ClassInfo & class_info,
      std::size_t offset) const = 0;
  virtual semantic_conversion::ExprInfo make_direct_call_expr(
      semantic_model::FunctionBinding & binding,
      const std::vector<semantic_conversion::ExprInfo> & args,
      bool mark_output_required = true) = 0;
  virtual semantic_model::FunctionBinding * first_function_by_internal_symbol(
      const std::string & internal_symbol) const = 0;
  virtual semantic_model::FunctionBinding * first_function_by_object_symbol(
      const std::string & object_symbol) const = 0;
  virtual semantic_conversion::ExprInfo apply_base_subobject_adjustment(
      const semantic_conversion::ExprInfo & expr,
      const cpp_decl::TypePtr & adjusted_type,
      const semantic_model::ClassInfo & target_class,
      std::size_t offset) const = 0;
  virtual void set_expr_info_metadata(semantic_conversion::ExprInfo & expr,
                                      const cpp_decl::TypePtr & type,
                                      semantic_conversion::ValueCategory category) const = 0;

  virtual bool lookup_constant_value(semantic_model::Scope & scope,
                                     const std::string & name,
                                     constant_eval::ConstexprValue & value) = 0;
  virtual bool lookup_constant_value_node(semantic_model::Scope & scope,
                                          const std::string & name,
                                          const CppAstNode * node,
                                          constant_eval::ConstexprValue & value) = 0;
  virtual bool lookup_constant_template_id_value(
      semantic_model::Scope & scope,
      const cpp_decl::TemplateIdSyntax & template_id,
      const std::string & display_name,
      constant_eval::ConstexprValue & value) = 0;
  virtual bool lookup_constant_template_member_value(
      semantic_model::Scope & scope,
      const cpp_decl::TemplateIdSyntax & qualifier_template_id,
      const std::string & member_name,
      const std::string & display_name,
      constant_eval::ConstexprValue & value) = 0;
  virtual bool evaluate_sizeof_operand_for_consteval(semantic_model::Scope & scope,
                                                     const CppAstNode & expr,
                                                     std::size_t & size) = 0;
  virtual bool lookup_pack_size(semantic_model::Scope & scope,
                                const std::string & name,
                                std::size_t & pack_size) = 0;
  virtual bool try_parse_builtin_type_trait_call(semantic_model::Scope & scope,
                                                 const CppAstNode & expr,
                                                 std::string & builtin_name,
                                                 std::vector<cpp_decl::TypePtr> & builtin_types) = 0;
  virtual bool expression_is_nothrow(semantic_model::Scope & scope,
                                     const CppAstNode & expr,
                                     bool & out) = 0;
  virtual void note_rtti_use(const cpp_decl::TypePtr & type,
                             bool dynamic_use) = 0;
  virtual void note_dynamic_typeid_use(const cpp_decl::TypePtr & type) = 0;
  virtual void append_rtti_candidates(const cpp_decl::TypePtr & static_type,
                                      CallSemNode & node,
                                      semantic_model::ClassInfo * required_target = nullptr) = 0;
  virtual void append_exception_candidates(const cpp_decl::TypePtr & catch_type,
                                           CallSemNode & node) = 0;
  virtual bool has_dynamic_cast_candidate(semantic_model::ClassInfo * source_class,
                                          semantic_model::ClassInfo * target_class) = 0;
  virtual bool try_parse_builtin_type_trait_call_arg(semantic_model::Scope & scope,
                                                     const CppAstNode & arg,
                                                     cpp_decl::TypePtr & type) = 0;
  virtual bool lookup_offsetof_field(const cpp_decl::TypePtr & object_type,
                                     const std::string & member_name,
                                     semantic_consteval::OffsetofFieldInfo & out) = 0;
  virtual bool evaluate_constant_call_expression_value(
      semantic_model::Scope & scope,
      constant_eval::Evaluator & evaluator,
      const CppAstNode & node,
      const std::vector<constant_eval::ConstexprValue> & args,
      constant_eval::ConstexprValue & out) = 0;
  virtual bool parse_constexpr_local_declaration(
      semantic_model::Scope & scope,
      const CppAstNode & decl,
      std::vector<constant_eval::LocalDeclaration> & locals,
      std::string & error) = 0;
  virtual bool parse_decltype_specifier(semantic_model::Scope & scope,
                                        const CppAstNode & node,
                                        cpp_decl::TypePtr & out) = 0;
};
