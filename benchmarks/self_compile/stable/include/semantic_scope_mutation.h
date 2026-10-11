#pragma once

#include <string>
#include <vector>

#include "cpp_decl_model.h"
#include "semantic_model.h"

namespace semantic_scope_mutation {

void note_binding_mutation(semantic_model::Scope & scope);

void bind_named_type(semantic_model::Scope & scope,
                     const std::string & name,
                     const cpp_decl::TypePtr & type);

void bind_named_type_with_access(semantic_model::Scope & scope,
                                 const std::string & name,
                                 const cpp_decl::TypePtr & type,
                                 semantic_model::MemberAccess access);

void bind_template_named_type(semantic_model::Scope & scope,
                              const std::string & name,
                              const cpp_decl::TypePtr & type);

void ensure_template_named_type(semantic_model::Scope & scope,
                                const std::string & name,
                                const cpp_decl::TypePtr & type);

void bind_template_named_type_with_access(semantic_model::Scope & scope,
                                          const std::string & name,
                                          const cpp_decl::TypePtr & type,
                                          semantic_model::MemberAccess access);

void bind_namespace(semantic_model::Scope & scope,
                    const std::string & name,
                    semantic_model::Scope * target);

void add_using_directive_if_needed(semantic_model::Scope & scope,
                                   semantic_model::Scope & target);

void import_inline_namespace_members(semantic_model::Scope & scope,
                                     semantic_model::Scope & target);

void bind_value(semantic_model::Scope & scope,
                const std::string & name,
                const semantic_model::ValueBinding & binding);

void bind_values(semantic_model::Scope & scope,
                 const std::vector<semantic_model::ValueBinding> & bindings);

void bind_value_aliases(semantic_model::Scope & scope,
                        const std::string & primary_name,
                        const std::string & alias_name,
                        const semantic_model::ValueBinding & binding);

void bind_named_pack_size(semantic_model::Scope & scope,
                          const std::string & name,
                          std::size_t size);

void bind_value_pack(semantic_model::Scope & scope,
                     const std::string & name,
                     const std::vector<semantic_model::ValueBinding> & bindings);

void bind_class_template(semantic_model::Scope & scope,
                         const std::string & name,
                         semantic_model::ClassTemplateDecl * decl);

void bind_alias_template(semantic_model::Scope & scope,
                         const std::string & name,
                         semantic_model::AliasTemplateDecl * decl);

void bind_variable_template(semantic_model::Scope & scope,
                            const std::string & name,
                            semantic_model::VariableTemplateDecl * decl);

void bind_template_template_parameter(semantic_model::Scope & scope,
                                      const std::string & name,
                                      semantic_model::ClassTemplateDecl * decl);

void bind_dependent_template_value(semantic_model::Scope & scope,
                                   const std::string & name,
                                   const cpp_decl::TypePtr & type);

void append_function_bindings(semantic_model::Scope & scope,
                              const std::string & name,
                              const std::vector<semantic_model::FunctionBinding *> & functions,
                              semantic_model::MemberAccess access);

void append_unique_function_templates(
    semantic_model::Scope & scope,
    const std::string & name,
    const std::vector<semantic_model::FunctionTemplateDecl *> & templates);

}  // namespace semantic_scope_mutation
