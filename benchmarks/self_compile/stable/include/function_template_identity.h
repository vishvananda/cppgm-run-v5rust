#pragma once

#include <string>
#include <vector>

namespace semantic_model {
struct FunctionTemplateDecl;
}  // namespace semantic_model

namespace template_model {
struct TemplateArgument;
}  // namespace template_model

struct FunctionTemplateRegistrationIdentity
{
  semantic_model::FunctionTemplateDecl * decl = nullptr;
  const std::vector<template_model::TemplateArgument> * arguments = nullptr;
  bool arguments_present = false;
  std::string key;
  bool prefer_overload_suffix = false;

  bool has_decl() const { return decl != nullptr; }
  bool has_arguments() const { return arguments_present || arguments != nullptr; }
};
