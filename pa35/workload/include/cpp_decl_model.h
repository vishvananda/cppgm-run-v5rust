#pragma once

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

#include "recog_token.h"
#include "types.h"

struct CppAstNode;

namespace semantic_model {
struct FunctionBinding;
struct Scope;
struct ValueBinding;
}

namespace cpp_decl {

struct QualifiedName
{
  bool rooted;
  std::vector<std::string> qualifiers;
  std::string name;

  QualifiedName() : rooted(false) {}
};

struct TemplateIdSyntax;
struct ClassTemplateSpecializationMangleInfo;
struct Type;
typedef std::shared_ptr<Type> TypePtr;

struct TemplateArgumentSyntax
{
  std::string text;
  std::string source_text;
  bool pack_expansion = false;
  bool dependent = false;
  bool has_source_token_start = false;
  std::size_t source_token_start = 0;
  uint32_t source_location_id = 0;
  std::shared_ptr<TemplateIdSyntax> template_id;
  std::shared_ptr<CppAstNode> type_id;
  std::shared_ptr<CppAstNode> source_type_id;
  std::shared_ptr<CppAstNode> expression;
  TypePtr resolved_type;
};

struct TemplateIdSyntax
{
  QualifiedName name;
  uint32_t source_location_id = 0;
  std::vector<TemplateIdSyntax> qualifier_template_id_syntaxes;
  std::vector<std::string> arguments;
  std::vector<TemplateArgumentSyntax> argument_syntaxes;
};

struct DependentAliasTemplateArgumentSyntax
{
  std::string text;
  TypePtr type;
  TemplateArgumentSyntax syntax;
  const semantic_model::FunctionBinding * function_value = nullptr;
  std::string function_internal_symbol;
  const semantic_model::ValueBinding * value_binding = nullptr;
  long long value = 0;
  semantic_model::Scope * semantic_scope = nullptr;
  std::shared_ptr<semantic_model::Scope> semantic_scope_storage;
  bool has_non_type_value = false;
  bool dependent_value = false;
  bool source_defaulted = false;
  bool partial_order_placeholder = false;
};

enum FunctionTypeRefQualifier
{
  FTRQ_NONE,
  FTRQ_LVALUE,
  FTRQ_RVALUE
};

struct Type
{
  struct HostAbiChunk
  {
    enum Kind
    {
      HC_INTEGER,
      HC_SSE
    } kind = HC_INTEGER;

    std::size_t size = 0;
  };

  struct LambdaMangleMetadata
  {
    QualifiedName context_function_qualified_name;
    std::string context_function_display_name;
    TypePtr context_function_type;
    std::shared_ptr<void> context_function_symbol_options;
    std::vector<TypePtr> signature_parameter_types;
    std::vector<std::string> namespace_qualifiers;
    std::string discriminator;
    std::string local_source_name;
  };

  struct NamedRareMetadata
  {
    QualifiedName qualified_name;
    std::string source_name;
    std::shared_ptr<LambdaMangleMetadata> lambda_mangle;
    bool dependent_type_expression_formed_with_placeholders = false;
  };

  enum Kind
  {
    TK_FUNDAMENTAL,
    TK_NAMED,
    TK_CV,
    TK_ATOMIC,
    TK_POINTER,
    TK_MEMBER_POINTER,
    TK_BLOCK_POINTER,
    TK_LVALUE_REFERENCE,
    TK_RVALUE_REFERENCE,
    TK_ARRAY,
    TK_FUNCTION
  };

  enum NamedSemanticKind
  {
    NSK_ORDINARY,
    NSK_TEMPLATE_PARAMETER,
    NSK_PARTIAL_ORDER,
    NSK_DEPENDENT_TYPE,
    NSK_DEPENDENT_ALIAS,
    NSK_DEPENDENT_DECLTYPE,
    NSK_DEPENDENT_TYPEOF
  };

