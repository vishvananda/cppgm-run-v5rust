#pragma once

#include <map>
#include <string>
#include <utility>
#include <vector>

#include "cpp_decl_model.h"
#include "cppast_ast.h"
#include "semantic_model.h"
#include "template_environment.h"
#include "template_model.h"
#include "template_witness.h"

namespace constant_eval {
struct ConstexprValue;
}  // namespace constant_eval

class SemanticContext;

namespace semantic_metrics {
struct AnalyzerCounters;
}  // namespace semantic_metrics

namespace template_api {

enum class ClassTemplateSourceUseMode
{
  EmitClassUse,
  EmitClassUseOnly,
  NestedArgumentsOnly,
  SemanticLookupOnly
};

inline bool class_template_source_use_emits_class_use(
    ClassTemplateSourceUseMode mode)
{
  return mode == ClassTemplateSourceUseMode::EmitClassUse ||
         mode == ClassTemplateSourceUseMode::EmitClassUseOnly;
}

inline bool class_template_source_use_recovers_nested_arguments(
    ClassTemplateSourceUseMode mode)
{
  return mode == ClassTemplateSourceUseMode::NestedArgumentsOnly;
}

inline bool class_template_source_use_suppresses_nested_arguments(
    ClassTemplateSourceUseMode mode)
{
  return mode == ClassTemplateSourceUseMode::EmitClassUseOnly ||
         mode == ClassTemplateSourceUseMode::SemanticLookupOnly;
}

inline bool class_template_source_use_is_semantic_lookup_only(
    ClassTemplateSourceUseMode mode)
{
  return mode == ClassTemplateSourceUseMode::SemanticLookupOnly;
}

struct TemplateNamedTypeMetadata
{
  std::string name;
  std::string class_kind;
  cpp_decl::TypePtr type;
  cpp_decl::TypePtr enclosing_class_type;
  semantic_model::Scope * member_scope = nullptr;
  bool complete = false;
  bool has_enclosing_function = false;
  bool is_final = false;
  bool is_lambda_closure = false;
  bool dependent_instantiation = false;
  bool is_polymorphic = false;
  bool is_abstract = false;
  bool has_virtual_destructor = false;
  bool has_virtual_bases = false;
  bool has_default_member_initializers = false;
  bool has_user_declared_destructor = false;
  bool has_user_declared_constructor = false;
  bool has_user_declared_constructor_template = false;
  bool has_default_constructor = false;
  bool has_structural_default_constructor = false;
  semantic_model::ClassTemplateDecl * source_template = nullptr;
  std::vector<cpp_decl::TypePtr> direct_base_types;
  std::vector<cpp_decl::TypePtr> field_types;
  std::vector<std::string> instantiation_arg_texts;
  std::vector<template_model::TemplateArgument> instantiation_arguments;
};

struct TemplateSemanticModelView
{
  const semantic_model::ClassIndexMap * classes_by_key = nullptr;
};

inline semantic_model::ClassInfo * find_named_type_class_info(
    const TemplateSemanticModelView & model,
    const cpp_decl::TypePtr & type)
{
  cpp_decl::TypePtr base = cpp_decl::strip_top_level_cv(type);
  if(!base || base->kind != cpp_decl::Type::TK_NAMED || !model.classes_by_key) {
    return nullptr;
  }
  if(base->definitely_not_class) {
    return nullptr;
  }

  auto found = model.classes_by_key->find(base->named_key);
  return found != model.classes_by_key->end() ? found->second : nullptr;
}

inline bool describe_named_type_metadata(const TemplateSemanticModelView & model,
                                         const cpp_decl::TypePtr & type,
                                         TemplateNamedTypeMetadata & out)
{
  out = TemplateNamedTypeMetadata();
  semantic_model::ClassInfo * info = find_named_type_class_info(model, type);
  if(!info) {
    return false;
  }

  out.name = info->name;
  out.class_kind = info->class_kind;
  out.type = info->type;
  if(info->enclosing_scope && info->enclosing_scope->class_info) {
    out.enclosing_class_type = info->enclosing_scope->class_info->type;
  }
  out.member_scope = info->member_scope.get();
  out.complete = info->complete;
  out.has_enclosing_function =
      info->enclosing_scope && info->enclosing_scope->function != nullptr;
  out.is_final = info->is_final;
  out.is_lambda_closure = info->is_lambda_closure;
  out.dependent_instantiation = info->dependent_instantiation;
  out.is_polymorphic = info->is_polymorphic;
  for(std::size_t i = 0; i < info->vtable_entries.size(); ++i) {
    semantic_model::FunctionBinding * binding = info->vtable_entries[i];
    if(binding && binding->is_pure_virtual) {
      out.is_abstract = true;
      break;
    }
  }
  out.source_template = info->source_template;
  out.direct_base_types.reserve(info->bases.size());
  for(std::size_t i = 0; i < info->bases.size(); ++i) {
    if(info->bases[i].type) {
      out.direct_base_types.push_back(info->bases[i].type->type);
    }
    if(info->bases[i].is_virtual) {
      out.has_virtual_bases = true;
    }
  }
  out.field_types.reserve(info->fields.size());
  for(std::size_t i = 0; i < info->fields.size(); ++i) {
    if(info->fields[i].type) {
      out.field_types.push_back(info->fields[i].type);
    }
    if(info->fields[i].default_initializer) {
      out.has_default_member_initializers = true;
    }
  }
  for(std::map<std::string, std::vector<semantic_model::FunctionBinding *> >::const_iterator
          it = info->methods.begin();
      it != info->methods.end();
      ++it) {
    for(std::size_t i = 0; i < it->second.size(); ++i) {
      semantic_model::FunctionBinding * binding = it->second[i];
      if(binding && binding->is_constructor && !binding->synthesized) {
        out.has_user_declared_constructor = true;
      }
      if(binding && binding->is_constructor &&
         binding->params.size() == 1 &&
         !binding->is_deleted) {
        out.has_default_constructor = true;
        const bool empty_body =
            binding->body &&
            binding->body->kind == CppAstKind::compound_statement &&
            binding->body->children.empty();
        if(binding->is_defaulted ||
           binding->synthesized ||
           binding->is_aggregate_constructor ||
           (binding->is_constructor &&
            !binding->ctor_initializer &&
            empty_body)) {
          out.has_structural_default_constructor = true;
        }
      }
      if(binding && binding->is_destructor && !binding->synthesized) {
        out.has_user_declared_destructor = true;
      }
      if(binding && binding->is_destructor && binding->has_virtual_slot) {
        out.has_virtual_destructor = true;
      }
    }
    if(out.has_user_declared_destructor &&
       out.has_virtual_destructor &&
       out.is_abstract) {
      break;
    }
  }
  if(info->member_scope) {
    for(std::map<std::string, std::vector<semantic_model::FunctionTemplateDecl *> >::const_iterator
            it = info->member_scope->function_templates.begin();
        it != info->member_scope->function_templates.end();
        ++it) {
      for(std::size_t i = 0; i < it->second.size(); ++i) {
        if(it->second[i] && it->second[i]->is_constructor) {
          out.has_user_declared_constructor_template = true;
          break;
        }
      }
      if(out.has_user_declared_constructor_template) {
        break;
      }
    }
  }
  out.instantiation_arg_texts = info->instantiation_arg_texts;
  out.instantiation_arguments = info->instantiation_arguments;
  return true;
}

enum TemplateSemanticBuiltinTypeTrait
{
  TSBTT_IS_CONSTRUCTIBLE,
  TSBTT_IS_NOTHROW_CONSTRUCTIBLE,
  TSBTT_IS_ASSIGNABLE,
  TSBTT_IS_NOTHROW_ASSIGNABLE,
  TSBTT_IS_TRIVIALLY_ASSIGNABLE,
  TSBTT_IS_CONVERTIBLE,
  TSBTT_IS_NOTHROW_CONVERTIBLE
};

enum TemplateElaboratedTypeKind
{
  TETK_NONE,
  TETK_CLASS,
  TETK_STRUCT,
  TETK_UNION,
  TETK_ENUM,
  TETK_ENUM_CLASS,
  TETK_ENUM_STRUCT
};

struct TemplateTypeLookupRequest
{
  semantic_model::Scope * scope = nullptr;
  cpp_decl::QualifiedName name;
  TemplateElaboratedTypeKind elaborated_kind = TETK_NONE;
  bool allow_class_templates = false;
  bool top_const = false;
  bool top_volatile = false;
  ClassTemplateSourceUseMode source_use_mode =
      ClassTemplateSourceUseMode::EmitClassUse;
  std::string source_location;
};

struct TemplateSelectedClassTemplateIdRequest
{
  TemplateTypeLookupRequest lookup;
  semantic_model::Scope * argument_scope = nullptr;
  semantic_model::ClassTemplateDecl * class_template = nullptr;
  std::vector<template_model::TemplateArgument> resolved_arguments;
  std::vector<std::string> source_arg_texts;
  std::vector<cpp_decl::TemplateArgumentSyntax> source_arg_syntaxes;
};

enum TemplateDependentTypeExprKind
{
  TDTEK_DECLTYPE,
  TDTEK_TYPEOF_EXPR
};

struct TemplateDependentTypeExprRequest
{
  semantic_model::Scope * scope = nullptr;
  TemplateDependentTypeExprKind kind = TDTEK_DECLTYPE;
  bool operand_was_parenthesized = false;
  std::string use_location;
  CppAstNode operand;
};

struct TemplateSemanticBuiltinTraitRequest
{
  semantic_model::Scope * scope = nullptr;
  TemplateSemanticBuiltinTypeTrait trait = TSBTT_IS_CONSTRUCTIBLE;
  std::vector<cpp_decl::TypePtr> types;
};

struct TemplateConstantEvaluationRequest
{
  semantic_model::Scope * scope = nullptr;
  CppAstNode expr;
  cpp_decl::TypePtr target_type;
};

struct TemplateTypeSystem
{
  TemplateSemanticModelView model;

