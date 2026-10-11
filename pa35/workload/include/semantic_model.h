#pragma once

#include <functional>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

#include "callsem_output.h"
#include "constant_value.h"
#include "cpp_decl_model.h"
#include "cppast_ast.h"
#include "symbol_linkage.h"
#include "template_model.h"

namespace semantic_model {

struct ClassInfo;
// Index of instantiated classes keyed by their mangled name. Used only for
// keyed lookup (never iterated for output), so it is hashed rather than ordered.
typedef std::unordered_map<std::string, ClassInfo *> ClassIndexMap;
struct FunctionBinding;
struct FunctionTemplateDecl;
struct VariableTemplateDecl;
struct AliasTemplateDecl;
struct ClassTemplateDecl;
struct DeductionGuideDecl;

struct FunctionTemplateInstantiationCacheEntry
{
  FunctionTemplateDecl * decl = nullptr;
  std::string key;
};

struct FunctionTemplateInstantiationCacheEntries
{
  FunctionTemplateInstantiationCacheEntry first;
  std::vector<FunctionTemplateInstantiationCacheEntry> extra;
};
struct ClassTemplateSpecializationDecl;
struct PartialClassTemplateSpecializationDecl;
struct VariableTemplateSpecializationDecl;
struct Scope;
// Index of durable type scopes keyed by their mangled name. Used only for keyed
// lookup (never iterated for output), so it is hashed rather than ordered.
typedef std::unordered_map<std::string, Scope *> TypeScopeIndexMap;

enum ExplicitFunctionNothrowKind
{
  EFNK_UNINITIALIZED,
  EFNK_NONE,
  EFNK_ALWAYS_TRUE,
  EFNK_ALWAYS_FALSE,
  EFNK_EXPR,
  EFNK_INVALID
};

enum BuiltinConstantEvaluationKind
{
  BCEK_NONE,
  BCEK_EXPECT
};

std::size_t next_scope_instance_id();

enum MemberAccess : int
{
  MA_PUBLIC,
  MA_PROTECTED,
  MA_PRIVATE
};

enum RefQualifier : int
{
  RQ_NONE,
  RQ_LVALUE,
  RQ_RVALUE
};

struct FunctionSemanticFlags
{
  MemberAccess access = MA_PUBLIC;
  bool is_constructor = false;
  bool is_inherited_constructor = false;
  bool is_destructor = false;
  bool is_conversion_operator = false;
  bool is_explicit = false;
  bool is_const_method = false;
  bool is_volatile_method = false;
  bool is_variadic = false;
  RefQualifier ref_qualifier = RQ_NONE;
  bool is_virtual_specified = false;
  bool is_override_specified = false;
  bool is_final = false;
  bool is_defaulted = false;
  bool is_deleted = false;
  bool is_constexpr = false;
  bool is_inline = false;
  const CppAstNode * function_qualifier = nullptr;
};

enum OutputRequirementKind : unsigned int
{
  ORK_NONE = 0,
  ORK_DECLARATION = 1u << 0,
  ORK_DEFINITION = 1u << 1,
  ORK_EXPORT = 1u << 2,
  ORK_RUNTIME = 1u << 3
};

inline bool has_output_requirement(unsigned int flags, OutputRequirementKind kind)
{
  return (flags & static_cast<unsigned int>(kind)) != 0;
}

inline void add_output_requirement(unsigned int & flags, OutputRequirementKind kind)
{
  flags |= static_cast<unsigned int>(kind);
}

inline void remove_output_requirement(unsigned int & flags, OutputRequirementKind kind)
{
  flags &= ~static_cast<unsigned int>(kind);
}

struct SourceDeclAnchorCache
{
  mutable bool cached = false;
  mutable std::string name_location;
  mutable std::string approximate_location;
};

inline const std::string & source_decl_anchor_location(
    const SourceDeclAnchorCache & cache)
{
  return !cache.name_location.empty() ? cache.name_location :
                                        cache.approximate_location;
}

inline bool source_decl_anchor_has_name_location(
    const SourceDeclAnchorCache & cache)
{
  return !cache.name_location.empty();
}

struct VariableTemplateInstantiationIdentity
{
  VariableTemplateDecl * source_template = nullptr;
  std::vector<template_model::TemplateArgument> arguments;
};

struct ValueBinding
{
  enum Kind
  {
    VK_VARIABLE,
    VK_PARAMETER,
    VK_FIELD
  };

  ValueBinding() : kind(VK_VARIABLE) {}
  ValueBinding(Kind kind, const std::string & name, const cpp_decl::TypePtr & type)
    : kind(kind), name(name), type(type)
  {}

