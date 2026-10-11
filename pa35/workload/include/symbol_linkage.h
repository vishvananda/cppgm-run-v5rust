#pragma once

#include <cstddef>
#include <map>
#include <memory>
#include <utility>
#include <string>
#include <vector>

#include "cpp_decl_model.h"
#include "template_model.h"
#include "abi_private_model.h"

namespace semantic_model {
struct ClassInfo;
struct Scope;
}

struct CppAstNode;

namespace symbol_linkage {

enum SymbolLinkage
{
  SL_INTERNAL,
  SL_EXTERNAL,
  SL_WEAK
};

enum FunctionRefQualifier
{
  FRQ_NONE,
  FRQ_LVALUE,
  FRQ_RVALUE
};

enum SpecialMemberEntryPointKind
{
  SMEK_COMPLETE,
  SMEK_BASE,
  SMEK_DELETING
};

struct FunctionSymbolOptions
{
  struct OwnerTemplateComponent
  {
    std::string template_name;
    const std::vector<template_model::TemplateParameterInfo> * parameters = nullptr;
    const std::vector<template_model::TemplateArgument> * arguments = nullptr;
    const std::vector<template_model::TemplateParameterInfo> * mangle_parameters = nullptr;
    const std::vector<cpp_decl::TemplateArgumentSyntax> * argument_syntaxes = nullptr;
  };

  bool is_member_function = false;
  bool has_implicit_object_parameter = false;
  bool is_const_method = false;
  bool is_volatile_method = false;
  FunctionRefQualifier ref_qualifier = FRQ_NONE;
  std::vector<std::string> abi_tags;
  bool is_constructor = false;
  bool is_destructor = false;
  bool is_conversion_operator = false;
  SpecialMemberEntryPointKind special_member_entry_point_kind = SMEK_COMPLETE;
  cpp_decl::TypePtr function_type_pattern;
  const std::vector<template_model::TemplateParameterInfo> * template_parameters = nullptr;
  const std::vector<template_model::TemplateArgument> * template_arguments = nullptr;
  const std::map<std::string, std::size_t> * template_argument_pack_sizes = nullptr;
  const std::vector<template_model::TemplateParameterInfo> * owner_template_parameters = nullptr;
  const std::vector<template_model::TemplateArgument> * owner_template_arguments = nullptr;
  const std::vector<template_model::TemplateParameterInfo> * owner_mangle_parameters = nullptr;
  std::string owner_template_name;
  std::vector<OwnerTemplateComponent> owner_template_components;
  const std::vector<std::pair<std::string, cpp_decl::TypePtr> > * parameter_pattern = nullptr;
  const CppAstNode * result_type_pattern = nullptr;
  const std::vector<const CppAstNode *> * parameter_declarations_pattern = nullptr;
  bool has_trailing_function_parameter_pack = false;
  bool suppress_template_argument_pack_grouping = false;
  const semantic_model::Scope * lookup_scope = nullptr;
  cpp_decl::TypePtr lambda_closure_type;
  cpp_decl::TypePtr local_class_type;
};

std::shared_ptr<void> make_lambda_context_function_symbol_options(
    const FunctionSymbolOptions & options);

class AbiMangleFactCaptureScope
{
public:
  explicit AbiMangleFactCaptureScope(bool enabled);
  ~AbiMangleFactCaptureScope();

private:
  bool previous_enabled;
};

struct SymbolIdentity
{
  struct AbiMangleFactEntry;
  struct AbiMangleFactEntries;

  SymbolIdentity();
  SymbolIdentity(const SymbolIdentity & rhs);
  SymbolIdentity(SymbolIdentity && rhs) noexcept;
  SymbolIdentity & operator=(const SymbolIdentity & rhs);
  SymbolIdentity & operator=(SymbolIdentity && rhs) noexcept;
  ~SymbolIdentity();

