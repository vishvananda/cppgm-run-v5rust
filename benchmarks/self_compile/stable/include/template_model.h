#pragma once

#include <functional>
#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "cpp_decl_model.h"
#include "cppast_ast.h"

namespace semantic_model {
struct AliasTemplateDecl;
struct ClassInfo;
struct ClassTemplateDecl;
struct FunctionBinding;
struct Scope;
struct ValueBinding;
}

namespace template_model {

struct TemplateValueDependency
{
  std::string entity;
  std::string decl_location;
  std::string public_use_location;
  semantic_model::Scope * value_scope = nullptr;
  std::string value_name;
  bool entity_has_template_identity = true;
};

struct TemplateParameterOwnedSyntax
{
  std::unique_ptr<CppAstNode> non_type_decl_specifier_seq;
  std::unique_ptr<CppAstNode> non_type_declarator;
  std::unique_ptr<CppAstNode> non_type_abstract_declarator;
  std::unique_ptr<CppAstNode> default_argument;
};

struct TemplateParameterInfo
{
  enum Kind
  {
    TP_TYPE,
    TP_NON_TYPE,
    TP_TEMPLATE_TEMPLATE
  };

  Kind kind = TP_TYPE;
  std::string name;
  std::vector<std::string> alternate_names;
  bool parameter_pack = false;
  std::size_t template_parameter_count = 0;
  std::unique_ptr<std::vector<TemplateParameterInfo> > template_parameters;
  std::string placeholder_key;
  cpp_decl::TypePtr value_type;
  std::shared_ptr<TemplateParameterOwnedSyntax> owned_syntax;
  const CppAstNode * non_type_decl_specifier_seq = nullptr;
  std::string non_type_decl_specifier_text;
  const CppAstNode * non_type_declarator = nullptr;
  const CppAstNode * non_type_abstract_declarator = nullptr;
  const CppAstNode * default_argument = nullptr;

