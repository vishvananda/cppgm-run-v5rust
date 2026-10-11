#include "semantic_model.h"

#include <atomic>
#include <sstream>
#include <utility>

#include "semantic_lookup.h"
#include "semantic_utils.h"

namespace semantic_model {

std::size_t next_scope_instance_id()
{
  static std::atomic<std::size_t> next_id(1);
  return next_id.fetch_add(1, std::memory_order_relaxed);
}

Scope::Scope(const Scope & other)
  : parent(other.parent),
    name(other.name),
    namespace_scope(other.namespace_scope),
    inline_namespace(other.inline_namespace),
    class_info(other.class_info),
    function(other.function),
    named_types(other.named_types),
    named_type_packs(other.named_type_packs),
    named_value_packs(other.named_value_packs),
    named_pack_sizes(other.named_pack_sizes),
    template_bound_type_names(other.template_bound_type_names),
    template_bound_type_pack_names(other.template_bound_type_pack_names),
    template_bound_value_names(other.template_bound_value_names),
    template_bound_template_names(other.template_bound_template_names),
    values(other.values),
    namespace_bindings(other.namespace_bindings),
    function_sets(other.function_sets),
    class_templates(other.class_templates),
    function_templates(other.function_templates),
    collected_template_declarations(other.collected_template_declarations),
    alias_templates(other.alias_templates),
    variable_templates(other.variable_templates),
    using_directives(other.using_directives),
    instance_id(next_scope_instance_id()),
    binding_fingerprint_epoch(other.binding_fingerprint_epoch)
{}

Scope::Scope(Scope && other)
  : parent(other.parent),
    name(std::move(other.name)),
    namespace_scope(other.namespace_scope),
    inline_namespace(other.inline_namespace),
    class_info(other.class_info),
    function(other.function),
    named_types(std::move(other.named_types)),
    named_type_packs(std::move(other.named_type_packs)),
    named_value_packs(std::move(other.named_value_packs)),
    named_pack_sizes(std::move(other.named_pack_sizes)),
    template_bound_type_names(std::move(other.template_bound_type_names)),
    template_bound_type_pack_names(std::move(other.template_bound_type_pack_names)),
    template_bound_value_names(std::move(other.template_bound_value_names)),
    template_bound_template_names(std::move(other.template_bound_template_names)),
    values(std::move(other.values)),
    namespace_bindings(std::move(other.namespace_bindings)),
    function_sets(std::move(other.function_sets)),
    class_templates(std::move(other.class_templates)),
    function_templates(std::move(other.function_templates)),
    collected_template_declarations(std::move(other.collected_template_declarations)),
    alias_templates(std::move(other.alias_templates)),
    variable_templates(std::move(other.variable_templates)),
    using_directives(std::move(other.using_directives)),
    namespace_children(std::move(other.namespace_children)),
    instance_id(other.instance_id),
    binding_fingerprint_epoch(other.binding_fingerprint_epoch)
{}

Scope & Scope::operator=(Scope && other)
{
  if(this == &other) {
    return *this;
  }

  parent = other.parent;
  name = std::move(other.name);
  namespace_scope = other.namespace_scope;
  inline_namespace = other.inline_namespace;
  class_info = other.class_info;
  function = other.function;
  named_types = std::move(other.named_types);
  named_type_packs = std::move(other.named_type_packs);
  named_value_packs = std::move(other.named_value_packs);
  named_pack_sizes = std::move(other.named_pack_sizes);
  template_bound_type_names = std::move(other.template_bound_type_names);
  template_bound_type_pack_names = std::move(other.template_bound_type_pack_names);
  template_bound_value_names = std::move(other.template_bound_value_names);
  template_bound_template_names = std::move(other.template_bound_template_names);
  values = std::move(other.values);
  namespace_bindings = std::move(other.namespace_bindings);
  function_sets = std::move(other.function_sets);
  class_templates = std::move(other.class_templates);
  function_templates = std::move(other.function_templates);
  collected_template_declarations = std::move(other.collected_template_declarations);
  alias_templates = std::move(other.alias_templates);
  variable_templates = std::move(other.variable_templates);
  using_directives = std::move(other.using_directives);
  namespace_children = std::move(other.namespace_children);
  binding_fingerprint_epoch = other.binding_fingerprint_epoch;
  instance_id = other.instance_id;
  cached_binding_scope_fingerprint_valid = false;
  cached_binding_scope_fingerprint = 0;
  cached_binding_scope_fingerprint_epoch = 0;
  cached_binding_scope_parent_fingerprint = 0;
  cached_instance_scope_fingerprint_valid = false;
  cached_instance_scope_fingerprint = 0;
  return *this;
}

std::string describe_scope_bindings(const Scope & scope)
{
  std::ostringstream out;
  bool wrote_any = false;

  for(const Scope * current = &scope; current; current = current->parent) {
    bool scope_has_any = false;
    std::ostringstream scope_out;

    for(const auto & named : current->named_types) {
      if(named.first.empty()) {
        continue;
      }
      if(!scope_has_any) {
        scope_out << current->name;
        scope_out << "{types:";
        scope_has_any = true;
      } else {
        scope_out << ",";
      }
      scope_out << named.first << "=" << cpp_decl::describe_type(named.second);
    }

    for(const auto & value : current->values) {
      if(value.first.empty()) {
        continue;
      }
      if(!scope_has_any) {
        scope_out << current->name;
        scope_out << "{values:";
        scope_has_any = true;
      } else {
        scope_out << ",";
      }
      scope_out << value.first << "=" << cpp_decl::describe_type(value.second.type);
      if(value.second.has_constant_value) {
        scope_out << ":" << value.second.constant_value;
      }
    }

    for(const auto & pack : current->named_type_packs) {
      if(pack.first.empty()) {
        continue;
      }
      if(!scope_has_any) {
        scope_out << current->name;
        scope_out << "{type_packs:";
        scope_has_any = true;
      } else {
        scope_out << ",";
      }
      scope_out << pack.first << "=[";
      for(std::size_t i = 0; i < pack.second.size(); ++i) {
        if(i != 0) {
          scope_out << ", ";
        }
        scope_out << cpp_decl::describe_type(pack.second[i]);
      }
      scope_out << "]";
    }

    for(const auto & pack : current->named_value_packs) {
      if(pack.first.empty()) {
        continue;
      }
      if(!scope_has_any) {
        scope_out << current->name;
        scope_out << "{value_packs:";
        scope_has_any = true;
      } else {
        scope_out << ",";
      }
      scope_out << pack.first << "=[";
      for(std::size_t i = 0; i < pack.second.size(); ++i) {
        if(i != 0) {
          scope_out << ", ";
        }
        scope_out << pack.second[i].name << ":" << cpp_decl::describe_type(pack.second[i].type);
      }
      scope_out << "]";
    }

    if(scope_has_any) {
      scope_out << "}";
      if(wrote_any) {
        out << " ";
      }
      out << scope_out.str();
      wrote_any = true;
    }
  }

  return out.str();
}

std::string function_binding_display_name_for_symbol(const FunctionBinding & binding)
{
  const std::string display_name =
      binding.display_name.empty() ?
          semantic_utils::unqualified_member_name(
              semantic_lookup::canonical_function_lookup_name(binding.name)) :
          binding.display_name;
  const std::size_t display_split = display_name.rfind("::");
  return display_split == std::string::npos ? display_name : display_name.substr(display_split + 2);
}

std::string function_binding_qualified_name_for_symbol(const FunctionBinding & binding)
{
  const std::string simple_name = function_binding_display_name_for_symbol(binding);
  if(binding.owner_class && binding.owner_class->member_scope) {
    return semantic_lookup::scope_symbol_qualified_name(*binding.owner_class->member_scope,
                                                        simple_name);
  }
  if(binding.declaration_scope && binding.name.find("::") == std::string::npos) {
    return semantic_lookup::scope_symbol_qualified_name(*binding.declaration_scope, simple_name);
  }
  return binding.name;
}

}  // namespace semantic_model
