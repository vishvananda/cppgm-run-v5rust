#pragma once

#include <string>
#include <vector>

#include "cpp_decl_model.h"
#include "semantic_model.h"

class SemanticContext;

namespace semantic_dependent_type {

bool resolve_instantiated_dependent_type(SemanticContext & ctx,
                                         semantic_model::Scope & scope,
                                         const cpp_decl::TypePtr & type,
                                         cpp_decl::TypePtr & out);

std::string lookup_type_argument_text(SemanticContext & ctx,
                                      const cpp_decl::TypePtr & type);

}  // namespace semantic_dependent_type