  TemplateParameterInfo();
  TemplateParameterInfo(const TemplateParameterInfo & other);
  TemplateParameterInfo(TemplateParameterInfo && other) noexcept;
  TemplateParameterInfo & operator=(const TemplateParameterInfo & other);
  TemplateParameterInfo & operator=(TemplateParameterInfo && other) noexcept;
};

inline const CppAstNode * store_template_parameter_ast(
    std::shared_ptr<TemplateParameterOwnedSyntax> & storage,
    std::unique_ptr<CppAstNode> TemplateParameterOwnedSyntax::* member,
    const CppAstNode * node)
{
  if(!node) {
    if(storage) {
      ((*storage).*member).reset();
    }
    return nullptr;
  }
  if(!storage) {
    storage.reset(new TemplateParameterOwnedSyntax);
  }
  ((*storage).*member).reset(new CppAstNode(*node));
  return ((*storage).*member).get();
}

inline const CppAstNode * copy_template_parameter_ast(
    std::shared_ptr<TemplateParameterOwnedSyntax> & target_storage,
    const std::shared_ptr<TemplateParameterOwnedSyntax> & source_storage,
    std::unique_ptr<CppAstNode> TemplateParameterOwnedSyntax::* member,
    const CppAstNode * source)
{
  if(!source) {
    return nullptr;
  }
  if(!target_storage) {
    target_storage.reset(new TemplateParameterOwnedSyntax);
  }
  ((*target_storage).*member).reset(new CppAstNode(*source));
  return ((*target_storage).*member).get();
}

inline void set_template_parameter_default_argument(
    TemplateParameterInfo & parameter,
    const CppAstNode * node)
{
  parameter.default_argument =
      store_template_parameter_ast(parameter.owned_syntax,
                                   &TemplateParameterOwnedSyntax::default_argument,
                                   node);
}

inline void copy_template_parameter_default_argument(
    TemplateParameterInfo & target,
    const TemplateParameterInfo & source)
{
  target.default_argument =
      copy_template_parameter_ast(target.owned_syntax,
                                  source.owned_syntax,
                                  &TemplateParameterOwnedSyntax::default_argument,
                                  source.default_argument);
}

inline void set_template_parameter_non_type_decl_specifier_seq(
    TemplateParameterInfo & parameter,
    const CppAstNode * node)
{
  parameter.non_type_decl_specifier_seq =
      store_template_parameter_ast(
          parameter.owned_syntax,
          &TemplateParameterOwnedSyntax::non_type_decl_specifier_seq,
          node);
}

inline void copy_template_parameter_non_type_decl_specifier_seq(
    TemplateParameterInfo & target,
    const TemplateParameterInfo & source)
{
  target.non_type_decl_specifier_seq =
      copy_template_parameter_ast(
          target.owned_syntax,
          source.owned_syntax,
          &TemplateParameterOwnedSyntax::non_type_decl_specifier_seq,
          source.non_type_decl_specifier_seq);
}

inline void set_template_parameter_non_type_declarator(
    TemplateParameterInfo & parameter,
    const CppAstNode * node)
{
  parameter.non_type_declarator =
      store_template_parameter_ast(parameter.owned_syntax,
                                   &TemplateParameterOwnedSyntax::non_type_declarator,
                                   node);
}

inline void copy_template_parameter_non_type_declarator(
    TemplateParameterInfo & target,
    const TemplateParameterInfo & source)
{
  target.non_type_declarator =
      copy_template_parameter_ast(target.owned_syntax,
                                  source.owned_syntax,
                                  &TemplateParameterOwnedSyntax::non_type_declarator,
                                  source.non_type_declarator);
}

inline void set_template_parameter_non_type_abstract_declarator(
    TemplateParameterInfo & parameter,
    const CppAstNode * node)
{
  parameter.non_type_abstract_declarator =
      store_template_parameter_ast(
          parameter.owned_syntax,
          &TemplateParameterOwnedSyntax::non_type_abstract_declarator,
          node);
}

inline void copy_template_parameter_non_type_abstract_declarator(
    TemplateParameterInfo & target,
    const TemplateParameterInfo & source)
{
  target.non_type_abstract_declarator =
      copy_template_parameter_ast(
          target.owned_syntax,
          source.owned_syntax,
          &TemplateParameterOwnedSyntax::non_type_abstract_declarator,
          source.non_type_abstract_declarator);
}

inline void copy_template_parameter_info(
    TemplateParameterInfo & target,
    const TemplateParameterInfo & source)
{
  target.kind = source.kind;
  target.name = source.name;
  target.alternate_names = source.alternate_names;
  target.parameter_pack = source.parameter_pack;
  target.template_parameter_count = source.template_parameter_count;
  target.template_parameters.reset(
      source.template_parameters ?
          new std::vector<TemplateParameterInfo>(*source.template_parameters) :
          nullptr);
  target.placeholder_key = source.placeholder_key;
  target.value_type = source.value_type;
  target.owned_syntax.reset();
  target.non_type_decl_specifier_seq = nullptr;
  target.non_type_decl_specifier_text = source.non_type_decl_specifier_text;
  target.non_type_declarator = nullptr;
  target.non_type_abstract_declarator = nullptr;
  target.default_argument = nullptr;

  copy_template_parameter_non_type_decl_specifier_seq(target, source);
  copy_template_parameter_non_type_declarator(target, source);
  copy_template_parameter_non_type_abstract_declarator(target, source);
  copy_template_parameter_default_argument(target, source);
}

inline void move_template_parameter_info(
    TemplateParameterInfo & target,
    TemplateParameterInfo & source)
{
  target.kind = source.kind;
  target.name = std::move(source.name);
  target.alternate_names = std::move(source.alternate_names);
  target.parameter_pack = source.parameter_pack;
  target.template_parameter_count = source.template_parameter_count;
  target.template_parameters = std::move(source.template_parameters);
  target.placeholder_key = std::move(source.placeholder_key);
  target.value_type = std::move(source.value_type);
  target.owned_syntax = std::move(source.owned_syntax);
  target.non_type_decl_specifier_seq = source.non_type_decl_specifier_seq;
  target.non_type_decl_specifier_text =
      std::move(source.non_type_decl_specifier_text);
  target.non_type_declarator = source.non_type_declarator;
  target.non_type_abstract_declarator = source.non_type_abstract_declarator;
  target.default_argument = source.default_argument;

  source.non_type_decl_specifier_seq = nullptr;
  source.non_type_declarator = nullptr;
  source.non_type_abstract_declarator = nullptr;
  source.default_argument = nullptr;
}

inline TemplateParameterInfo::TemplateParameterInfo()
{
}

inline TemplateParameterInfo::TemplateParameterInfo(const TemplateParameterInfo & other)
{
  copy_template_parameter_info(*this, other);
}

inline TemplateParameterInfo::TemplateParameterInfo(
    TemplateParameterInfo && other) noexcept
{
  move_template_parameter_info(*this, other);
}

inline TemplateParameterInfo & TemplateParameterInfo::operator=(
    const TemplateParameterInfo & other)
{
  if(this != &other) {
    copy_template_parameter_info(*this, other);
  }
  return *this;
}

inline TemplateParameterInfo & TemplateParameterInfo::operator=(
    TemplateParameterInfo && other) noexcept
{
  if(this != &other) {
    move_template_parameter_info(*this, other);
  }
  return *this;
}

struct TemplateArgument
{
  struct TemplateEntityIdentity
  {
    std::string scope_prefix;
    std::string name;
    cpp_decl::QualifiedName name_syntax;
  };