  Kind kind;
  std::string name;
  cpp_decl::TypePtr type;
  ClassInfo * owner_class = nullptr;
  std::size_t field_offset = 0;
  std::string anonymous_storage_field_name;
  std::string anonymous_storage_variable_name;
  std::size_t anonymous_storage_member_offset = 0;
  bool is_mutable = false;
  bool is_bit_field = false;
  std::size_t bit_field_width = 0;
  std::size_t bit_field_offset = 0;
  std::size_t bit_field_storage_size = 0;
  MemberAccess access = MA_PUBLIC;
  bool is_c_linkage = false;
  bool is_thread_local = false;
  bool has_storage_definition = true;
  bool is_explicit_specialization = false;
  bool has_constant_value = false;
  long long constant_value = 0;
  bool has_constexpr_value = false;
  std::shared_ptr<constant_eval::ConstexprValue> constexpr_value;
  bool dependent_template_value = false;
  std::string non_type_template_argument_text;
  std::shared_ptr<CppAstNode> non_type_template_argument_expression;
  FunctionBinding * non_type_template_function_value = nullptr;
  std::string non_type_template_function_internal_symbol;
  const ValueBinding * non_type_template_value_binding = nullptr;
  std::shared_ptr<VariableTemplateInstantiationIdentity> variable_template_instantiation;
  Scope * declaration_scope = nullptr;
  const CppAstNode * declaration_node = nullptr;
  const CppAstNode * definition_node = nullptr;
  const CppAstNode * constant_initializer = nullptr;
  Scope * constant_initializer_scope = nullptr;
  bool constant_value_in_progress = false;
  bool requires_constant_initializer = false;
  symbol_linkage::SymbolIdentity symbol;
  unsigned int output_requirements = ORK_NONE;
  bool definition_output_emitted = false;
  mutable bool witness_member_value_instantiation_noted = false;
  mutable bool witness_static_member_definition_replayed = false;
  mutable bool witness_static_member_definition_source_captured = false;
  mutable SourceDeclAnchorCache declaration_anchor;
};

inline bool value_binding_has_constexpr_value(const ValueBinding & binding)
{
  return binding.has_constexpr_value && binding.constexpr_value;
}

inline const constant_eval::ConstexprValue & value_binding_constexpr_value(
    const ValueBinding & binding)
{
  return *binding.constexpr_value;
}

inline void set_value_binding_constexpr_value(
    ValueBinding & binding,
    const constant_eval::ConstexprValue & value)
{
  binding.has_constexpr_value = true;
  binding.constexpr_value.reset(new constant_eval::ConstexprValue(value));
}

inline void clear_value_binding_constexpr_value(ValueBinding & binding)
{
  binding.has_constexpr_value = false;
  binding.constexpr_value.reset();
}

struct Scope
{
  explicit Scope(Scope * parent = nullptr,
                 const std::string & name = std::string(),
                 bool namespace_scope = false)
    : parent(parent),
      name(name),
      namespace_scope(namespace_scope),
      instance_id(next_scope_instance_id())
  {}

  Scope(const Scope & other);

  Scope & operator=(const Scope & other)
  {
    if(this == &other) {
      return *this;
    }

    parent = other.parent;
    name = other.name;
    namespace_scope = other.namespace_scope;
    inline_namespace = other.inline_namespace;
    class_info = other.class_info;
    function = other.function;
    named_types = other.named_types;
    named_type_access = other.named_type_access;
    named_type_packs = other.named_type_packs;
    named_value_packs = other.named_value_packs;
    named_pack_sizes = other.named_pack_sizes;
    template_bound_type_names = other.template_bound_type_names;
    template_bound_type_pack_names = other.template_bound_type_pack_names;
    template_bound_value_names = other.template_bound_value_names;
    template_bound_value_pack_names = other.template_bound_value_pack_names;
    template_bound_template_names = other.template_bound_template_names;
    template_bound_template_arguments = other.template_bound_template_arguments;
    values = other.values;
    namespace_bindings = other.namespace_bindings;
    function_sets = other.function_sets;
    function_set_access_overrides = other.function_set_access_overrides;
    cached_direct_function_lookups.clear();
    direct_function_lookup_cache_epoch = other.direct_function_lookup_cache_epoch;
    class_templates = other.class_templates;
    function_templates = other.function_templates;
    collected_template_declarations = other.collected_template_declarations;
    alias_templates = other.alias_templates;
    variable_templates = other.variable_templates;
    using_directives = other.using_directives;
    namespace_children.clear();
    instance_id = next_scope_instance_id();
    binding_fingerprint_epoch = other.binding_fingerprint_epoch;
    cached_binding_scope_fingerprint_valid = false;
    cached_binding_scope_fingerprint = 0;
    cached_binding_scope_fingerprint_epoch = 0;
    cached_binding_scope_parent_fingerprint = 0;
    cached_binding_scope_global_epoch = 0;
    cached_instance_scope_fingerprint_valid = false;
    cached_instance_scope_fingerprint = 0;
    return *this;
  }

  Scope(Scope && other);
  Scope & operator=(Scope && other);

