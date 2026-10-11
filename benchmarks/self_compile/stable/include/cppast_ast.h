#ifndef CPPGM_CPPAST_AST_H
#define CPPGM_CPPAST_AST_H

#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "cpp_decl_model.h"
#include "recog_token.h"
#include "template_angle_lookup.h"

#define CPP_AST_KIND_LIST(X) \
  X(invalid, "<invalid>") \
  X(abstract_declarator, "abstract-declarator") \
  X(access_specifier, "access-specifier") \
  X(alias_declaration, "alias-declaration") \
  X(asm_clause, "asm-clause") \
  X(asm_operand, "asm-operand") \
  X(asm_statement, "asm-statement") \
  X(argument_list, "argument-list") \
  X(array_delete, "array-delete") \
  X(array_suffix, "array-suffix") \
  X(assignment_expression, "assignment-expression") \
  X(attributed_statement, "attributed-statement") \
  X(base_clause, "base-clause") \
  X(base_name, "base-name") \
  X(base_specifier, "base-specifier") \
  X(bit_field_declaration, "bit-field-declaration") \
  X(bit_field_declarator, "bit-field-declarator") \
  X(binary_expression, "binary-expression") \
  X(braced_init_list, "braced-init-list") \
  X(break_statement, "break-statement") \
  X(call_expression, "call-expression") \
  X(case_statement, "case-statement") \
  X(cast_expression, "cast-expression") \
  X(class_forward_declaration, "class-forward-declaration") \
  X(class_key, "class-key") \
  X(class_specifier, "class-specifier") \
  X(compound_statement, "compound-statement") \
  X(condition, "condition") \
  X(condition_declaration, "condition-declaration") \
  X(conditional_expression, "conditional-expression") \
  X(continue_statement, "continue-statement") \
  X(coroutine_return_statement, "coroutine-return-statement") \
  X(ctor_initializer, "ctor-initializer") \
  X(cv_qualifier, "cv-qualifier") \
  X(decl_specifier, "decl-specifier") \
  X(decl_specifier_seq, "decl-specifier-seq") \
  X(declarator, "declarator") \
  X(designated_initializer, "designated-initializer") \
  X(designator, "designator") \
  X(decltype_specifier, "decltype-specifier") \
  X(default_argument, "default-argument") \
  X(default_statement, "default-statement") \
  X(default_template_argument, "default-template-argument") \
  X(delete_expression, "delete-expression") \
  X(deduction_guide_declaration, "deduction-guide-declaration") \
  X(do_statement, "do-statement") \
  X(ellipsis, "ellipsis") \
  X(else_node, "else") \
  X(empty_declaration, "empty-declaration") \
  X(enum_key, "enum-key") \
  X(enum_specifier, "enum-specifier") \
  X(enumerator, "enumerator") \
  X(exception_declaration, "exception-declaration") \
  X(explicit_instantiation_declaration, "explicit-instantiation-declaration") \
  X(explicit_instantiation_definition, "explicit-instantiation-definition") \
  X(expression_statement, "expression-statement") \
  X(for_init_statement, "for-init-statement") \
  X(for_statement, "for-statement") \
  X(fold_expression, "fold-expression") \
  X(function_definition, "function-definition") \
  X(function_qualifier, "function-qualifier") \
  X(lazy_function_body, "lazy-function-body") \
  X(global_scope, "global-scope") \
  X(goto_statement, "goto-statement") \
  X(handler, "handler") \
  X(id_expression, "id-expression") \
  X(identifier, "identifier") \
  X(if_statement, "if-statement") \
  X(init_declarator, "init-declarator") \
  X(init_declarator_list, "init-declarator-list") \
  X(initializer, "initializer") \
  X(inline_node, "inline") \
  X(iteration, "iteration") \
  X(keyword_literal, "keyword-literal") \
  X(linkage_specification, "linkage-specification") \
  X(labeled_statement, "labeled-statement") \
  X(lambda_declarator, "lambda-declarator") \
  X(lambda_expression, "lambda-expression") \
  X(lambda_introducer, "lambda-introducer") \
  X(lambda_specifier, "lambda-specifier") \
  X(literal, "literal") \
  X(mem_initializer, "mem-initializer") \
  X(mem_initializer_id, "mem-initializer-id") \
  X(member_expression, "member-expression") \
  X(member_specifiers, "member-specifiers") \
  X(message, "message") \
  X(namespace_alias_definition, "namespace-alias-definition") \
  X(namespace_definition, "namespace-definition") \
  X(nested_declarator, "nested-declarator") \
  X(new_expression, "new-expression") \
  X(noexcept_specification, "noexcept-specification") \
  X(nullability_qualifier, "nullability-qualifier") \
  X(non_type_template_parameter, "non-type-template-parameter") \
  X(parameter_clause, "parameter-clause") \
  X(parameter_declaration, "parameter-declaration") \
  X(parameter_key, "parameter-key") \
  X(parameter_pack, "parameter-pack") \
  X(paren_argument_list, "paren-argument-list") \
  X(paren_initializer, "paren-initializer") \
  X(pack_expansion_expression, "pack-expansion-expression") \
  X(parenthesized_expression, "parenthesized-expression") \
  X(placement, "placement") \
  X(postfix_expression, "postfix-expression") \
  X(ptr_operator, "ptr-operator") \
  X(range_declaration, "range-declaration") \
  X(range_for_statement, "range-for-statement") \
  X(range_initializer, "range-initializer") \
  X(ref_qualifier, "ref-qualifier") \
  X(return_statement, "return-statement") \
  X(simple_declaration, "simple-declaration") \
  X(sizeof_expression, "sizeof-expression") \
  X(sizeof_pack_expression, "sizeof-pack-expression") \
  X(statement_expression, "statement-expression") \
  X(special_definition, "special-definition") \
  X(special_initializer, "special-initializer") \
  X(special_member_declaration, "special-member-declaration") \
  X(special_member_definition, "special-member-definition") \
  X(specifier, "specifier") \
  X(static_assert_declaration, "static-assert-declaration") \
  X(structured_binding_declaration, "structured-binding-declaration") \
  X(structured_binding_declarator, "structured-binding-declarator") \
  X(structured_binding_identifier_list, "structured-binding-identifier-list") \
  X(subscript_expression, "subscript-expression") \
  X(switch_statement, "switch-statement") \
  X(target, "target") \
  X(template_declaration, "template-declaration") \
  X(template_parameter_clause, "template-parameter-clause") \
  X(template_parameter_list, "template-parameter-list") \
  X(template_template_parameter, "template-template-parameter") \
  X(then_node, "then") \
  X(throw_statement, "throw-statement") \
  X(trailing_return_type, "trailing-return-type") \
  X(translation_unit, "translation-unit") \
  X(try_block, "try-block") \
  X(type_id, "type-id") \
  X(type_name, "type-name") \
  X(type_parameter, "type-parameter") \
  X(type_specifier, "type-specifier") \
  X(type_specifier_seq, "type-specifier-seq") \
  X(type_trait_expression, "type-trait-expression") \
  X(unary_expression, "unary-expression") \
  X(using_declaration, "using-declaration") \
  X(using_directive, "using-directive") \
  X(virt_specifier, "virt-specifier") \
  X(virtual_node, "virtual") \
  X(while_statement, "while-statement")