  enum Kind
  {
    TA_TYPE,
    TA_VALUE,
    TA_CLASS_TEMPLATE,
    TA_ALIAS_TEMPLATE
  };

  Kind kind = TA_TYPE;
  cpp_decl::TypePtr type;
  void * template_decl = nullptr;
  cpp_decl::TypePtr template_owner_type;
  std::shared_ptr<TemplateEntityIdentity> template_entity_identity;
  const semantic_model::FunctionBinding * function_value = nullptr;
  std::string function_internal_symbol;
  const semantic_model::ValueBinding * value_binding = nullptr;
  std::string text;
  std::vector<TemplateValueDependency> value_dependencies;
  std::shared_ptr<cpp_decl::TemplateArgumentSyntax> source_syntax;
  std::shared_ptr<CppAstNode> expression;
  long long value = 0;
  bool dependent = false;
  bool source_defaulted = false;
  bool partial_order_placeholder = false;

  const std::string & template_entity_scope_prefix() const
  {
    static const std::string empty;
    return template_entity_identity ? template_entity_identity->scope_prefix : empty;
  }

  const std::string & template_entity_name() const
  {
    static const std::string empty;
    return template_entity_identity ? template_entity_identity->name : empty;
  }

  const cpp_decl::QualifiedName & template_entity_name_syntax() const
  {
    static const cpp_decl::QualifiedName empty;
    return template_entity_identity ? template_entity_identity->name_syntax : empty;
  }
};

inline TemplateArgument::TemplateEntityIdentity &
mutable_template_argument_entity_identity(TemplateArgument & argument)
{
  if(!argument.template_entity_identity) {
    argument.template_entity_identity.reset(
        new TemplateArgument::TemplateEntityIdentity());
  } else if(!argument.template_entity_identity.unique()) {
    argument.template_entity_identity.reset(
        new TemplateArgument::TemplateEntityIdentity(
            *argument.template_entity_identity));
  }
  return *argument.template_entity_identity;
}

inline void set_template_argument_entity_identity(
    TemplateArgument & argument,
    const std::string & scope_prefix,
    const std::string & name)
{
  TemplateArgument::TemplateEntityIdentity & identity =
      mutable_template_argument_entity_identity(argument);
  identity.scope_prefix = scope_prefix;
  identity.name = name;
}

inline void set_template_argument_entity_name_syntax(
    TemplateArgument & argument,
    const cpp_decl::QualifiedName & syntax)
{
  mutable_template_argument_entity_identity(argument).name_syntax = syntax;
}

bool template_arguments_are_dependent(
    const std::vector<TemplateArgument> & arguments,
    const std::function<bool(const cpp_decl::TypePtr &)> & type_is_dependent);
bool template_arguments_fully_bind_parameters(
    const std::vector<TemplateParameterInfo> & parameters,
    const std::vector<TemplateArgument> & arguments);
std::string template_argument_text(
    const TemplateArgument & arg,
    const std::function<std::string(const cpp_decl::TypePtr &)> & type_text);
semantic_model::AliasTemplateDecl * template_argument_alias_template(
    const TemplateArgument & arg);
semantic_model::ClassTemplateDecl * template_argument_class_template(
    const TemplateArgument & arg);
std::string template_argument_key(
    const std::vector<TemplateArgument> & args,
    const std::function<std::string(const cpp_decl::TypePtr &)> & type_text);
const TemplateParameterInfo * find_template_parameter_by_name(
    const std::vector<TemplateParameterInfo> & parameters,
    const std::string & name);
const TemplateParameterInfo * find_template_parameter(
    const std::vector<TemplateParameterInfo> & parameters,
    const std::string & placeholder_key);
const TemplateParameterInfo * find_template_parameter(
    const std::vector<TemplateParameterInfo> & parameters,
    const cpp_decl::TypePtr & type);

}  // namespace template_model