  Scope * parent;
  std::string name;
  bool namespace_scope;
  bool inline_namespace = false;
  ClassInfo * class_info = nullptr;
  FunctionBinding * function = nullptr;
  // Point-lookup keyed by type name; iteration order is not relied upon (the one
  // order-sensitive lookup, dependent_member_type_owner_text_from_scope, picks the
  // lexicographically-smallest match explicitly).
  typedef std::unordered_map<std::string, cpp_decl::TypePtr> NamedTypeMap;
  NamedTypeMap named_types;
  std::map<std::string, MemberAccess> named_type_access;
  std::map<std::string, std::vector<cpp_decl::TypePtr> > named_type_packs;
  std::map<std::string, std::vector<ValueBinding> > named_value_packs;
  std::map<std::string, std::size_t> named_pack_sizes;
  std::set<std::string> template_bound_type_names;
  std::set<std::string> template_bound_type_pack_names;
  std::set<std::string> template_bound_value_names;
  std::set<std::string> template_bound_value_pack_names;
  std::set<std::string> template_bound_template_names;
  std::map<std::string, template_model::TemplateArgument> template_bound_template_arguments;
  std::map<std::string, ValueBinding> values;
  std::map<std::string, Scope *> namespace_bindings;
  std::map<std::string, std::vector<FunctionBinding *> > function_sets;
  std::map<std::string, std::map<const FunctionBinding *, MemberAccess> >
      function_set_access_overrides;
  std::map<std::string, ClassTemplateDecl *> class_templates;
  std::map<std::string, std::vector<FunctionTemplateDecl *> > function_templates;
  std::set<const CppAstNode *> collected_template_declarations;
  std::map<std::string, AliasTemplateDecl *> alias_templates;
  std::map<std::string, VariableTemplateDecl *> variable_templates;
  std::vector<Scope *> using_directives;
  std::vector<std::unique_ptr<Scope> > namespace_children;
  std::size_t instance_id;
  std::size_t binding_fingerprint_epoch = 0;
  struct DirectFunctionLookupCacheEntry
  {
    std::size_t dependency_token = 0;
    std::vector<FunctionBinding *> functions;
  };
  mutable std::map<std::string, DirectFunctionLookupCacheEntry>
      cached_direct_function_lookups;
  std::size_t direct_function_lookup_cache_epoch = 0;
  mutable bool cached_binding_scope_fingerprint_valid = false;
  mutable std::size_t cached_binding_scope_fingerprint = 0;
  mutable std::size_t cached_binding_scope_fingerprint_epoch = 0;
  mutable std::size_t cached_binding_scope_parent_fingerprint = 0;
  mutable std::size_t cached_binding_scope_global_epoch = 0;
  mutable bool cached_instance_scope_fingerprint_valid = false;
  mutable std::size_t cached_instance_scope_fingerprint = 0;
};

struct FunctionBinding
{
  std::string name;
  std::string display_name;
  cpp_decl::TypePtr declared_type;
  cpp_decl::TypePtr type;
  Scope * declaration_scope = nullptr;
  std::vector<std::pair<std::string, cpp_decl::TypePtr> > params;
  std::vector<std::string> parameter_aliases;
  std::vector<const CppAstNode *> default_arguments;
  const CppAstNode * declaration_node = nullptr;
  const CppAstNode * definition_node = nullptr;
  std::vector<std::string> declaration_abi_tags;
  std::vector<std::string> definition_abi_tags;
  bool definition_suppresses_declaration_abi_tags = false;
  const CppAstNode * parameter_syntax_node = nullptr;
  const CppAstNode * body = nullptr;
  const CppAstNode * function_qualifier = nullptr;
  bool has_definition = false;
  ClassInfo * owner_class = nullptr;
  ClassInfo * lexical_access_class = nullptr;
  FunctionBinding * lexical_access_function = nullptr;
  bool hidden_friend_only = false;
  MemberAccess access = MA_PUBLIC;
  bool is_method = false;
  bool is_constexpr = false;
  bool is_inline = false;
  bool is_constructor = false;
  bool is_inherited_constructor = false;
  bool is_destructor = false;
  bool is_conversion_operator = false;
  bool is_defaulted = false;
  bool is_explicit = false;
  bool is_copy_constructor = false;
  bool is_move_constructor = false;
  bool is_copy_assignment = false;
  bool is_move_assignment = false;
  bool is_const_method = false;
  bool is_volatile_method = false;
  RefQualifier ref_qualifier = RQ_NONE;
  bool is_deleted = false;
  bool is_virtual = false;
  bool is_virtual_specified = false;
  bool is_pure_virtual = false;
  bool is_override_specified = false;
  bool is_final = false;
  bool is_c_linkage = false;
  bool is_builtin = false;
  BuiltinConstantEvaluationKind builtin_constant_evaluation_kind = BCEK_NONE;
  bool permits_host_builtin_missing_nothrow_redeclaration = false;
  std::string object_symbol_override;
  symbol_linkage::SymbolIdentity symbol;
  mutable bool cached_lookup_dedupe_key_valid = false;
  mutable std::size_t cached_lookup_dedupe_name_hash = 0;
  mutable std::size_t cached_lookup_dedupe_type_hash = 0;
  mutable const cpp_decl::Type * cached_lookup_dedupe_type = nullptr;
  mutable std::size_t cached_lookup_dedupe_name_size = 0;
  bool has_virtual_slot = false;
  std::size_t virtual_slot = 0;
  bool synthesized = false;
  bool is_aggregate_constructor = false;
  unsigned int output_requirements = ORK_NONE;
  bool output_emitted = false;
  bool definition_output_in_progress = false;
  bool definition_output_emitted = false;
  bool odr_mergeable_definition = false;
  bool template_definition_materialized_by_enclosing_closure = false;
  bool template_definition_required_by_public_source_call = false;
  ExplicitFunctionNothrowKind explicit_function_nothrow_kind = EFNK_UNINITIALIZED;
  const CppAstNode * explicit_function_nothrow_cached_qualifier = nullptr;
  std::string explicit_function_nothrow_expr_text;
  bool explicit_function_nothrow_eval_cached = false;
  bool explicit_function_nothrow_eval_value = false;
  bool suppress_implicit_instantiation_definition = false;
  bool is_explicit_instantiation_definition = false;
  bool exclude_from_explicit_instantiation = false;
  std::unique_ptr<CallSemNode> cached_body_output;
  const CppAstNode * ctor_initializer = nullptr;
  FunctionTemplateDecl * source_template = nullptr;
  std::vector<template_model::TemplateArgument> instantiation_arguments;
  bool has_instantiation_arguments = false;
  std::string instantiation_use_location;
  bool is_explicit_specialization = false;
  std::map<std::string, std::size_t> instantiation_pack_sizes;
  std::string template_instantiation_key;
  std::unique_ptr<FunctionTemplateInstantiationCacheEntries>
      instantiation_cache_entries;
  mutable SourceDeclAnchorCache declaration_anchor;
};

inline std::string function_output_name(const FunctionBinding & binding);
std::string function_binding_display_name_for_symbol(const FunctionBinding & binding);
std::string predefined_pretty_function_text(const FunctionBinding & binding);
std::string function_binding_qualified_name_for_symbol(const FunctionBinding & binding);
bool function_binding_qualified_name_syntax_for_symbol(
    const FunctionBinding & binding,
    cpp_decl::QualifiedName & out);

inline std::vector<std::string> function_binding_abi_tags(const FunctionBinding & binding);

struct FieldInfo
{
  std::string name;
  cpp_decl::TypePtr type;
  const CppAstNode * default_initializer = nullptr;
  const CppAstNode * bit_width_expression = nullptr;
  std::size_t offset = 0;
  bool is_mutable = false;
  bool is_no_unique_address = false;
  bool is_anonymous_storage = false;
  bool is_bit_field = false;
  std::size_t bit_width = 0;
  std::size_t bit_offset = 0;
  std::size_t bit_storage_size = 0;
  MemberAccess access = MA_PUBLIC;
};

struct BaseInfo
{
  ClassInfo * type = nullptr;
  MemberAccess access = MA_PUBLIC;
  std::size_t offset = 0;
  bool is_virtual = false;
  bool source_dependent = false;
};

struct AnonymousMemberClassInfo
{
  std::string class_kind;
  const CppAstNode * class_node = nullptr;
};

struct SubobjectInfo
{
  ClassInfo * type = nullptr;
  std::size_t offset = 0;
  MemberAccess access = MA_PUBLIC;
  bool is_virtual = false;
};

struct VTableSlotInfo
{
  FunctionBinding * function = nullptr;
  long long this_adjust = 0;
};

struct VTableInfo
{
  std::string key;
  ClassInfo * view_type = nullptr;
  std::size_t view_offset = 0;
  bool use_extended_layout = false;
  std::vector<VTableSlotInfo> slots;
};

struct ClassInfo
{
  std::string name;
  std::string qualified_name;
  cpp_decl::QualifiedName symbol_qualified_name_syntax;
  std::string display_qualified_name;
  std::string class_kind;
  std::string creation_context;
  cpp_decl::TypePtr type;
  Scope * enclosing_scope = nullptr;
  const CppAstNode * class_node = nullptr;
  const CppAstNode * template_output_node = nullptr;
  std::unique_ptr<Scope> member_scope;
  std::vector<FieldInfo> fields;
  std::map<std::string, std::vector<FunctionBinding *> > methods;
  std::vector<FunctionBinding *> method_declaration_order;
  std::vector<BaseInfo> bases;
  std::vector<AnonymousMemberClassInfo> anonymous_member_classes;
  std::vector<FunctionBinding *> vtable_entries;
  std::vector<SubobjectInfo> complete_subobjects;
  std::vector<SubobjectInfo> virtual_base_subobjects;
  std::vector<VTableInfo> vtables;
  std::vector<FunctionBinding *> friend_functions;
  std::vector<FunctionBinding *> friend_access_functions;
  std::vector<FunctionTemplateDecl *> friend_function_templates;
  std::vector<FunctionTemplateDecl *> friend_access_function_templates;
  std::vector<std::string> friend_class_names;
  struct DeferredMemberAlias
  {
    const CppAstNode * type_id = nullptr;
    std::string type_id_text;
    bool dependent_class = false;
    bool resolving = false;
  };
  std::map<std::string, DeferredMemberAlias> deferred_member_aliases;
  bool is_polymorphic = false;
  bool rtti_required = false;
  bool is_final = false;
  bool is_initializer_list = false;
  cpp_decl::TypePtr initializer_list_element_type;
  bool is_lambda_closure = false;
  FunctionBinding * captureless_lambda_conversion_target = nullptr;
  bool source_is_unnamed_class = false;
  const CppAstNode * source_unnamed_class_node = nullptr;
  bool source_is_named_function_local_class = false;
  bool dependent_instantiation = false;
  bool template_instantiation_tracked = false;
  bool source_capture_header_instantiation_tracked = false;
  bool template_instantiation_log_emitted = false;
  bool template_instantiation_in_progress = false;
  bool full_member_collection_in_progress = false;
  bool reference_member_collection_in_progress = false;
  bool reference_members_collected = false;
  bool implicit_special_members_ensured = false;
  bool out_of_class_member_function_template_definitions_applied = false;
  bool out_of_class_member_function_definitions_applied = false;
  bool out_of_class_special_member_definitions_applied = false;
  bool out_of_class_static_member_definitions_applied = false;
  bool definition_output_in_progress = false;
  bool definition_output_emitted = false;
  bool has_late_required_static_member_output = false;
  bool late_required_static_member_output_queued = false;
  bool has_late_required_class_method_output = false;
  bool has_late_required_class_static_function_output = false;
  bool vtable_output_in_progress = false;
  bool vtable_output_emitted = false;
  bool has_own_vptr = false;
  bool is_explicit_specialization = false;
  bool suppress_implicit_instantiation_definition = false;
  std::size_t nonvirtual_size = 0;
  std::size_t nonvirtual_alignment = 1;
  bool complete = false;
  MemberAccess default_access = MA_PRIVATE;
  ClassTemplateDecl * source_template = nullptr;
  std::map<std::string, ClassTemplateDecl *>
      reference_reset_witness_class_templates;
  std::string instantiation_key;
  std::size_t instantiation_specialization_epoch = 0;
  std::vector<std::string> instantiation_arg_texts;
  std::vector<template_model::TemplateArgument> instantiation_arguments;
  std::vector<template_model::TemplateArgument> instantiation_binding_arguments;
  std::map<std::string, std::size_t> instantiation_binding_pack_sizes;
  bool has_instantiation_binding_arguments = false;
  std::vector<template_model::TemplateValueDependency> template_value_dependencies;
  std::string first_qualifier_use_location;
  bool reentrant_primary_selection = false;
  mutable SourceDeclAnchorCache declaration_anchor;
};

inline const std::string & class_output_qualified_name(const ClassInfo & info)
{
  return info.display_qualified_name.empty() ? info.qualified_name : info.display_qualified_name;
}

inline std::string function_output_name(const FunctionBinding & binding)
{
  if(binding.owner_class) {
    const std::string member_name =
        !binding.display_name.empty() ? binding.display_name :
        (binding.name.rfind("::") == std::string::npos ?
             binding.name :
             binding.name.substr(binding.name.rfind("::") + 2));
    return class_output_qualified_name(*binding.owner_class) + "::" + member_name;
  }
  return binding.name;
}

struct FunctionTemplateDecl
{
  Scope * declaring_scope = nullptr;
  Scope * pattern_scope = nullptr;
  std::string name;
  ClassInfo * lexical_access_class = nullptr;
  FunctionBinding * lexical_access_function = nullptr;
  MemberAccess access = MA_PUBLIC;
  bool is_constructor = false;
  bool is_inherited_constructor = false;
  bool is_destructor = false;
  bool is_conversion_operator = false;
  bool is_static_member = false;
  bool is_constexpr = false;
  bool is_explicit = false;
  bool is_const_method = false;
  bool is_volatile_method = false;
  RefQualifier ref_qualifier = RQ_NONE;
  bool is_deleted = false;
  bool decl_virtual = false;
  bool is_override = false;
  bool is_final = false;
  bool is_lambda_call_operator_template = false;
  bool is_member_function_template = false;
  bool exclude_from_explicit_instantiation = false;
  const CppAstNode * declaration_node = nullptr;
  const CppAstNode * definition_node = nullptr;
  const CppAstNode * inner = nullptr;
  const CppAstNode * definition_inner = nullptr;
  const CppAstNode * specifiers = nullptr;
  const CppAstNode * definition_specifiers = nullptr;
  const CppAstNode * declarator = nullptr;
  const CppAstNode * definition_declarator = nullptr;
  std::shared_ptr<CppAstNode> function_qualifier_storage;
  const CppAstNode * function_qualifier = nullptr;
  const CppAstNode * body = nullptr;
  const CppAstNode * ctor_initializer = nullptr;
  std::vector<template_model::TemplateParameterInfo> parameters;
  std::vector<template_model::TemplateParameterInfo> definition_owner_parameters;
  cpp_decl::TypePtr type_pattern;
  std::vector<std::pair<std::string, cpp_decl::TypePtr> > params_pattern;
  CppAstNode result_type_pattern;
  std::vector<const CppAstNode *> parameter_declarations_pattern;
  std::vector<std::string> parameter_aliases_pattern;
  std::vector<const CppAstNode *> default_arguments_pattern;
  std::vector<ClassInfo *> friend_access_classes;
  bool has_trailing_function_parameter_pack = false;
  bool trailing_function_parameter_pack_analyzed = false;
  std::string debug_decl_location;
  std::string debug_decl_location_details;
  std::string debug_scope_name;
  std::string debug_signature;
  std::map<std::string, FunctionBinding *> instantiations;
  mutable SourceDeclAnchorCache declaration_anchor;
};

inline std::vector<std::string> function_template_definition_abi_tags(
    const FunctionTemplateDecl & decl)
{
  std::vector<std::string> tags;
  append_function_declaration_abi_tags(tags, decl.definition_node);
  return tags;
}

inline bool function_binding_definition_suppresses_declaration_abi_tags(
    const FunctionBinding & binding)
{
  if(binding.definition_suppresses_declaration_abi_tags) {
    return true;
  }
  return binding.has_definition &&
         binding.definition_abi_tags.empty() &&
         binding.definition_node &&
         binding.definition_node != binding.declaration_node &&
         !binding.source_template &&
         binding.owner_class &&
         binding.owner_class->source_template;
}

inline std::vector<std::string> function_binding_abi_tags(const FunctionBinding & binding)
{
  if(function_binding_definition_suppresses_declaration_abi_tags(binding)) {
    return std::vector<std::string>();
  }
  if(!binding.definition_abi_tags.empty()) {
    return binding.definition_abi_tags;
  }
  return binding.declaration_abi_tags;
}

inline void initialize_function_parameter_aliases(FunctionBinding & binding)
{
  binding.parameter_aliases.resize(binding.params.size());
  for(std::size_t i = 0; i < binding.params.size(); ++i) {
    binding.parameter_aliases[i] = binding.params[i].first;
  }
}

inline void ensure_function_parameter_aliases(FunctionBinding & binding)
{
  if(binding.parameter_aliases.size() != binding.params.size()) {
    initialize_function_parameter_aliases(binding);
  }
}

inline void initialize_function_template_parameter_aliases(FunctionTemplateDecl & decl)
{
  decl.parameter_aliases_pattern.resize(decl.params_pattern.size());
  for(std::size_t i = 0; i < decl.params_pattern.size(); ++i) {
    decl.parameter_aliases_pattern[i] = decl.params_pattern[i].first;
  }
}

inline void ensure_function_template_parameter_aliases(FunctionTemplateDecl & decl)
{
  if(decl.parameter_aliases_pattern.size() != decl.params_pattern.size()) {
    initialize_function_template_parameter_aliases(decl);
  }
}

inline std::size_t function_binding_explicit_parameter_offset(const FunctionBinding & binding)
{
  return binding.is_method && !binding.params.empty() ? 1 : 0;
}

inline std::string function_parameter_binding_name(const FunctionBinding & binding,
                                                   std::size_t index)
{
  if(index >= binding.params.size()) {
    return std::string();
  }
  if(!binding.params[index].first.empty()) {
    return binding.params[index].first;
  }
  return std::string("__param") + std::to_string(index);
}

inline std::string function_parameter_alias_name(const FunctionBinding & binding,
                                                 std::size_t index)
{
  if(index < binding.parameter_aliases.size() &&
     !binding.parameter_aliases[index].empty()) {
    return binding.parameter_aliases[index];
  }
  if(index < binding.params.size()) {
    return binding.params[index].first;
  }
  return std::string();
}

inline std::string function_template_parameter_alias_name(const FunctionTemplateDecl & decl,
                                                          std::size_t index)
{
  if(index < decl.parameter_aliases_pattern.size() &&
     !decl.parameter_aliases_pattern[index].empty()) {
    return decl.parameter_aliases_pattern[index];
  }
  if(index < decl.params_pattern.size()) {
    return decl.params_pattern[index].first;
  }
  return std::string();
}

struct ClassTemplateSpecializationDecl
{
  ClassTemplateSpecializationDecl() {}
  ClassTemplateSpecializationDecl(Scope * declaring_scope,
                                  const CppAstNode * class_node)
    : declaring_scope(declaring_scope),
      class_node(class_node)
  {}

