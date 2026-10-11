#pragma once

#include <cstddef>
#include <cstdint>
#include <iosfwd>
#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "cpp_decl_model.h"
#include "cppast_ast.h"
#include "posttokenizer.h"
#include "symbol_linkage.h"
#include "text_intern.h"

enum CallValueCategory
{
  CVC_NONE,
  CVC_LVALUE,
  CVC_PRVALUE,
  CVC_XVALUE
};

#define CALL_SEM_KIND_LIST(X) \
  X(invalid, "<invalid>") \
  X(assignment_expression, "assignment-expression") \
  X(asm_clause, "asm-clause") \
  X(asm_operand, "asm-operand") \
  X(asm_statement, "asm-statement") \
  X(binary_expression, "binary-expression") \
  X(break_statement, "break-statement") \
  X(braced_init_list, "braced-init-list") \
  X(call_expression, "call-expression") \
  X(callee, "callee") \
  X(closure_capture, "closure-capture") \
  X(closure_object, "closure-object") \
  X(compound_statement, "compound-statement") \
  X(condition, "condition") \
  X(condition_declaration, "condition-declaration") \
  X(conditional_expression, "conditional-expression") \
  X(constructor_action, "constructor-action") \
  X(continue_statement, "continue-statement") \
  X(catch_handler, "catch-handler") \
  X(case_statement, "case-statement") \
  X(cast_expression, "cast-expression") \
  X(do_statement, "do-statement") \
  X(default_statement, "default-statement") \
  X(dynamic_cast_expression, "dynamic-cast-expression") \
  X(destructor_action, "destructor-action") \
  X(else_node, "else") \
  X(expression_statement, "expression-statement") \
  X(for_init_statement, "for-init-statement") \
  X(for_statement, "for-statement") \
  X(function_declaration, "function-declaration") \
  X(function_definition, "function-definition") \
  X(goto_statement, "goto-statement") \
  X(id_expression, "id-expression") \
  X(if_statement, "if-statement") \
  X(iteration, "iteration") \
  X(initializer_list_object, "initializer-list-object") \
  X(labeled_statement, "labeled-statement") \
  X(literal, "literal") \
  X(member_expression, "member-expression") \
  X(namespace_definition, "namespace-definition") \
  X(new_expression, "new-expression") \
  X(parameter, "parameter") \
  X(postfix_expression, "postfix-expression") \
  X(range_for_statement, "range-for-statement") \
  X(rtti_candidate, "rtti-candidate") \
  X(rtti_base, "rtti-base") \
  X(return_statement, "return-statement") \
  X(rtti_definition, "rtti-definition") \
  X(simple_declaration, "simple-declaration") \
  X(sizeof_expression, "sizeof-expression") \
  X(statement_expression, "statement-expression") \
  X(subscript_expression, "subscript-expression") \
  X(switch_statement, "switch-statement") \
  X(then_node, "then") \
  X(throw_statement, "throw-statement") \
  X(try_statement, "try-statement") \
  X(typeid_expression, "typeid-expression") \
  X(translation_unit, "translation-unit") \
  X(type_alias, "type-alias") \
  X(unary_expression, "unary-expression") \
  X(vtt_definition, "vtt-definition") \
  X(vtt_entry, "vtt-entry") \
  X(vptr_action, "vptr-action") \
  X(vtable_definition, "vtable-definition") \
  X(vtable_entry, "vtable-entry") \
  X(variable, "variable") \
  X(while_statement, "while-statement")

enum class CallSemKind
{
#define CALL_SEM_KIND_ENUM(name, text) name,
  CALL_SEM_KIND_LIST(CALL_SEM_KIND_ENUM)
#undef CALL_SEM_KIND_ENUM
};

const char * callsem_kind_text(CallSemKind kind);
bool callsem_construction_census_enabled();
void callsem_note_constructed_node(CallSemKind kind,
                                  const std::string & text,
                                  std::size_t explicit_child_count);
void dump_callsem_construction_census(std::ostream & out);