  explicit Type(Kind kind)
    : kind(kind),
      fundamental(FT_INT),
      named_semantic_kind(NSK_ORDINARY),
      named_complete(false),
      named_has_layout(false),
      named_alignment(0),
      named_size(0),
      named_is_empty(false),
      definitely_not_class(false),
      cv_const(false),
      cv_volatile(false),
      has_bound(false),
      bound(0),
      variadic(false),
      prototype_relaxed(false),
      function_const(false),
      function_volatile(false),
      function_ref_qualifier(FTRQ_NONE),
      named_dependent_template_template_parameter_arity(static_cast<std::size_t>(-1)),
      named_dependent_qualified_leading_typename(false)
  {}

  const QualifiedName & named_qualified_name_syntax() const;
  const std::string & named_source_name() const;
  const std::shared_ptr<LambdaMangleMetadata> & named_lambda_mangle() const;
  bool dependent_type_expression_formed_with_placeholders() const;
  void set_named_qualified_name_syntax(const QualifiedName & syntax);
  void set_named_source_name(const std::string & name);
  void set_named_lambda_mangle(
      const std::shared_ptr<LambdaMangleMetadata> & metadata);
  void set_dependent_type_expression_formed_with_placeholders(bool formed);
  NamedRareMetadata & mutable_named_rare_metadata();