  std::string internal_symbol;
  std::string object_symbol;
  std::string thread_local_wrapper_object_symbol;
  std::unique_ptr<AbiMangleFactEntries> abi_mangle_facts;
  bool keep_internal_alias = false;
  bool prefer_local_object_binding = false;
  SymbolLinkage linkage = SL_EXTERNAL;
};

bool has_object_symbol(const SymbolIdentity & symbol);
bool has_exported_object_symbol(const SymbolIdentity & symbol);
std::string exported_object_symbol(const SymbolIdentity & symbol);
bool has_weak_linkage(const SymbolIdentity & symbol);
std::string mangle_symbol_name(const std::string & text);
std::string internal_symbol_from_name(const std::string & name);
bool type_needs_structural_internal_symbol(const cpp_decl::TypePtr & type);
std::string internal_symbol_from_type_encoding(const std::string & prefix,
                                               const cpp_decl::TypePtr & type);
std::string thread_local_wrapper_internal_symbol(const std::string & variable_internal_symbol);
std::string thread_local_guard_internal_symbol(const std::string & variable_internal_symbol);
std::string thread_local_wrapper_object_symbol_for_qualified_name(
    const cpp_decl::QualifiedName & qualified_name);
std::string thread_local_wrapper_object_symbol_for_scoped_variable(
    const semantic_model::Scope & scope,
    const std::string & name);
std::string thread_local_wrapper_object_symbol_for_static_member_variable(
    const semantic_model::ClassInfo & owner_class,
    const std::string & member_name);
std::string typeinfo_symbol_for_type(const cpp_decl::TypePtr & type);
std::string typeinfo_name_symbol_for_type(const cpp_decl::TypePtr & type);
std::string vtable_object_symbol_for_type(const cpp_decl::TypePtr & type);
bool mangle_itanium_type_encoding(const cpp_decl::TypePtr & type,
                                  std::string & out);
bool mangle_itanium_name_encoding(const cpp_decl::QualifiedName & qualified_name,
                                  std::string & out);
std::string virtual_override_thunk_object_symbol_for_function(
    const cpp_decl::QualifiedName & qualified_name,
    const std::string & display_name,
    bool is_c_linkage,
    const cpp_decl::TypePtr & type,
    const FunctionSymbolOptions & options,
    long long this_adjust,
    bool has_result_adjust = false,
    long long result_adjust = 0);
std::string virtual_override_thunk_object_symbol_for_object_symbol(
    const std::string & target_object_symbol,
    long long this_adjust,
    bool has_result_adjust = false,
    long long result_adjust = 0);
std::string virtual_base_override_thunk_object_symbol_for_function(
    const cpp_decl::QualifiedName & qualified_name,
    const std::string & display_name,
    bool is_c_linkage,
    const cpp_decl::TypePtr & type,
    const FunctionSymbolOptions & options,
    long long vcall_offset);
std::string virtual_base_override_thunk_object_symbol_for_object_symbol(
    const std::string & target_object_symbol,
    long long vcall_offset);
std::string construction_vtable_object_symbol(const semantic_model::ClassInfo & dynamic_class,
                                              unsigned long long base_offset,
                                              const semantic_model::ClassInfo & base_class);
std::string vtt_object_symbol_for_type(const cpp_decl::TypePtr & type);
std::string vtt_object_symbol(const semantic_model::ClassInfo & class_info);
std::size_t abi_mangle_fact_count(const SymbolIdentity & symbol);
const std::string & abi_mangle_fact_object_symbol(const SymbolIdentity & symbol,
                                                  std::size_t index);
unsigned abi_mangle_fact_target_kind(const SymbolIdentity & symbol,
                                     std::size_t index);
const std::string & abi_mangle_fact_target_qualified_name(
    const SymbolIdentity & symbol,
    std::size_t index);
bool abi_mangle_fact_target_c_linkage(const SymbolIdentity & symbol,
                                      std::size_t index);
const abi_mangle::AbiMangleTarget & abi_mangle_fact_target(
    const SymbolIdentity & symbol,
    std::size_t index);
SymbolIdentity make_c_function_symbol_identity(const std::string & name,
                                               SymbolLinkage linkage = SL_EXTERNAL);
SymbolIdentity make_function_symbol_identity(const cpp_decl::QualifiedName & qualified_name,
                                             const std::string & display_name,
                                             bool is_c_linkage,
                                             const cpp_decl::TypePtr & type,
                                             const FunctionSymbolOptions & options =
                                                 FunctionSymbolOptions(),
                                             const std::string & symbol_key = std::string(),
                                             SymbolLinkage linkage = SL_EXTERNAL);
SymbolIdentity make_internal_symbol_identity(const std::string & internal_symbol,
                                             SymbolLinkage linkage = SL_EXTERNAL);
SymbolIdentity make_object_symbol_identity(const std::string & internal_symbol,
                                           const std::string & object_symbol,
                                           SymbolLinkage linkage = SL_EXTERNAL);
SymbolIdentity make_scoped_variable_symbol_identity(
    const semantic_model::Scope & scope,
    const std::string & name,
    bool is_c_linkage,
    SymbolLinkage linkage = SL_EXTERNAL);
SymbolIdentity make_static_member_variable_symbol_identity(
    const semantic_model::ClassInfo & owner_class,
    const std::string & member_name,
    bool is_c_linkage,
    SymbolLinkage linkage = SL_EXTERNAL);
SymbolIdentity make_static_member_variable_template_symbol_identity(
    const semantic_model::ClassInfo & owner_class,
    const std::string & internal_member_name,
    const std::string & template_name,
    const std::vector<template_model::TemplateArgument> & template_arguments,
    const std::vector<template_model::TemplateParameterInfo> & template_parameters,
    bool is_c_linkage,
    SymbolLinkage linkage = SL_EXTERNAL);
bool has_external_vtable_symbol_candidate(const cpp_decl::TypePtr & type);

}  // namespace symbol_linkage