class ScopedCallSemConstructionPath
{
public:
  explicit ScopedCallSemConstructionPath(const char * path);
  ~ScopedCallSemConstructionPath();

private:
  const char * saved_;
  bool active_;
};

typedef std::vector<std::pair<std::string, unsigned long long> >
    CallSemVirtualBaseLayout;

struct CallSemNode;

class CallSemText
{
public:
  CallSemText() : atom_(nullptr) {}
  CallSemText(const std::string & value) : atom_(nullptr) { assign(value); }
  CallSemText(const char * value) : atom_(nullptr)
  {
    assign(value == nullptr ? std::string() : std::string(value));
  }

  CallSemText & operator=(const std::string & value)
  {
    assign(value);
    return *this;
  }

  CallSemText & operator=(const char * value)
  {
    assign(value == nullptr ? std::string() : std::string(value));
    return *this;
  }

  operator const std::string &() const { return str(); }

  const std::string & str() const
  {
    return atom_ == nullptr ? empty_string() : *atom_;
  }

  text_intern::Atom atom() const { return atom_; }
  bool empty() const { return atom_ == nullptr || atom_->empty(); }
  std::size_t size() const { return str().size(); }
  std::size_t capacity() const { return str().capacity(); }
  const char * data() const { return str().data(); }
  const char * c_str() const { return str().c_str(); }
  char operator[](std::size_t index) const { return str()[index]; }
  std::string::size_type find(char value,
                              std::string::size_type pos = 0) const
  {
    return str().find(value, pos);
  }
  std::string::size_type find(const std::string & value,
                              std::string::size_type pos = 0) const
  {
    return str().find(value, pos);
  }
  std::string::size_type find(const char * value,
                              std::string::size_type pos = 0) const
  {
    return str().find(value, pos);
  }
  std::string substr(
      std::string::size_type pos = 0,
      std::string::size_type len = std::string::npos) const
  {
    return str().substr(pos, len);
  }
  int compare(const std::string & value) const { return str().compare(value); }
  void clear() { atom_ = nullptr; }

private:
  void assign(const std::string & value)
  {
    atom_ = value.empty() ? nullptr : text_intern::intern(value);
  }

  static const std::string & empty_string()
  {
    static const std::string empty;
    return empty;
  }

  text_intern::Atom atom_;
};

inline bool operator==(const CallSemText & lhs, const CallSemText & rhs)
{
  return lhs.atom() == rhs.atom() || lhs.str() == rhs.str();
}

inline bool operator==(const CallSemText & lhs, const std::string & rhs)
{
  return lhs.str() == rhs;
}

inline bool operator==(const std::string & lhs, const CallSemText & rhs)
{
  return lhs == rhs.str();
}

inline bool operator==(const CallSemText & lhs, const char * rhs)
{
  return lhs.str() == (rhs == nullptr ? "" : rhs);
}

inline bool operator==(const char * lhs, const CallSemText & rhs)
{
  return (lhs == nullptr ? "" : lhs) == rhs.str();
}

inline bool operator!=(const CallSemText & lhs, const CallSemText & rhs)
{
  return !(lhs == rhs);
}

inline bool operator!=(const CallSemText & lhs, const std::string & rhs)
{
  return !(lhs == rhs);
}

inline bool operator!=(const std::string & lhs, const CallSemText & rhs)
{
  return !(lhs == rhs);
}

inline bool operator!=(const CallSemText & lhs, const char * rhs)
{
  return !(lhs == rhs);
}

inline bool operator!=(const char * lhs, const CallSemText & rhs)
{
  return !(lhs == rhs);
}

inline bool operator<(const CallSemText & lhs, const CallSemText & rhs)
{
  return lhs.str() < rhs.str();
}

inline std::string operator+(const CallSemText & lhs, const std::string & rhs)
{
  return lhs.str() + rhs;
}

inline std::string operator+(const std::string & lhs, const CallSemText & rhs)
{
  return lhs + rhs.str();
}

inline std::string operator+(const CallSemText & lhs, const char * rhs)
{
  return lhs.str() + (rhs == nullptr ? "" : rhs);
}

inline std::string operator+(const char * lhs, const CallSemText & rhs)
{
  return std::string(lhs == nullptr ? "" : lhs) + rhs.str();
}