  Kind kind;
  EFundamentalType fundamental;
  std::string named_display;
  std::string named_key;
  std::shared_ptr<NamedRareMetadata> named_rare_metadata;
  NamedSemanticKind named_semantic_kind;
  std::string named_semantic_payload;
  bool named_complete;
  bool named_has_layout;
  std::size_t named_alignment;
  std::size_t named_size;
  TypePtr named_enum_underlying_type;
  bool named_is_empty;
  std::vector<HostAbiChunk> named_host_abi_chunks;
  bool definitely_not_class;
  bool cv_const;
  bool cv_volatile;
  bool has_bound;
  std::size_t bound;
  std::string bound_text;
  bool variadic;
  bool prototype_relaxed;
  bool function_const;
  bool function_volatile;
  FunctionTypeRefQualifier function_ref_qualifier;
  std::vector<TypePtr> params;
  TypePtr inner;
  TypePtr owner;
  std::shared_ptr<CppAstNode> named_dependent_type_expression_node;
  void * named_dependent_alias_template_decl = nullptr;
  std::vector<DependentAliasTemplateArgumentSyntax> named_dependent_alias_arguments;
  void * named_dependent_class_template_decl = nullptr;
  std::vector<DependentAliasTemplateArgumentSyntax> named_dependent_class_arguments;
  std::string named_dependent_template_template_parameter_name;
  std::size_t named_dependent_template_template_parameter_arity;
  std::vector<DependentAliasTemplateArgumentSyntax>
      named_dependent_template_template_arguments;
  std::shared_ptr<ClassTemplateSpecializationMangleInfo>
      named_class_template_specialization_mangle_info;
  TypePtr named_member_owner_type;
  std::string named_member_name;
  TypePtr named_dependent_qualified_owner;
  std::shared_ptr<TemplateIdSyntax> named_dependent_qualified_owner_template_id;
  std::vector<std::string> named_dependent_qualified_members;
  std::vector<TemplateIdSyntax> named_dependent_qualified_member_template_ids;
  bool named_dependent_qualified_leading_typename;
};

TypePtr make_fundamental(EFundamentalType type);
TypePtr make_named(const std::string & display_name,
                   const std::string & unique_key,
                   bool complete,
                   bool has_layout = false,
                   std::size_t alignment = 0,
                   std::size_t size = 0);
TypePtr make_semantic_named(const std::string & display_name,
                            Type::NamedSemanticKind semantic_kind,
                            const std::string & semantic_payload,
                            bool complete,
                            bool has_layout = false,
                            std::size_t alignment = 0,
                            std::size_t size = 0);
TypePtr make_template_parameter_type(const std::string & display_name,
                                     const std::string & semantic_payload,
                                     const std::string & source_name);
TypePtr make_dependent_type_expression_type(
    const std::string & display_name,
    Type::NamedSemanticKind semantic_kind,
    const std::string & semantic_payload,
    const CppAstNode & expression_node,
    bool formed_with_placeholders);
TypePtr make_dependent_alias_type(
    const std::string & display_name,
    const std::string & semantic_payload,
    void * alias_template_decl,
    const std::vector<DependentAliasTemplateArgumentSyntax> & arguments);
TypePtr make_dependent_qualified_member_type(
    const std::string & display_name,
    const TypePtr & owner,
    const std::vector<std::string> & members,
    bool leading_typename,
    const std::vector<TemplateIdSyntax> & member_template_ids =
        std::vector<TemplateIdSyntax>(),
    const TemplateIdSyntax & owner_template_id = TemplateIdSyntax());
bool named_type_is_template_parameter(const TypePtr & type);
bool named_type_is_partial_order_placeholder(const TypePtr & type);
bool named_type_is_dependent_alias(const TypePtr & type);
bool named_type_is_dependent_type(const TypePtr & type);
bool named_type_is_dependent_decltype(const TypePtr & type);
bool named_type_is_dependent_typeof(const TypePtr & type);
bool named_type_has_dependent_semantic(const TypePtr & type);
bool named_type_key_contains_dependent_semantic(const TypePtr & type);
bool named_type_key_contains_partial_order_placeholder(const TypePtr & type);
std::string named_type_semantic_payload(const TypePtr & type);
const CppAstNode * named_type_dependent_type_expression_node(const TypePtr & type);
bool named_type_dependent_type_expression_formed_with_placeholders(
    const TypePtr & type);
bool named_type_dependent_alias_template(
    const TypePtr & type,
    void *& alias_template_decl,
    std::vector<DependentAliasTemplateArgumentSyntax> & arguments);
void set_named_type_dependent_class_template(
    const TypePtr & type,
    void * class_template_decl,
    const std::vector<DependentAliasTemplateArgumentSyntax> & arguments);
bool named_type_dependent_class_template(
    const TypePtr & type,
    void *& class_template_decl,
    std::vector<DependentAliasTemplateArgumentSyntax> & arguments);
void set_named_type_dependent_template_template_parameter(
    const TypePtr & type,
    const std::string & parameter_name,
    std::size_t parameter_arity,
    const std::vector<DependentAliasTemplateArgumentSyntax> & arguments);
bool named_type_dependent_template_template_parameter(
    const TypePtr & type,
    std::string & parameter_name,
    std::size_t & parameter_arity,
    std::vector<DependentAliasTemplateArgumentSyntax> & arguments);
bool named_type_dependent_qualified_member(
    const TypePtr & type,
    TypePtr & owner,
    std::vector<std::string> & members,
    bool & leading_typename,
    std::vector<TemplateIdSyntax> * member_template_ids = nullptr);
bool type_is_definitely_not_class(const TypePtr & type);
TypePtr make_cv(const TypePtr & base, bool cv_const, bool cv_volatile);
TypePtr make_atomic(const TypePtr & base);
TypePtr make_pointer(const TypePtr & base);
TypePtr make_member_pointer(const TypePtr & owner, const TypePtr & base);
TypePtr make_block_pointer(const TypePtr & base);
TypePtr make_array(const TypePtr & element,
                   bool has_bound,
                   std::size_t bound,
                   const std::string & bound_text = std::string());
TypePtr make_function(const TypePtr & result_type,
                      const std::vector<TypePtr> & params,
                      bool variadic,
                      bool function_const = false,
                      bool function_volatile = false,
                      bool prototype_relaxed = false,
                      FunctionTypeRefQualifier function_ref_qualifier = FTRQ_NONE);
TypePtr make_lvalue_reference_raw(const TypePtr & base);
TypePtr make_rvalue_reference_raw(const TypePtr & base);
TypePtr apply_cv(const TypePtr & base, bool cv_const, bool cv_volatile);
TypePtr strip_top_level_cv(const TypePtr & type);
TypePtr normalize_parameter_type(const TypePtr & type);
TypePtr parameter_object_type(const TypePtr & type);
bool type_equals(const TypePtr & lhs, const TypePtr & rhs);
TypePtr merge_types(const TypePtr & lhs, const TypePtr & rhs);
TypePtr remove_reference_type(const TypePtr & type);
bool is_integral_type(const TypePtr & type);
bool is_floating_type(const TypePtr & type);
bool is_function_type(const TypePtr & type);
bool is_block_pointer_type(const TypePtr & type);
bool is_pointer_type(const TypePtr & type);
bool is_reference_type(const TypePtr & type);
bool is_array_type(const TypePtr & type);
bool is_bool_type(const TypePtr & type);
bool is_unsigned_integral_type(const TypePtr & type);
bool resolve_callable_function_type(const TypePtr & type, TypePtr & out);
bool is_void_type(const TypePtr & type);
bool type_is_const_object(const TypePtr & type);
TypePtr sizeof_operand_type(const TypePtr & type);
bool type_is_valid_sizeof_operand(const TypePtr & type);
bool type_is_complete(const TypePtr & type);
std::size_t type_alignment(const TypePtr & type);
std::size_t type_size(const TypePtr & type);
std::string describe_type(const TypePtr & type);
std::string template_argument_type_text(const TypePtr & type);
bool finalize_fundamental_type_specifiers(int signed_count,
                                          int unsigned_count,
                                          int short_count,
                                          int long_count,
                                          bool saw_int,
                                          bool saw_char,
                                          bool saw_char16,
                                          bool saw_char32,
                                          bool saw_wchar,
                                          bool saw_bool,
                                          bool saw_float,
                                          bool saw_double,
                                          bool saw_void,
                                          TypePtr & out);

struct TypeSpecifierAccumulator
{
  bool cv_const = false;
  bool cv_volatile = false;
  bool saw_named_type = false;
  int signed_count = 0;
  int unsigned_count = 0;
  int short_count = 0;
  int long_count = 0;
  bool saw_int = false;
  bool saw_char = false;
  bool saw_char16 = false;
  bool saw_char32 = false;
  bool saw_wchar = false;
  bool saw_bool = false;
  bool saw_float = false;
  bool saw_double = false;
  bool saw_void = false;
  TypePtr named_type;

