#pragma once

#include <cstddef>
#include <map>
#include <set>
#include <string>
#include <vector>

#include "cpp_decl_model.h"
#include "semantic_conversion.h"
#include "semantic_model.h"
#include "template_model.h"
#include "witness_api.h"

class SemanticContext;

namespace semantic_template_function {

struct FunctionTemplateDeduction
{
  std::vector<template_model::TemplateArgument> arguments;
  std::map<std::string, std::size_t> pack_sizes;
};

struct FunctionTemplateCallSourceUseRequest
{
  semantic_model::FunctionBinding * binding = nullptr;
  std::string use_location;
  std::string template_name;
  std::string selected;
  witness::SourceSelectionKind selection = witness::SourceSelectionKind::None;
  std::string selected_decl_location;
  witness::TemplateWitnessSourceAnchor selected_decl_anchor;
  std::vector<witness::TemplateWitnessSourceBinding> bindings;
  std::vector<witness::TemplateWitnessSourceDrop> drops;
  std::size_t explicit_arg_count = 0;
  int candidate_count = -1;
  int candidates_built = -1;
  int candidates_viable = -1;
};

bool deduce_function_template_from_arguments(
    SemanticContext & ctx,
    semantic_model::FunctionTemplateDecl & decl,
    const std::vector<semantic_conversion::ExprInfo> & args,
    semantic_model::Scope * use_scope,
    FunctionTemplateDeduction & out,
    semantic_model::Scope * resolution_scope = nullptr,
    const std::vector<template_model::TemplateArgument> * explicit_arguments = nullptr);

bool deduce_function_template_from_target_type(SemanticContext & ctx,
                                               semantic_model::FunctionTemplateDecl & decl,
                                               const cpp_decl::TypePtr & target_type,
                                               semantic_model::Scope * use_scope,
                                               FunctionTemplateDeduction & out,
                                               semantic_model::Scope * resolution_scope = nullptr,
                                               const std::vector<template_model::TemplateArgument> *
                                                   explicit_arguments = nullptr);

bool resolve_explicit_function_template_arguments(
    SemanticContext & ctx,
    semantic_model::FunctionTemplateDecl & decl,
    semantic_model::Scope & use_scope,
    const std::vector<std::string> & arg_texts,
    std::vector<template_model::TemplateArgument> & out,
    bool require_fully_bound = true);

bool resolve_call_explicit_function_template_arguments(
    SemanticContext & ctx,
    semantic_model::FunctionTemplateDecl & decl,
    semantic_model::Scope & resolution_scope,
    const std::vector<std::string> & explicit_arg_texts,
    std::vector<template_model::TemplateArgument> & out,
    const std::vector<cpp_decl::TemplateArgumentSyntax> * explicit_arg_syntaxes = nullptr);

bool transformed_function_template_parameter_types(
    semantic_model::FunctionTemplateDecl & decl,
    std::vector<cpp_decl::TypePtr> & out);

bool function_template_accepts_transformed_parameter_types(
    SemanticContext & ctx,
    semantic_model::FunctionTemplateDecl & decl,
    const std::vector<cpp_decl::TypePtr> & actual_params,
    semantic_model::Scope * actual_lookup_scope = nullptr,
    bool allow_prefix = false);

void overlay_instantiation_use_scope_bindings(
    semantic_model::Scope & target,
    const semantic_model::Scope & source,
    const semantic_model::Scope * excluded_declaring_scope = nullptr);

void overlay_instantiation_local_named_types(
    SemanticContext & ctx,
    semantic_model::Scope & target,
    semantic_model::Scope & source,
    const semantic_model::Scope * declaring_scope,
    const std::vector<template_model::TemplateArgument> & local_type_arguments,
    const std::set<std::string> * excluded_parameter_names = nullptr);

semantic_model::FunctionBinding * acquire_function_template_binding(
    SemanticContext & ctx,
    semantic_model::FunctionTemplateDecl & decl,
    const std::vector<template_model::TemplateArgument> & arguments,
    semantic_model::Scope * use_scope = nullptr,
    const std::map<std::string, std::size_t> * pack_sizes = nullptr,
    bool include_body = true,
    semantic_model::ClassInfo * active_owner = nullptr,
    const std::string & instantiation_use_location = std::string());

semantic_model::FunctionBinding * acquire_function_definition_binding(
    SemanticContext & ctx,
    semantic_model::FunctionBinding * binding,
    semantic_model::Scope & use_scope);

semantic_model::FunctionBinding * acquire_required_function_definition_binding(
    SemanticContext & ctx,
    semantic_model::FunctionBinding * binding,
    semantic_model::Scope & use_scope);

void note_required_function_definition_materialized_by_lifecycle(
    SemanticContext & ctx,
    semantic_model::FunctionBinding * binding);

void note_ensured_function_definition_materialized_by_lifecycle(
    SemanticContext & ctx,
    semantic_model::FunctionBinding * binding);

void emit_function_template_call_source_use(
    SemanticContext & ctx,
    const FunctionTemplateCallSourceUseRequest & request);

}  // namespace semantic_template_function