std::ostream & operator<<(std::ostream & out, const CallSemText & text);

struct CallSemRareStrings
{
  std::string vtt_symbol;
  std::string vtt_object_symbol;
  std::string runtime_bridge_symbol;
  std::string local_static_guard_symbol;
};

struct CallSemRarePayload
{
  cpp_decl::TypePtr vtt_owner_type;
  cpp_decl::TypePtr materialization_source_type;
  cpp_decl::TypePtr conversion_source_type;
  cpp_decl::TypePtr initializer_list_element_type;
  cpp_decl::TypePtr typeid_operand_type;
  CallSemVirtualBaseLayout virtual_base_layout;
  unsigned long long uint_value = 0;
  long long int_value = 0;
  symbol_linkage::SpecialMemberEntryPointKind special_member_entry_point_kind =
      symbol_linkage::SMEK_COMPLETE;
  long long result_adjust = 0;
  long long virtual_dispatch_view_offset = 0;
  unsigned long long bit_field_width = 0;
  unsigned long long bit_field_offset = 0;
  unsigned long long bit_field_storage_size = 0;
  unsigned long long trivial_storage_copy_prefix_bytes = 0;
  unsigned long long vtt_slice_offset = 0;
  unsigned long long vtt_entry_index = 0;
  symbol_linkage::FunctionRefQualifier function_ref_qualifier =
      symbol_linkage::FRQ_NONE;
  std::vector<std::string> abi_tags;
  std::vector<std::string> object_aliases;
  std::shared_ptr<CallSemNode> lowered_condition_test;
};

struct CallSemNodeExtra
{
  CallSemText resolved_name;
  std::shared_ptr<cpp_decl::QualifiedName> qualified_name_syntax;
  std::shared_ptr<symbol_linkage::SymbolIdentity> symbol;
  std::shared_ptr<CallSemRareStrings> rare_strings;
  std::shared_ptr<CallSemRarePayload> rare_payload;
};

struct CallSemNode
{
  CallSemNode()
    : source_file_index(0),
      source_line(0),
      source_column(0),
      token_type(static_cast<ETokenType>(0)),
      implicit_return_move_eligible(false),
      has_uint_value(false),
      has_int_value(false),
      has_result_adjust(false),
      is_bit_field(false),
      is_reference_storage(false),
      is_reference_storage_target(false),
      is_base_subobject(false),
      is_public_access(false),
      is_virtual_base_subobject(false),
      has_token(false),
      is_virtual_dispatch(false),
      is_virtual_member_function(false),
      is_constructor(false),
      is_destructor(false),
      is_conversion_operator(false),
      is_const_method(false),
      is_volatile_method(false),
      has_function_ref_qualifier(false),
      has_virtual_dispatch_view_offset(false),
      is_primary_vtable(false),
      uses_extended_vtable_layout(false),
      is_extern_declaration(false),
      is_static_storage(false),
      is_c_linkage(false),
      is_thread_local(false),
      is_inline_namespace(false),
      is_declval_callee(false),
      is_explicit_nothrow(false),
      is_semantically_nothrow(false),
      is_explicit_instantiation_definition(false),
      has_dynamic_exception_spec(false),
      needs_noexcept_terminate(false),
      trivial_lifecycle(false),
      value_initializes_result(false),
      object_trivial_lifecycle(false),
      has_trivial_storage_copy_prefix(false),
      has_special_member_entry_point_kind(false),
      uses_vtt_parameter(false),
      has_vtt_slice_offset(false),
      has_vtt_entry_index(false)
  {}
  CallSemNode(CallSemKind kind,
              const std::string & text,
              const std::vector<CallSemNode> & children)
    : CallSemNode()
  {
    this->kind = kind;
    this->text = text;
    this->children = children;
    callsem_note_constructed_node(kind, text, children.size());
  }

  CallSemKind kind = CallSemKind::invalid;
  CallValueCategory value_category = CVC_NONE;