  Scope * declaring_scope = nullptr;
  const CppAstNode * class_node = nullptr;
};

struct AliasTemplateDecl
{
  struct StableSubstitutionArgumentKey
  {
    enum Kind
    {
      AK_TYPE_POINTER,
      AK_TYPE_FUNDAMENTAL,
      AK_VALUE,
      AK_TEMPLATE_DECL
    };

    Kind kind = AK_TYPE_POINTER;
    const void * pointer = nullptr;
    int type_code = 0;
    long long value = 0;
    bool value_type_is_fundamental = false;

    bool operator<(const StableSubstitutionArgumentKey & other) const
    {
      if(kind != other.kind) {
        return kind < other.kind;
      }
      if(type_code != other.type_code) {
        return type_code < other.type_code;
      }
      if(value != other.value) {
        return value < other.value;
      }
      if(value_type_is_fundamental != other.value_type_is_fundamental) {
        return value_type_is_fundamental < other.value_type_is_fundamental;
      }
      return std::less<const void *>()(pointer, other.pointer);
    }
  };

  struct StableSubstitutionKey
  {
    std::vector<StableSubstitutionArgumentKey> arguments;

    bool operator<(const StableSubstitutionKey & other) const
    {
      return arguments < other.arguments;
    }
  };

  struct StableSubstitutionFailure
  {
    std::string owner_type_key;
    std::string owner_type_display;
    std::string member_name;
  };