  bool has_type_content() const;
  bool add_cv(ETokenType type);
  bool add_simple_type(ETokenType type);
  bool set_named_type(const TypePtr & type);
  bool finalize(TypePtr & out) const;
};

struct PtrOperator
{
  enum Kind
  {
    PK_POINTER,
    PK_MEMBER_POINTER,
    PK_BLOCK_POINTER,
    PK_LVALUE_REFERENCE,
    PK_RVALUE_REFERENCE
  };

  explicit PtrOperator(Kind kind)
    : kind(kind),
      cv_const(false),
      cv_volatile(false)
  {}

  Kind kind;
  bool cv_const;
  bool cv_volatile;
  TypePtr owner;
};

struct DeclaratorSuffix
{
  enum Kind
  {
    SK_ARRAY,
    SK_FUNCTION
  };

  explicit DeclaratorSuffix(Kind kind)
    : kind(kind),
      has_bound(false),
      has_evaluated_bound(false),
      bound_value(0),
      variadic(false),
      function_const(false),
      function_volatile(false),
      function_ref_qualifier(FTRQ_NONE)
  {}

  Kind kind;
  bool has_bound;
  bool has_evaluated_bound;
  unsigned long long bound_value;
  std::string bound_text;
  std::vector<RecogToken> bound_tokens;
  std::vector<TypePtr> params;
  bool variadic;
  bool function_const;
  bool function_volatile;
  FunctionTypeRefQualifier function_ref_qualifier;
};

struct Declarator
{
  Declarator() : has_name(false) {}

  bool has_name;
  QualifiedName name;
  std::vector<PtrOperator> prefixes;
  std::vector<DeclaratorSuffix> suffixes;
  std::unique_ptr<Declarator> nested;
};

struct DeclSpec
{
  DeclSpec()
    : is_typedef(false),
      is_extern(false),
      is_static(false),
      is_thread_local(false),
      is_constexpr(false),
      is_inline(false),
      saw_type(false)
  {}

  bool is_typedef;
  bool is_extern;
  bool is_static;
  bool is_thread_local;
  bool is_constexpr;
  bool is_inline;
  bool saw_type;
  TypePtr type;
};

struct ParameterDecl
{
  ParameterDecl() : had_declarator(false) {}

  TypePtr type;
  bool had_declarator;
};

}  // namespace cpp_decl