  CallSemText text;
  std::vector<CallSemNode> children;
  cpp_decl::TypePtr semantic_type;
  std::shared_ptr<CallSemNodeExtra> extra;
  std::uint64_t source_file_index : 16;
  std::uint64_t source_line : 24;
  std::uint64_t source_column : 24;
  ETokenType token_type : 8;

  std::uint64_t implicit_return_move_eligible : 1;
  std::uint64_t has_uint_value : 1;
  std::uint64_t has_int_value : 1;
  std::uint64_t has_result_adjust : 1;
  std::uint64_t is_bit_field : 1;
  std::uint64_t is_reference_storage : 1;
  std::uint64_t is_reference_storage_target : 1;
  std::uint64_t is_base_subobject : 1;
  std::uint64_t is_public_access : 1;
  std::uint64_t is_virtual_base_subobject : 1;
  std::uint64_t has_token : 1;
  std::uint64_t is_virtual_dispatch : 1;
  std::uint64_t is_virtual_member_function : 1;
  std::uint64_t is_constructor : 1;
  std::uint64_t is_destructor : 1;
  std::uint64_t is_conversion_operator : 1;
  std::uint64_t is_const_method : 1;
  std::uint64_t is_volatile_method : 1;
  std::uint64_t has_function_ref_qualifier : 1;
  std::uint64_t has_virtual_dispatch_view_offset : 1;
  std::uint64_t is_primary_vtable : 1;
  std::uint64_t uses_extended_vtable_layout : 1;
  std::uint64_t is_extern_declaration : 1;
  std::uint64_t is_static_storage : 1;
  std::uint64_t is_c_linkage : 1;
  std::uint64_t is_thread_local : 1;
  std::uint64_t is_inline_namespace : 1;
  std::uint64_t is_declval_callee : 1;
  std::uint64_t is_explicit_nothrow : 1;
  std::uint64_t is_semantically_nothrow : 1;
  std::uint64_t is_explicit_instantiation_definition : 1;
  std::uint64_t has_dynamic_exception_spec : 1;
  std::uint64_t needs_noexcept_terminate : 1;
  std::uint64_t trivial_lifecycle : 1;
  std::uint64_t value_initializes_result : 1;
  std::uint64_t object_trivial_lifecycle : 1;
  std::uint64_t has_trivial_storage_copy_prefix : 1;
  std::uint64_t has_special_member_entry_point_kind : 1;
  std::uint64_t uses_vtt_parameter : 1;
  std::uint64_t has_vtt_slice_offset : 1;
  std::uint64_t has_vtt_entry_index : 1;

  bool has_source_location() const;
};

const std::string & callsem_empty_extra_string();
std::uint32_t callsem_intern_source_file_index(const std::string & value);
const std::string & callsem_source_file_by_index(std::uint32_t index);
std::uint32_t callsem_checked_source_line(unsigned long long value);
std::uint32_t callsem_checked_source_column(unsigned long long value);
const cpp_decl::TypePtr & callsem_empty_extra_type();
const symbol_linkage::SymbolIdentity & callsem_empty_symbol();
std::shared_ptr<symbol_linkage::SymbolIdentity>
callsem_intern_symbol(const symbol_linkage::SymbolIdentity & symbol);
const std::shared_ptr<cpp_decl::QualifiedName> &
callsem_empty_qualified_name_syntax();
const CallSemVirtualBaseLayout & callsem_empty_virtual_base_layout();
const std::shared_ptr<CallSemNode> & callsem_empty_lowered_condition_test();

inline bool callsem_symbol_is_empty(const symbol_linkage::SymbolIdentity & symbol)
{
  return symbol.internal_symbol.empty() &&
         symbol.object_symbol.empty() &&
         symbol.thread_local_wrapper_object_symbol.empty() &&
         !symbol.keep_internal_alias &&
         !symbol.prefer_local_object_binding &&
         symbol.linkage == symbol_linkage::SL_EXTERNAL;
}

inline CallSemNodeExtra & ensure_callsem_extra(CallSemNode & node)
{
  if(!node.extra) {
    node.extra.reset(new CallSemNodeExtra());
  } else if(node.extra.use_count() != 1) {
    node.extra.reset(new CallSemNodeExtra(*node.extra));
  }
  return *node.extra;
}