  struct StableAliasExpansionScopeKey
  {
    std::size_t instance_id = 0;
    std::size_t binding_fingerprint = 0;

    bool operator<(const StableAliasExpansionScopeKey & other) const
    {
      if(instance_id != other.instance_id) {
        return instance_id < other.instance_id;
      }
      return binding_fingerprint < other.binding_fingerprint;
    }
  };

  struct StableAliasExpansionArgumentKey
  {
    int kind = 0;
    bool dependent = false;
    bool source_defaulted = false;
    const void * type_pointer = nullptr;
    int type_code = 0;
    const void * template_decl = nullptr;
    const void * function_value = nullptr;
    std::string function_internal_symbol;
    const void * value_binding = nullptr;
    long long value = 0;
    std::string text;

    bool operator<(const StableAliasExpansionArgumentKey & other) const
    {
      if(kind != other.kind) {
        return kind < other.kind;
      }
      if(dependent != other.dependent) {
        return dependent < other.dependent;
      }
      if(source_defaulted != other.source_defaulted) {
        return source_defaulted < other.source_defaulted;
      }
      if(type_code != other.type_code) {
        return type_code < other.type_code;
      }
      if(value != other.value) {
        return value < other.value;
      }
      if(text != other.text) {
        return text < other.text;
      }
      if(type_pointer != other.type_pointer) {
        return std::less<const void *>()(type_pointer, other.type_pointer);
      }
      if(template_decl != other.template_decl) {
        return std::less<const void *>()(template_decl, other.template_decl);
      }
      if(function_internal_symbol != other.function_internal_symbol) {
        return function_internal_symbol < other.function_internal_symbol;
      }
      if(function_internal_symbol.empty() &&
         other.function_internal_symbol.empty() &&
         function_value != other.function_value) {
        return std::less<const void *>()(function_value, other.function_value);
      }
      return std::less<const void *>()(value_binding, other.value_binding);
    }
  };