enum class CppAstKind
{
#define CPP_AST_KIND_ENUM(name, text) name,
  CPP_AST_KIND_LIST(CPP_AST_KIND_ENUM)
#undef CPP_AST_KIND_ENUM
};

template <typename T>
class CppAstLazyVector
{
public:
  typedef typename std::vector<T>::iterator iterator;
  typedef typename std::vector<T>::const_iterator const_iterator;

  CppAstLazyVector() {}

  CppAstLazyVector(const CppAstLazyVector & other)
  {
    if(other.values_) {
      values_.reset(new std::vector<T>(*other.values_));
    }
  }

  CppAstLazyVector(CppAstLazyVector && other) noexcept
    : values_(std::move(other.values_))
  {
  }

  CppAstLazyVector(const std::vector<T> & values)
  {
    if(!values.empty()) {
      values_.reset(new std::vector<T>(values));
    }
  }

  CppAstLazyVector(std::vector<T> && values)
  {
    if(!values.empty()) {
      values_.reset(new std::vector<T>(std::move(values)));
    }
  }

  CppAstLazyVector & operator=(const CppAstLazyVector & other)
  {
    if(this == &other) {
      return *this;
    }
    if(other.values_) {
      values_.reset(new std::vector<T>(*other.values_));
    } else {
      values_.reset();
    }
    return *this;
  }

  CppAstLazyVector & operator=(CppAstLazyVector && other) noexcept
  {
    values_ = std::move(other.values_);
    return *this;
  }

  CppAstLazyVector & operator=(const std::vector<T> & values)
  {
    if(values.empty()) {
      values_.reset();
    } else {
      values_.reset(new std::vector<T>(values));
    }
    return *this;
  }