inline CallSemRareStrings & ensure_callsem_rare_strings(CallSemNode & node)
{
  CallSemNodeExtra & extra = ensure_callsem_extra(node);
  if(!extra.rare_strings) {
    extra.rare_strings.reset(new CallSemRareStrings());
  } else if(extra.rare_strings.use_count() != 1) {
    extra.rare_strings.reset(new CallSemRareStrings(*extra.rare_strings));
  }
  return *extra.rare_strings;
}

inline CallSemRarePayload & ensure_callsem_rare_payload(CallSemNode & node)
{
  CallSemNodeExtra & extra = ensure_callsem_extra(node);
  if(!extra.rare_payload) {
    extra.rare_payload.reset(new CallSemRarePayload());
  } else if(extra.rare_payload.use_count() != 1) {
    extra.rare_payload.reset(new CallSemRarePayload(*extra.rare_payload));
  }
  return *extra.rare_payload;
}

inline const CallSemRarePayload * callsem_rare_payload(const CallSemNode & node)
{
  return node.extra ? node.extra->rare_payload.get() : nullptr;
}

inline const symbol_linkage::SymbolIdentity & callsem_symbol(const CallSemNode & node)
{
  return node.extra && node.extra->symbol ? *node.extra->symbol : callsem_empty_symbol();
}

inline symbol_linkage::SymbolIdentity & mutable_callsem_symbol(CallSemNode & node)
{
  CallSemNodeExtra & extra = ensure_callsem_extra(node);
  if(!extra.symbol) {
    extra.symbol.reset(new symbol_linkage::SymbolIdentity());
  } else if(extra.symbol.use_count() != 1) {
    extra.symbol.reset(new symbol_linkage::SymbolIdentity(*extra.symbol));
  }
  return *extra.symbol;
}

inline void set_callsem_symbol(CallSemNode & node,
                               const symbol_linkage::SymbolIdentity & symbol)
{
  if(callsem_symbol_is_empty(symbol)) {
    if(node.extra) {
      node.extra->symbol.reset();
    }
    return;
  }
  ensure_callsem_extra(node).symbol = callsem_intern_symbol(symbol);
}

inline unsigned long long callsem_uint_value(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->uint_value : 0;
}

inline void set_callsem_uint_value(CallSemNode & node, unsigned long long value)
{
  node.has_uint_value = true;
  ensure_callsem_rare_payload(node).uint_value = value;
}

inline void clear_callsem_uint_value(CallSemNode & node)
{
  node.has_uint_value = false;
  if(CallSemRarePayload * payload = node.extra ? node.extra->rare_payload.get() : nullptr) {
    payload->uint_value = 0;
  }
}

inline long long callsem_int_value(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->int_value : 0;
}

inline void set_callsem_int_value(CallSemNode & node, long long value)
{
  node.has_int_value = true;
  ensure_callsem_rare_payload(node).int_value = value;
}

inline void clear_callsem_int_value(CallSemNode & node)
{
  node.has_int_value = false;
  if(CallSemRarePayload * payload = node.extra ? node.extra->rare_payload.get() : nullptr) {
    payload->int_value = 0;
  }
}

inline const std::string & callsem_source_file(const CallSemNode & node)
{
  return node.source_file_index != 0 ?
      callsem_source_file_by_index(node.source_file_index) :
      callsem_empty_extra_string();
}

inline void set_callsem_source_file(CallSemNode & node, const std::string & value)
{
  node.source_file_index = callsem_intern_source_file_index(value);
}

inline unsigned long long callsem_source_line(const CallSemNode & node)
{
  return node.source_line;
}

inline void set_callsem_source_line(CallSemNode & node, unsigned long long value)
{
  node.source_line = callsem_checked_source_line(value);
}

inline unsigned long long callsem_source_column(const CallSemNode & node)
{
  return node.source_column;
}

inline void set_callsem_source_column(CallSemNode & node, unsigned long long value)
{
  node.source_column = callsem_checked_source_column(value);
}