  virtual ~TemplateTypeSystem() {}
  virtual bool prepare_named_type_member_scope(
      TemplateEnvironmentHandle scope,
      const cpp_decl::TypePtr & type,
      semantic_model::Scope *& out) = 0;
  virtual bool complete_named_type_member_scope(
      TemplateEnvironmentHandle scope,
      const cpp_decl::TypePtr & type,
      semantic_model::Scope *& out) = 0;
  virtual bool resolve_member_type_lookup(semantic_model::Scope & lexical_scope,
                                          semantic_model::ClassInfo & owner,
                                          const std::string & name,
                                          bool ensure_current_reference_members,
                                          cpp_decl::TypePtr & out) = 0;
  virtual bool resolve_direct_type_lookup(const TemplateTypeLookupRequest & request,
                                          cpp_decl::TypePtr & out) = 0;
  virtual bool resolve_selected_class_template_id(
      const TemplateSelectedClassTemplateIdRequest & request,
      cpp_decl::TypePtr & out) = 0;
};

// This gateway contains the remaining semantic services that may still
// recurse into broader semantic/template machinery. Keep its surface small,
// structured, and free of witness side effects.
struct TemplateRecursiveSemanticGateway
{
  virtual ~TemplateRecursiveSemanticGateway() {}
  virtual bool evaluate_dependent_type_expression(
      const TemplateDependentTypeExprRequest & request,
      cpp_decl::TypePtr & out) = 0;
  virtual bool evaluate_semantic_builtin_type_trait(
      const TemplateSemanticBuiltinTraitRequest & request,
      long long & out) = 0;
  virtual bool evaluate_initializer_constant_value(const TemplateConstantEvaluationRequest & request,
                                                   constant_eval::ConstexprValue & out) = 0;
};

struct TemplateServices
{
  TemplateServices(TemplateTypeSystem & type_system_in,
                   TemplateRecursiveSemanticGateway & recursive_semantic_in,
                   const TemplateWitnessContext & witness_context_in,
                   SemanticContext * semantic_context_in = nullptr,
                   semantic_metrics::AnalyzerCounters * counters_in = nullptr)
    : type_system(type_system_in),
      recursive_semantic(recursive_semantic_in),
      witness_context(witness_context_in),
      semantic_context(semantic_context_in),
      counters(counters_in)
  {}

  TemplateTypeSystem & type_system;
  TemplateRecursiveSemanticGateway & recursive_semantic;
  TemplateWitnessContext witness_context;
  SemanticContext * semantic_context;
  semantic_metrics::AnalyzerCounters * counters = nullptr;
};

}  // namespace template_api