  struct StableAliasExpansionKey
  {
    bool allow_dependent_expansion = false;
    StableAliasExpansionScopeKey match_scope;
    StableAliasExpansionScopeKey argument_scope;
    StableAliasExpansionScopeKey resolution_scope;
    std::vector<StableAliasExpansionArgumentKey> arguments;

    bool operator<(const StableAliasExpansionKey & other) const
    {
      if(allow_dependent_expansion != other.allow_dependent_expansion) {
        return allow_dependent_expansion < other.allow_dependent_expansion;
      }
      if(match_scope < other.match_scope) {
        return true;
      }
      if(other.match_scope < match_scope) {
        return false;
      }
      if(argument_scope < other.argument_scope) {
        return true;
      }
      if(other.argument_scope < argument_scope) {
        return false;
      }
      if(resolution_scope < other.resolution_scope) {
        return true;
      }
      if(other.resolution_scope < resolution_scope) {
        return false;
      }
      return arguments < other.arguments;
    }
  };

  struct StableAliasExpansionValue
  {
    enum Kind
    {
      EK_SUCCESS,
      EK_DEPENDENT_DEFER
    };

    Kind kind = EK_SUCCESS;
    std::string expanded_text;
    cpp_decl::TypePtr expanded_type;
  };

  Scope * declaring_scope = nullptr;
  Scope * pattern_scope = nullptr;
  std::string name;
  const CppAstNode * type_id = nullptr;
  cpp_decl::TypePtr resolved_type_pattern;
  std::vector<template_model::TemplateParameterInfo> parameters;
  std::map<std::string, cpp_decl::TypePtr> instantiations;
  std::map<std::string, cpp_decl::TypePtr> reference_instantiations;
  mutable bool dependent_qualified_member_scope_sensitive_cached = false;
  mutable bool dependent_qualified_member_scope_sensitive = false;
  mutable std::map<StableSubstitutionKey, StableSubstitutionFailure>
      stable_substitution_failures;
  mutable std::map<StableAliasExpansionKey, StableAliasExpansionValue>
      stable_alias_expansions;
  mutable SourceDeclAnchorCache declaration_anchor;
};

struct DeductionGuideDecl
{
  Scope * declaring_scope = nullptr;
  Scope * pattern_scope = nullptr;
  std::string name;
  const CppAstNode * node = nullptr;
  const CppAstNode * declarator = nullptr;
  std::vector<template_model::TemplateParameterInfo> parameters;
  std::vector<std::string> return_arg_texts;
};

struct OutOfClassStaticMemberDecl
{
  Scope * declaring_scope = nullptr;
  Scope * pattern_scope = nullptr;
  cpp_decl::QualifiedName qualified_name_syntax;
  const CppAstNode * node = nullptr;
  const CppAstNode * specifiers = nullptr;
  const CppAstNode * declarator = nullptr;
  const CppAstNode * initializer = nullptr;
  std::vector<template_model::TemplateParameterInfo> parameters;
  bool has_storage_definition = false;
};

struct OutOfClassMemberClassDecl
{
  Scope * declaring_scope = nullptr;
  Scope * pattern_scope = nullptr;
  const CppAstNode * class_node = nullptr;
  std::vector<template_model::TemplateParameterInfo> parameters;
};

struct OutOfClassMemberFunctionDecl
{
  Scope * declaring_scope = nullptr;
  Scope * pattern_scope = nullptr;
  std::string qualified_name;
  cpp_decl::QualifiedName qualified_name_syntax;
  const CppAstNode * owner_output_node = nullptr;
  const CppAstNode * specifiers = nullptr;
  const CppAstNode * declarator = nullptr;
  const CppAstNode * body = nullptr;
  const CppAstNode * ctor_initializer = nullptr;
  cpp_decl::TypePtr declared_type_pattern;
  std::vector<std::pair<std::string, cpp_decl::TypePtr> > params;
  bool is_const_method = false;
  bool is_volatile_method = false;
  semantic_model::RefQualifier ref_qualifier = semantic_model::RQ_NONE;
  bool exclude_from_explicit_instantiation = false;
  std::vector<template_model::TemplateParameterInfo> parameters;
};

struct OutOfClassMemberFunctionTemplateDefinition
{
  FunctionTemplateDecl * declaration = nullptr;
  const CppAstNode * definition_node = nullptr;
  const CppAstNode * definition_specifiers = nullptr;
  const CppAstNode * definition_declarator = nullptr;
  const CppAstNode * body = nullptr;
  const CppAstNode * ctor_initializer = nullptr;
  std::vector<std::string> parameter_aliases_pattern;
  std::vector<template_model::TemplateParameterInfo> owner_parameters;
};

struct PartialClassTemplateSpecializationDecl
{
  Scope * declaring_scope = nullptr;
  Scope * pattern_scope = nullptr;
  const CppAstNode * class_node = nullptr;
  std::vector<template_model::TemplateParameterInfo> parameters;
  std::vector<std::string> arg_texts;
  std::vector<cpp_decl::TemplateArgumentSyntax> arg_syntaxes;
  mutable std::vector<cpp_decl::TypePtr> placeholder_arg_type_patterns;
  mutable std::vector<unsigned char>
      concrete_expression_recheck_pattern_states;
  std::map<std::string, OutOfClassStaticMemberDecl> static_member_definitions;
  std::map<std::string, OutOfClassStaticMemberDecl>
      witness_static_member_definitions;
  std::map<std::string, std::vector<OutOfClassMemberFunctionDecl> >
      member_function_definitions;
  std::map<std::string, std::vector<OutOfClassMemberFunctionTemplateDefinition> >
      member_function_template_definitions;
};

struct ClassTemplateDecl
{
  struct SpecializationSelectionCacheEntry
  {
    std::size_t specialization_epoch = 0;
    const CppAstNode * class_node = nullptr;
    Scope * binding_scope = nullptr;
    const std::vector<template_model::TemplateParameterInfo> * parameters = nullptr;
    std::vector<template_model::TemplateArgument> arguments;
    std::map<std::string, std::size_t> pack_sizes;
    std::string selection_key;
    int kind = 0;
  };