inline const std::string & callsem_resolved_name(const CallSemNode & node)
{
  return node.extra ? node.extra->resolved_name.str() : callsem_empty_extra_string();
}

inline void set_callsem_resolved_name(CallSemNode & node, const std::string & value)
{
  if(value.empty() && !node.extra) {
    return;
  }
  ensure_callsem_extra(node).resolved_name = value;
}

inline void clear_callsem_resolved_name(CallSemNode & node)
{
  if(node.extra) {
    node.extra->resolved_name.clear();
  }
}

inline const std::shared_ptr<cpp_decl::QualifiedName> &
callsem_qualified_name_syntax(const CallSemNode & node)
{
  return node.extra ? node.extra->qualified_name_syntax :
      callsem_empty_qualified_name_syntax();
}

inline void set_callsem_qualified_name_syntax(
    CallSemNode & node,
    const std::shared_ptr<cpp_decl::QualifiedName> & value)
{
  if(!value && !node.extra) {
    return;
  }
  ensure_callsem_extra(node).qualified_name_syntax = value;
}

inline void set_callsem_qualified_name_syntax(
    CallSemNode & node,
    const cpp_decl::QualifiedName & value)
{
  ensure_callsem_extra(node).qualified_name_syntax.reset(
      new cpp_decl::QualifiedName(value));
}

inline void clear_callsem_qualified_name_syntax(CallSemNode & node)
{
  if(node.extra) {
    node.extra->qualified_name_syntax.reset();
  }
}

inline symbol_linkage::SpecialMemberEntryPointKind
callsem_special_member_entry_point_kind(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->special_member_entry_point_kind :
      symbol_linkage::SMEK_COMPLETE;
}

inline void set_callsem_special_member_entry_point_kind(
    CallSemNode & node,
    symbol_linkage::SpecialMemberEntryPointKind value)
{
  if(value == symbol_linkage::SMEK_COMPLETE && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).special_member_entry_point_kind = value;
}

inline symbol_linkage::FunctionRefQualifier
callsem_function_ref_qualifier(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->function_ref_qualifier : symbol_linkage::FRQ_NONE;
}

inline void set_callsem_function_ref_qualifier(
    CallSemNode & node,
    symbol_linkage::FunctionRefQualifier value)
{
  if(value == symbol_linkage::FRQ_NONE && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).function_ref_qualifier = value;
}

inline const std::vector<std::string> & callsem_abi_tags(const CallSemNode & node)
{
  static const std::vector<std::string> empty;
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->abi_tags : empty;
}

inline void set_callsem_abi_tags(CallSemNode & node,
                                 const std::vector<std::string> & value)
{
  if(value.empty() && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).abi_tags = value;
}

inline const std::vector<std::string> & callsem_object_aliases(const CallSemNode & node)
{
  static const std::vector<std::string> empty;
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->object_aliases : empty;
}

inline void set_callsem_object_aliases(CallSemNode & node,
                                       const std::vector<std::string> & value)
{
  if(value.empty() && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).object_aliases = value;
}

inline long long callsem_result_adjust(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->result_adjust : 0;
}

inline void set_callsem_result_adjust(CallSemNode & node, long long value)
{
  if(value == 0 && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).result_adjust = value;
}

inline long long callsem_virtual_dispatch_view_offset(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->virtual_dispatch_view_offset : 0;
}

inline void set_callsem_virtual_dispatch_view_offset(CallSemNode & node,
                                                     long long value)
{
  if(value == 0 && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).virtual_dispatch_view_offset = value;
}

inline unsigned long long callsem_bit_field_width(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->bit_field_width : 0;
}

inline void set_callsem_bit_field_width(CallSemNode & node,
                                        unsigned long long value)
{
  if(value == 0 && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).bit_field_width = value;
}

inline unsigned long long callsem_bit_field_offset(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->bit_field_offset : 0;
}

inline void set_callsem_bit_field_offset(CallSemNode & node,
                                         unsigned long long value)
{
  if(value == 0 && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).bit_field_offset = value;
}

inline unsigned long long callsem_bit_field_storage_size(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->bit_field_storage_size : 0;
}