  CppAstLazyVector & operator=(std::vector<T> && values)
  {
    if(values.empty()) {
      values_.reset();
    } else {
      values_.reset(new std::vector<T>(std::move(values)));
    }
    return *this;
  }

  operator const std::vector<T> &() const
  {
    return as_vector();
  }

  operator std::vector<T> &()
  {
    return mutable_vector();
  }

  bool empty() const
  {
    return !values_ || values_->empty();
  }

  std::size_t size() const
  {
    return values_ ? values_->size() : 0;
  }

  void clear()
  {
    values_.reset();
  }

  void reserve(std::size_t count)
  {
    mutable_vector().reserve(count);
  }

  void push_back(const T & value)
  {
    mutable_vector().push_back(value);
  }

  void push_back(T && value)
  {
    mutable_vector().push_back(std::move(value));
  }

  template <typename InputIt>
  void assign(InputIt first, InputIt last)
  {
    if(first == last) {
      values_.reset();
      return;
    }
    mutable_vector().assign(first, last);
  }

  T & operator[](std::size_t index)
  {
    return mutable_vector()[index];
  }

  const T & operator[](std::size_t index) const
  {
    return as_vector()[index];
  }

  T & front()
  {
    return mutable_vector().front();
  }

  const T & front() const
  {
    return as_vector().front();
  }

  T & back()
  {
    return mutable_vector().back();
  }

  const T & back() const
  {
    return as_vector().back();
  }

  iterator begin()
  {
    return mutable_vector().begin();
  }

  iterator end()
  {
    return mutable_vector().end();
  }

  const_iterator begin() const
  {
    return as_vector().begin();
  }

  const_iterator end() const
  {
    return as_vector().end();
  }

  const_iterator cbegin() const
  {
    return as_vector().cbegin();
  }

  const_iterator cend() const
  {
    return as_vector().cend();
  }

  const std::vector<T> & as_vector() const
  {
    return values_ ? *values_ : empty_vector();
  }

  std::vector<T> & mutable_vector()
  {
    if(!values_) {
      values_.reset(new std::vector<T>());
    }
    return *values_;
  }

private:
  static const std::vector<T> & empty_vector()
  {
    static const std::vector<T> empty;
    return empty;
  }

  std::unique_ptr<std::vector<T> > values_;
};

inline const char * cppast_kind_text(CppAstKind kind)
{
  switch(kind) {
#define CPP_AST_KIND_CASE(name, text) case CppAstKind::name: return text;
    CPP_AST_KIND_LIST(CPP_AST_KIND_CASE)
#undef CPP_AST_KIND_CASE
  }
  return "<invalid>";
}

struct CppAstNameLookupSnapshot
{
  typedef template_angle_lookup::NameSet NameSet;
  typedef std::vector<NameSet> NameSetStack;

  NameSetStack template_type_parameter_scopes;
  NameSetStack template_value_parameter_scopes;
  NameSetStack template_name_scopes;
  NameSetStack type_name_scopes;
  NameSetStack value_name_scopes;
  std::vector<std::string> class_name_stack;
  std::vector<std::string> namespace_path_stack;
  std::vector<bool> namespace_inline_stack;
};

struct CppAstNode
{
  CppAstKind kind = CppAstKind::invalid;
  std::string value;
  cpp_decl::TypePtr semantic_type;
  std::string builtin_type_transform_name;
  std::shared_ptr<cpp_decl::QualifiedName> qualified_name_syntax;
  std::shared_ptr<cpp_decl::TemplateIdSyntax> template_id_syntax;
  std::shared_ptr<CppAstNode> conversion_type_id_syntax;
  std::shared_ptr<CppAstNode> base_type_syntax;
  CppAstLazyVector<cpp_decl::TemplateIdSyntax> qualifier_template_id_syntaxes;
  CppAstLazyVector<CppAstNode> qualifier_type_syntaxes;
  bool has_leading_typename = false;
  bool allows_implicit_typename = false;
  bool has_exception_type_id_syntaxes = false;
  CppAstLazyVector<CppAstNode> exception_type_id_syntaxes;
  bool linkage_has_braces = false;
  bool has_no_unique_address = false;
  bool has_using_if_exists = false;
  bool has_exclude_from_explicit_instantiation = false;
  std::string asm_label;
  CppAstLazyVector<std::string> abi_tags;
  CppAstLazyVector<std::string> alignment_specifiers;
  CppAstLazyVector<CppAstNode> alignment_specifier_nodes;
  bool is_final_specifier = false;
  bool uses_assignment_form = false;
  bool is_typeof_specifier = false;
  bool has_token = false;
  ERecogTokenKind token_kind = RT_EOF;
  ETokenType simple_type = static_cast<ETokenType>(0);
  // Token half-open range [token_start, token_end) in the original RecogToken stream.
  std::size_t token_start = 0;
  std::size_t token_end = 0;
  uint32_t source_location_id = 0;
  std::shared_ptr<const CppAstNameLookupSnapshot> name_lookup_snapshot;
  std::vector<CppAstNode> children;
};