  Scope * declaring_scope = nullptr;
  Scope * pattern_scope = nullptr;
  std::string name;
  const CppAstNode * class_node = nullptr;
  std::vector<template_model::TemplateParameterInfo> parameters;
  std::map<std::string, ClassInfo *> instantiations;
  std::map<std::string, ClassInfo *> reference_instantiations;
  std::map<std::string, ClassInfo *> fast_reference_cache;
  std::size_t specialization_epoch = 0;
  mutable std::map<std::string, SpecializationSelectionCacheEntry>
      specialization_selection_cache;
  std::set<std::string> suppress_implicit_instantiation_definitions;
  std::set<std::string> explicit_static_member_specializations;
  std::set<std::pair<std::string, std::string> >
      explicit_static_member_specialization_keys;
  std::set<std::pair<std::string, std::string> >
      explicit_member_function_specialization_keys;
  std::map<std::string, ClassTemplateSpecializationDecl> explicit_specializations;
  std::vector<PartialClassTemplateSpecializationDecl> partial_specializations;
  std::vector<DeductionGuideDecl> deduction_guides;
  std::map<std::string, OutOfClassStaticMemberDecl> static_member_definitions;
  std::map<std::string, OutOfClassStaticMemberDecl>
      witness_static_member_definitions;
  std::map<std::string, OutOfClassMemberClassDecl> member_class_definitions;
  std::map<std::string, std::vector<PartialClassTemplateSpecializationDecl> >
      member_class_template_partial_specializations;
  std::map<std::string, std::vector<OutOfClassMemberFunctionDecl> > member_function_definitions;
  std::map<std::string, std::vector<OutOfClassMemberFunctionTemplateDefinition> >
      member_function_template_definitions;
  bool comes_from_standard_include_path = false;
  mutable SourceDeclAnchorCache declaration_anchor;
};

struct VariableTemplateSpecializationDecl
{
  Scope * declaring_scope = nullptr;
  Scope * pattern_scope = nullptr;
  const CppAstNode * specifiers = nullptr;
  const CppAstNode * declarator = nullptr;
  const CppAstNode * initializer = nullptr;
  std::vector<template_model::TemplateParameterInfo> parameters;
  std::vector<std::string> arg_texts;
  std::vector<cpp_decl::TemplateArgumentSyntax> arg_syntaxes;
  mutable std::vector<cpp_decl::TypePtr> placeholder_arg_type_patterns;
  mutable std::vector<unsigned char>
      concrete_expression_recheck_pattern_states;
};

struct VariableTemplateDecl
{
  Scope * declaring_scope = nullptr;
  Scope * pattern_scope = nullptr;
  std::string name;
  const CppAstNode * specifiers = nullptr;
  const CppAstNode * declarator = nullptr;
  const CppAstNode * initializer = nullptr;
  std::vector<template_model::TemplateParameterInfo> parameters;
  cpp_decl::TypePtr type_pattern;
  std::map<std::string, ValueBinding> instantiations;
  std::map<std::string, VariableTemplateSpecializationDecl> explicit_specializations;
  std::vector<VariableTemplateSpecializationDecl> partial_specializations;
  bool comes_from_standard_include_path = false;
  mutable SourceDeclAnchorCache declaration_anchor;
};

std::string describe_scope_bindings(const Scope & scope);

}  // namespace semantic_model