inline void set_callsem_bit_field_storage_size(CallSemNode & node,
                                               unsigned long long value)
{
  if(value == 0 && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).bit_field_storage_size = value;
}

inline unsigned long long callsem_trivial_storage_copy_prefix_bytes(
    const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->trivial_storage_copy_prefix_bytes : 0;
}

inline void set_callsem_trivial_storage_copy_prefix_bytes(
    CallSemNode & node,
    unsigned long long value)
{
  if(value == 0 && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).trivial_storage_copy_prefix_bytes = value;
}

inline unsigned long long callsem_vtt_slice_offset(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->vtt_slice_offset : 0;
}

inline void set_callsem_vtt_slice_offset(CallSemNode & node,
                                         unsigned long long value)
{
  if(value == 0 && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).vtt_slice_offset = value;
}

inline unsigned long long callsem_vtt_entry_index(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->vtt_entry_index : 0;
}

inline void set_callsem_vtt_entry_index(CallSemNode & node,
                                        unsigned long long value)
{
  if(value == 0 && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).vtt_entry_index = value;
}

inline const std::string & callsem_vtt_symbol(const CallSemNode & node)
{
  return node.extra && node.extra->rare_strings ?
      node.extra->rare_strings->vtt_symbol :
      callsem_empty_extra_string();
}

inline void set_callsem_vtt_symbol(CallSemNode & node, const std::string & value)
{
  if(value.empty() &&
     (!node.extra || !node.extra->rare_strings)) {
    return;
  }
  ensure_callsem_rare_strings(node).vtt_symbol = value;
}

inline const std::string & callsem_vtt_object_symbol(const CallSemNode & node)
{
  return node.extra && node.extra->rare_strings ?
      node.extra->rare_strings->vtt_object_symbol :
      callsem_empty_extra_string();
}

inline void set_callsem_vtt_object_symbol(CallSemNode & node, const std::string & value)
{
  if(value.empty() &&
     (!node.extra || !node.extra->rare_strings)) {
    return;
  }
  ensure_callsem_rare_strings(node).vtt_object_symbol = value;
}

inline const std::string & callsem_runtime_bridge_symbol(const CallSemNode & node)
{
  return node.extra && node.extra->rare_strings ?
      node.extra->rare_strings->runtime_bridge_symbol :
      callsem_empty_extra_string();
}

inline void set_callsem_runtime_bridge_symbol(CallSemNode & node, const std::string & value)
{
  if(value.empty() &&
     (!node.extra || !node.extra->rare_strings)) {
    return;
  }
  ensure_callsem_rare_strings(node).runtime_bridge_symbol = value;
}

inline const std::string & callsem_local_static_guard_symbol(const CallSemNode & node)
{
  return node.extra && node.extra->rare_strings ?
      node.extra->rare_strings->local_static_guard_symbol :
      callsem_empty_extra_string();
}

inline void set_callsem_local_static_guard_symbol(CallSemNode & node,
                                                  const std::string & value)
{
  if(value.empty() &&
     (!node.extra || !node.extra->rare_strings)) {
    return;
  }
  ensure_callsem_rare_strings(node).local_static_guard_symbol = value;
}

inline const cpp_decl::TypePtr & callsem_vtt_owner_type(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->vtt_owner_type : callsem_empty_extra_type();
}

inline void set_callsem_vtt_owner_type(CallSemNode & node,
                                       const cpp_decl::TypePtr & value)
{
  if(!value && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).vtt_owner_type = value;
}

inline const cpp_decl::TypePtr &
callsem_materialization_source_type(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ?
      payload->materialization_source_type :
      callsem_empty_extra_type();
}

inline void set_callsem_materialization_source_type(
    CallSemNode & node,
    const cpp_decl::TypePtr & value)
{
  if(!value && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).materialization_source_type = value;
}

inline const cpp_decl::TypePtr &
callsem_conversion_source_type(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->conversion_source_type : callsem_empty_extra_type();
}

