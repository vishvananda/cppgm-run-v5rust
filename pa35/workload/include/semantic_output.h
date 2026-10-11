#pragma once

#include <map>
#include <memory>
#include <set>
#include <vector>

#include "callsem_output.h"
#include "cpp_decl_model.h"
#include "semantic_model.h"

class SemanticContext;
struct CppAstNode;

namespace semantic_output {

struct OutputState
{
  OutputState(std::vector<std::unique_ptr<semantic_model::ClassInfo> > & classes,
              std::vector<semantic_model::FunctionBinding *> & required_function_definitions,
              std::vector<semantic_model::FunctionBinding *> & late_required_class_methods,
              std::vector<semantic_model::FunctionBinding *> &
                  late_required_class_static_functions,
              std::vector<semantic_model::ClassInfo *> & instantiated_classes,
              std::vector<semantic_model::FunctionBinding *> & instantiated_functions,
              std::vector<semantic_model::FunctionBinding *> & synthetic_functions,
              std::vector<semantic_model::FunctionBinding *> & deferred_constexpr_functions,
              std::vector<semantic_model::ClassInfo *> & rtti_required_classes,
              std::set<std::string> & emitted_rtti_symbols,
              std::map<std::string, cpp_decl::TypePtr> & emitted_rtti_types,
              std::set<std::string> & emitted_rtti_output,
              std::set<std::string> & emitted_vtable_output)
    : classes(classes),
      required_function_definitions(required_function_definitions),
      late_required_class_methods(late_required_class_methods),
      late_required_class_static_functions(late_required_class_static_functions),
      instantiated_classes(instantiated_classes),
      instantiated_functions(instantiated_functions),
      synthetic_functions(synthetic_functions),
      deferred_constexpr_functions(deferred_constexpr_functions),
      rtti_required_classes(rtti_required_classes),
      emitted_rtti_symbols(emitted_rtti_symbols),
      emitted_rtti_types(emitted_rtti_types),
      emitted_rtti_output(emitted_rtti_output),
      emitted_vtable_output(emitted_vtable_output)
  {}

  std::vector<std::unique_ptr<semantic_model::ClassInfo> > & classes;
  std::vector<semantic_model::FunctionBinding *> & required_function_definitions;
  std::vector<semantic_model::FunctionBinding *> & late_required_class_methods;
  std::vector<semantic_model::FunctionBinding *> & late_required_class_static_functions;
  std::vector<semantic_model::ClassInfo *> & instantiated_classes;
  std::vector<semantic_model::FunctionBinding *> & instantiated_functions;
  std::vector<semantic_model::FunctionBinding *> & synthetic_functions;
  std::vector<semantic_model::FunctionBinding *> & deferred_constexpr_functions;
  std::vector<semantic_model::ClassInfo *> & rtti_required_classes;
  std::set<std::string> & emitted_rtti_symbols;
  std::map<std::string, cpp_decl::TypePtr> & emitted_rtti_types;
  std::set<std::string> & emitted_rtti_output;
  std::set<std::string> & emitted_vtable_output;
  std::size_t required_function_definition_refresh_index = 0;
  std::size_t deferred_constexpr_function_output_index = 0;
  std::size_t late_required_function_output_index = 0;
  std::size_t instantiated_class_output_index = 0;
  std::size_t instantiated_function_output_index = 0;
  std::size_t late_synthesized_class_scan_index = 0;
  std::size_t late_required_class_method_output_index = 0;
  std::size_t late_required_class_static_function_output_index = 0;
  std::size_t synthetic_function_output_index = 0;
  std::size_t emitted_output_callee_scan_index = 0;
  std::vector<semantic_model::ClassInfo *> pending_instantiated_class_output;
  std::vector<semantic_model::FunctionBinding *> pending_instantiated_function_output;
  std::vector<semantic_model::FunctionBinding *> pending_late_required_functions;
  std::vector<semantic_model::FunctionBinding *> pending_late_required_class_methods;
  std::vector<semantic_model::FunctionBinding *>
      pending_late_required_class_static_functions;
};

void analyze_function_binding_output(SemanticContext & ctx,
                                     OutputState & state,
                                     semantic_model::Scope & scope,
                                     semantic_model::FunctionBinding & binding,
                                     CallSemNode & out);

void analyze_function_body_for_witness_semantics(SemanticContext & ctx,
                                                 semantic_model::Scope & scope,
                                                 semantic_model::FunctionBinding & binding);

void validate_function_body_and_cache_output(SemanticContext & ctx,
                                             semantic_model::Scope & scope,
                                             semantic_model::FunctionBinding & binding);

bool validate_function_body_for_semantic_use(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::FunctionBinding & binding);

void analyze_class_output_from_info(SemanticContext & ctx,
                                    OutputState & state,
                                    semantic_model::ClassInfo & info,
                                    const CppAstNode & node,
                                    CallSemNode & out);

void analyze_declaration_output(SemanticContext & ctx,
                                OutputState & state,
                                semantic_model::Scope & scope,
                                const CppAstNode & node,
                                CallSemNode & out,
                                bool is_c_linkage,
                                bool linkage_has_braces = false);

void analyze_instantiated_template_output(SemanticContext & ctx,
                                          OutputState & state,
                                          CallSemNode & out);

void analyze_synthetic_function_output(SemanticContext & ctx,
                                       OutputState & state,
                                       CallSemNode & out);

semantic_model::FunctionBinding * resolve_output_function_binding(
    SemanticContext & ctx,
    semantic_model::FunctionBinding * binding);

std::string explain_function_output_state(
    SemanticContext & ctx,
    semantic_model::FunctionBinding & binding);

void analyze_late_required_function_output(SemanticContext & ctx,
                                           OutputState & state,
                                           CallSemNode & out);

void analyze_late_required_synthesized_output(SemanticContext & ctx,
                                              OutputState & state,
                                              CallSemNode & out);

void expand_required_function_definition_closure(SemanticContext & ctx,
                                                 OutputState & state);

void expand_emitted_output_callee_closure(SemanticContext & ctx,
                                          OutputState & state,
                                          const CallSemNode & out);

void validate_required_function_definition_closure(SemanticContext & ctx,
                                                   OutputState & state);

void complete_output_layout_types(SemanticContext & ctx,
                                  CallSemNode & out);

}  // namespace semantic_output
