#pragma once

#include <map>
#include <string>
#include <vector>

#include "cppast_ast.h"
#include "semantic_model.h"
#include "template_environment.h"
#include "template_model.h"

namespace template_api {

enum class TemplateInstantiationIntent
{
  LookupOnly,
  TrackInstantiation,
  RequireDefinition,
  RequireDefinitionAndExport
};

enum class TemplateFunctionBindingAcquisitionCause
{
  None,
  RequireDefinition
};

struct TemplateFunctionInstantiationRequest
{
  semantic_model::FunctionTemplateDecl * decl = nullptr;
  std::vector<template_model::TemplateArgument> arguments;
  semantic_model::ClassInfo * active_owner = nullptr;
  const CppAstNode * body_override = nullptr;
  const CppAstNode * definition_node_override = nullptr;
  bool explicit_specialization = false;
  bool explicit_specialization_is_constexpr = false;
  bool include_body = true;
  TemplateEnvironmentHandle use_scope;
  std::map<std::string, std::size_t> pack_sizes;
  bool has_pack_sizes = false;
  bool prefer_overload_suffix = false;
  std::string instantiation_use_location;
  TemplateInstantiationIntent intent = TemplateInstantiationIntent::LookupOnly;
};

struct TemplateClassInstantiationRequest
{
  semantic_model::ClassTemplateDecl * decl = nullptr;
  TemplateEnvironmentHandle use_scope;
  std::vector<template_model::TemplateArgument> arguments;
};

struct TemplateClassFinalizationRequest
{
  semantic_model::ClassTemplateDecl * decl = nullptr;
  semantic_model::ClassInfo * info = nullptr;
  std::vector<template_model::TemplateArgument> arguments;
};

struct TemplateVariableInstantiationRequest
{
  semantic_model::VariableTemplateDecl * decl = nullptr;
  std::vector<template_model::TemplateArgument> arguments;
  std::string source_use_location;
  semantic_model::Scope * source_use_scope = nullptr;
  TemplateInstantiationIntent intent = TemplateInstantiationIntent::LookupOnly;
};

struct TemplateFunctionBindingAcquisitionRequest
{
  semantic_model::FunctionBinding * binding = nullptr;
  bool include_body = true;
  TemplateEnvironmentHandle use_scope;
  TemplateInstantiationIntent intent = TemplateInstantiationIntent::LookupOnly;
  TemplateFunctionBindingAcquisitionCause cause =
      TemplateFunctionBindingAcquisitionCause::None;
};

struct TemplateInstantiationResult
{
  TemplateInstantiationIntent intent = TemplateInstantiationIntent::LookupOnly;
  semantic_model::FunctionBinding * function_binding = nullptr;
  semantic_model::ClassInfo * class_info = nullptr;
  const semantic_model::ValueBinding * value_binding = nullptr;
  bool created_new_binding = false;
  bool created_new_class = false;
  bool created_new_value = false;
  bool definition_materialized = false;
  bool class_finalized = false;
  bool output_tracked = false;
  bool definition_required = false;
};

}  // namespace template_api
