#pragma once

#include "semantic_model.h"

namespace template_api {

struct TemplateEnvironmentHandle
{
  TemplateEnvironmentHandle() {}

  explicit TemplateEnvironmentHandle(semantic_model::Scope & scope_in)
    : scope(&scope_in)
  {}

  bool valid() const
  {
    return scope != nullptr;
  }

  semantic_model::Scope & require() const
  {
    return *scope;
  }

  semantic_model::Scope * scope = nullptr;
};

inline TemplateEnvironmentHandle make_template_environment(semantic_model::Scope & scope)
{
  return TemplateEnvironmentHandle(scope);
}

}  // namespace template_api
