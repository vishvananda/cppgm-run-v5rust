#pragma once

#include <string>
#include <vector>

#include "cpp_decl_model.h"
#include "semantic_model.h"
#include "template_model.h"
#include "witness_api.h"

class SemanticContext;

namespace semantic_template_class {

void emit_instantiated_class_template_use_source(
    SemanticContext & ctx,
    const semantic_model::ClassInfo & info,
    const std::string & use_location,
    witness::SourceUseRole role = witness::SourceUseRole::TypeUse);

void append_base_clause_template_value_dependencies(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & base_name,
    const cpp_decl::TemplateIdSyntax * base_template_syntax,
    const std::vector<cpp_decl::TemplateArgumentSyntax> * base_arg_syntaxes,
    std::vector<template_model::TemplateValueDependency> & out);

bool note_constant_value_member_instantiations_in_expression(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & expr);

void emit_constructor_initializer_template_id_source_use(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    const CppAstNode & initializer_id,
    const cpp_decl::TemplateIdSyntax & syntax,
    const std::string & use_location,
    const std::vector<std::string> & argument_locations);

}  // namespace semantic_template_class