inline bool node_has_simple_type(const CppAstNode & node, ETokenType type)
{
  return node.has_token && node.token_kind == RT_SIMPLE && node.simple_type == type;
}

inline bool node_has_kind(const CppAstNode & node, CppAstKind kind)
{
  return node.kind == kind;
}

inline const cpp_decl::QualifiedName * cppast_qualified_name_syntax(const CppAstNode & node)
{
  return node.qualified_name_syntax.get();
}

inline void set_cppast_qualified_name_syntax(
    CppAstNode & node,
    const cpp_decl::QualifiedName & qualified_name)
{
  node.qualified_name_syntax.reset(new cpp_decl::QualifiedName(qualified_name));
}

inline void set_cppast_qualified_name_syntax(
    CppAstNode & node,
    cpp_decl::QualifiedName && qualified_name)
{
  node.qualified_name_syntax.reset(
      new cpp_decl::QualifiedName(std::move(qualified_name)));
}

inline const cpp_decl::TemplateIdSyntax * cppast_template_id_syntax(
    const CppAstNode & node)
{
  return node.template_id_syntax.get();
}

inline void set_cppast_template_id_syntax(
    CppAstNode & node,
    const cpp_decl::TemplateIdSyntax & template_id)
{
  node.template_id_syntax.reset(new cpp_decl::TemplateIdSyntax(template_id));
}

inline void set_cppast_template_id_syntax(
    CppAstNode & node,
    cpp_decl::TemplateIdSyntax && template_id)
{
  node.template_id_syntax.reset(
      new cpp_decl::TemplateIdSyntax(std::move(template_id)));
}

inline const CppAstNode * cppast_conversion_type_id_syntax(
    const CppAstNode & node)
{
  return node.conversion_type_id_syntax.get();
}

inline void set_cppast_conversion_type_id_syntax(
    CppAstNode & node,
    const CppAstNode & type_id)
{
  node.conversion_type_id_syntax.reset(new CppAstNode(type_id));
}

inline void set_cppast_conversion_type_id_syntax(
    CppAstNode & node,
    CppAstNode && type_id)
{
  node.conversion_type_id_syntax.reset(new CppAstNode(std::move(type_id)));
}

inline const std::vector<CppAstNode> * cppast_exception_type_id_syntaxes(
    const CppAstNode & node)
{
  return node.has_exception_type_id_syntaxes ?
      &node.exception_type_id_syntaxes.as_vector() :
      nullptr;
}

inline void set_cppast_exception_type_id_syntaxes(
    CppAstNode & node,
    const std::vector<CppAstNode> & type_ids)
{
  node.has_exception_type_id_syntaxes = true;
  node.exception_type_id_syntaxes = type_ids;
}

inline void set_cppast_exception_type_id_syntaxes(
    CppAstNode & node,
    std::vector<CppAstNode> && type_ids)
{
  node.has_exception_type_id_syntaxes = true;
  node.exception_type_id_syntaxes = std::move(type_ids);
}

inline const cpp_decl::TemplateIdSyntax * cppast_qualifier_template_id_syntax(
    const CppAstNode & node,
    std::size_t qualifier_index)
{
  if(qualifier_index < node.qualifier_template_id_syntaxes.size() &&
     !node.qualifier_template_id_syntaxes[qualifier_index].name.name.empty()) {
    return &node.qualifier_template_id_syntaxes[qualifier_index];
  }
  if(node.template_id_syntax &&
     qualifier_index <
         node.template_id_syntax->qualifier_template_id_syntaxes.size() &&
     !node.template_id_syntax
          ->qualifier_template_id_syntaxes[qualifier_index]
          .name.name.empty()) {
    return &node.template_id_syntax
                ->qualifier_template_id_syntaxes[qualifier_index];
  }
  return nullptr;
}