inline void set_callsem_conversion_source_type(CallSemNode & node,
                                               const cpp_decl::TypePtr & value)
{
  if(!value && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).conversion_source_type = value;
}

inline const cpp_decl::TypePtr &
callsem_initializer_list_element_type(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ?
      payload->initializer_list_element_type :
      callsem_empty_extra_type();
}

inline void set_callsem_initializer_list_element_type(
    CallSemNode & node,
    const cpp_decl::TypePtr & value)
{
  if(!value && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).initializer_list_element_type = value;
}

inline const cpp_decl::TypePtr &
callsem_typeid_operand_type(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ? payload->typeid_operand_type : callsem_empty_extra_type();
}

inline void set_callsem_typeid_operand_type(CallSemNode & node,
                                            const cpp_decl::TypePtr & value)
{
  if(!value && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).typeid_operand_type = value;
}

inline const CallSemVirtualBaseLayout &
callsem_virtual_base_layout(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ?
      payload->virtual_base_layout :
      callsem_empty_virtual_base_layout();
}

inline CallSemVirtualBaseLayout & mutable_callsem_virtual_base_layout(CallSemNode & node)
{
  return ensure_callsem_rare_payload(node).virtual_base_layout;
}

inline void set_callsem_virtual_base_layout(
    CallSemNode & node,
    const CallSemVirtualBaseLayout & value)
{
  if(value.empty() && !callsem_rare_payload(node)) {
    return;
  }
  ensure_callsem_rare_payload(node).virtual_base_layout = value;
}

inline const std::shared_ptr<CallSemNode> &
callsem_lowered_condition_test(const CallSemNode & node)
{
  const CallSemRarePayload * payload = callsem_rare_payload(node);
  return payload ?
      payload->lowered_condition_test :
      callsem_empty_lowered_condition_test();
}

inline std::shared_ptr<CallSemNode> &
mutable_callsem_lowered_condition_test(CallSemNode & node)
{
  return ensure_callsem_rare_payload(node).lowered_condition_test;
}

inline bool CallSemNode::has_source_location() const
{
  return !callsem_source_file(*this).empty() &&
         callsem_source_line(*this) != 0 &&
         callsem_source_column(*this) != 0;
}

bool callsem_has_token(const CallSemNode & node, ETokenType type);
inline void append_callsem_recursive_input_children(
    const CallSemNode & node,
    std::vector<const CallSemNode *> & out)
{
  out.clear();
  const std::shared_ptr<CallSemNode> & lowered_test =
      callsem_lowered_condition_test(node);
  out.reserve(node.children.size() + (lowered_test ? 1 : 0));
  for(size_t i = 0; i < node.children.size(); ++i) {
    out.push_back(&node.children[i]);
  }
  if(lowered_test) {
    out.push_back(lowered_test.get());
  }
}

CallSemNode make_dump_node(CallSemKind kind, const std::string & text = std::string());
void set_dump_source_location(CallSemNode & node, const CppAstNode & ast);
void set_dump_token(CallSemNode & node, const CppAstNode & ast);
void set_dump_symbol(CallSemNode & node, const symbol_linkage::SymbolIdentity & symbol);

class SemanticContext;

struct ScopedCallSemDumpSourceLocationContext
{
  explicit ScopedCallSemDumpSourceLocationContext(const SemanticContext * ctx);
  ~ScopedCallSemDumpSourceLocationContext();

private:
  const SemanticContext * saved_;
};
std::string runtime_bridge_symbol_for_function_type(const cpp_decl::TypePtr & function_type);
std::string runtime_bridge_symbol_for_function_name_and_type(const std::string & name,
                                                             const cpp_decl::TypePtr & function_type);
std::string runtime_bridge_symbol_for_bound_function(const std::string & qualified_name,
                                                     const std::string & owner_class_name,
                                                     const cpp_decl::TypePtr & function_type);
std::string runtime_bridge_symbol_for_object_symbol(const std::string & object_symbol);

std::string call_value_category_text(CallValueCategory category);
std::string callsem_payload_text(const CallSemNode & node);
std::string callsem_display_text(const CallSemNode & node);
std::string callsem_dump_tree(const CallSemNode & node);