inline bool cppast_has_qualifier_template_id_syntaxes(const CppAstNode & node)
{
  for(std::size_t i = 0; i < node.qualifier_template_id_syntaxes.size(); ++i) {
    if(!node.qualifier_template_id_syntaxes[i].name.name.empty()) {
      return true;
    }
  }
  if(!node.template_id_syntax) {
    return false;
  }
  for(std::size_t i = 0;
      i < node.template_id_syntax->qualifier_template_id_syntaxes.size();
      ++i) {
    if(!node.template_id_syntax
            ->qualifier_template_id_syntaxes[i]
            .name.name.empty()) {
      return true;
    }
  }
  return false;
}

inline void set_cppast_qualifier_template_id_syntaxes(
    CppAstNode & node,
    const std::vector<cpp_decl::TemplateIdSyntax> & qualifier_template_ids)
{
  node.qualifier_template_id_syntaxes = qualifier_template_ids;
}

inline void set_cppast_qualifier_template_id_syntaxes(
    CppAstNode & node,
    std::vector<cpp_decl::TemplateIdSyntax> && qualifier_template_ids)
{
  node.qualifier_template_id_syntaxes = std::move(qualifier_template_ids);
}

inline const CppAstNode * cppast_qualifier_type_syntax(
    const CppAstNode & node,
    std::size_t qualifier_index)
{
  if(qualifier_index >= node.qualifier_type_syntaxes.size() ||
     node.qualifier_type_syntaxes[qualifier_index].kind == CppAstKind::invalid) {
    return nullptr;
  }
  return &node.qualifier_type_syntaxes[qualifier_index];
}

inline void set_cppast_qualifier_type_syntaxes(
    CppAstNode & node,
    const std::vector<CppAstNode> & qualifier_type_syntaxes)
{
  node.qualifier_type_syntaxes = qualifier_type_syntaxes;
}

inline void set_cppast_qualifier_type_syntaxes(
    CppAstNode & node,
    std::vector<CppAstNode> && qualifier_type_syntaxes)
{
  node.qualifier_type_syntaxes = std::move(qualifier_type_syntaxes);
}

inline void append_unique_cppast_text(std::vector<std::string> & out,
                                      const std::string & value)
{
  if(value.empty()) {
    return;
  }
  for(std::size_t i = 0; i < out.size(); ++i) {
    if(out[i] == value) {
      return;
    }
  }
  out.push_back(value);
}

inline void append_cppast_alignment_specifier(CppAstNode & out,
                                              const std::string & text,
                                              const CppAstNode * syntax)
{
  if(text.empty()) {
    return;
  }
  for(std::size_t i = 0; i < out.alignment_specifiers.size(); ++i) {
    if(out.alignment_specifiers[i] == text) {
      return;
    }
  }
  out.alignment_specifiers.push_back(text);
  out.alignment_specifier_nodes.push_back(syntax ? *syntax : CppAstNode());
}

inline void append_cppast_abi_tags(std::vector<std::string> & out,
                                   const CppAstNode & node)
{
  for(std::size_t i = 0; i < node.abi_tags.size(); ++i) {
    append_unique_cppast_text(out, node.abi_tags[i]);
  }
}

inline void append_cppast_alignment_specifiers(CppAstNode & out,
                                               const CppAstNode & node)
{
  for(std::size_t i = 0; i < node.alignment_specifiers.size(); ++i) {
    const CppAstNode * syntax =
        (i < node.alignment_specifier_nodes.size() &&
         node.alignment_specifier_nodes[i].kind != CppAstKind::invalid) ?
            &node.alignment_specifier_nodes[i] :
            nullptr;
    append_cppast_alignment_specifier(out, node.alignment_specifiers[i], syntax);
  }
}

inline const CppAstNode * cppast_find_child_kind(const CppAstNode & node,
                                                 CppAstKind kind,
                                                 std::size_t ordinal = 0)
{
  for(std::size_t i = 0; i < node.children.size(); ++i) {
    if(node.children[i].kind != kind) {
      continue;
    }
    if(ordinal == 0) {
      return &node.children[i];
    }
    --ordinal;
  }
  return nullptr;
}

inline bool cppast_recover_sizeof_type_id_expression_operand(
    const CppAstNode & type_id,
    CppAstNode & out)
{
  if(type_id.kind != CppAstKind::type_id) {
    return false;
  }

  const CppAstNode * specifiers =
      cppast_find_child_kind(type_id, CppAstKind::type_specifier_seq);
  if(!specifiers ||
     specifiers->children.size() != 1 ||
     specifiers->children[0].kind != CppAstKind::type_name) {
    return false;
  }

  const CppAstNode & name = specifiers->children[0];
  const cpp_decl::QualifiedName * qualified =
      cppast_qualified_name_syntax(name);
  if(name.has_leading_typename ||
     !qualified ||
     (!qualified->rooted && qualified->qualifiers.empty())) {
    return false;
  }

  CppAstNode expr = name;
  expr.kind = CppAstKind::id_expression;
  expr.children.clear();

  for(std::size_t i = 0; i < type_id.children.size(); ++i) {
    const CppAstNode & child = type_id.children[i];
    if(child.kind == CppAstKind::type_specifier_seq) {
      continue;
    }
    if(child.kind != CppAstKind::abstract_declarator) {
      return false;
    }
    for(std::size_t j = 0; j < child.children.size(); ++j) {
      const CppAstNode & suffix = child.children[j];
      if(suffix.kind != CppAstKind::array_suffix ||
         suffix.children.size() != 1) {
        return false;
      }

      CppAstNode subscript;
      subscript.kind = CppAstKind::subscript_expression;
      subscript.token_start = expr.token_start;
      subscript.token_end = suffix.token_end;
      subscript.source_location_id = expr.source_location_id;
      subscript.children.push_back(std::move(expr));
      subscript.children.push_back(suffix.children[0]);
      expr = std::move(subscript);
    }
  }

  out = std::move(expr);
  return true;
}

inline void append_function_declaration_abi_tags(std::vector<std::string> & out,
                                                 const CppAstNode * node)
{
  if(!node) {
    return;
  }

  append_cppast_abi_tags(out, *node);

  if(node->kind == CppAstKind::template_declaration) {
    for(std::size_t i = 0; i < node->children.size(); ++i) {
      if(node->children[i].kind == CppAstKind::template_parameter_clause) {
        continue;
      }
      append_function_declaration_abi_tags(out, &node->children[i]);
    }
    return;
  }

  if(const CppAstNode * specifiers =
         cppast_find_child_kind(*node, CppAstKind::decl_specifier_seq)) {
    append_cppast_abi_tags(out, *specifiers);
  }
  if(const CppAstNode * member_specifiers =
         cppast_find_child_kind(*node, CppAstKind::member_specifiers)) {
    append_cppast_abi_tags(out, *member_specifiers);
  }
  if(const CppAstNode * init_declarator_list =
         cppast_find_child_kind(*node, CppAstKind::init_declarator_list)) {
    for(std::size_t i = 0; i < init_declarator_list->children.size(); ++i) {
      const CppAstNode & child = init_declarator_list->children[i];
      if(child.kind != CppAstKind::init_declarator) {
        continue;
      }
      append_cppast_abi_tags(out, child);
      if(const CppAstNode * declarator =
             cppast_find_child_kind(child, CppAstKind::declarator)) {
        append_cppast_abi_tags(out, *declarator);
      }
    }
  }
  if(const CppAstNode * declarator = cppast_find_child_kind(*node, CppAstKind::declarator)) {
    append_cppast_abi_tags(out, *declarator);
  }
}

inline void append_effective_function_abi_tags(std::vector<std::string> & out,
                                               const CppAstNode * declaration_node,
                                               const CppAstNode * definition_node)
{
  if(definition_node) {
    append_function_declaration_abi_tags(out, definition_node);
    if(!out.empty()) {
      return;
    }
  }
  append_function_declaration_abi_tags(out, declaration_node);
}

inline void append_function_specifier_and_declarator_abi_tags(
    std::vector<std::string> & out,
    const CppAstNode * specifiers,
    const CppAstNode * declarator)
{
  if(specifiers) {
    append_cppast_abi_tags(out, *specifiers);
  }
  if(declarator) {
    append_cppast_abi_tags(out, *declarator);
  }
}

inline std::string cppast_display_value(const CppAstNode & node)
{
  if(!node.has_token) {
    return node.value;
  }

  switch(node.token_kind) {
  case RT_INVALID:
    return std::string("TT_INVALID:") + node.value;
  case RT_EOF:
    return "EOF";
  case RT_RSHIFT_1:
    return "ST_RSHIFT_1";
  case RT_RSHIFT_2:
    return "ST_RSHIFT_2";
  case RT_SIMPLE:
    return std::string(token_type_to_string(node.simple_type)) + ":" + node.value;
  case RT_IDENTIFIER:
    return std::string("TT_IDENTIFIER:") + node.value;
  case RT_LITERAL:
    return std::string("TT_LITERAL:") + node.value;
  }

  return node.value;
}

#endif
