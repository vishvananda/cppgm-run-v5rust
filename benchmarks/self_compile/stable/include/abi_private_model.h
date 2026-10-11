#pragma once

// Private ABI encoder model for production mangling and the withheld PA30 encoder.

#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>
#include <utility>
#include <vector>

namespace abi_mangle { namespace model {

struct DependentExpression;
struct FunctionEncoding;
struct Type;

struct SubstitutionKey
{
  enum Kind
  {
    SK_NONE,
    SK_NAMED,
    SK_TYPE,
    SK_TEMPLATE_ENTITY,
    SK_PREFIX,
    SK_TYPE_BUILTIN,
    SK_TYPE_CV,
    SK_TYPE_POINTER,
    SK_TYPE_LVALUE_REFERENCE,
    SK_TYPE_RVALUE_REFERENCE,
    SK_TYPE_ARRAY,
    SK_TYPE_FUNCTION,
    SK_TYPE_MEMBER_POINTER,
    SK_TYPE_TEMPLATE_PARAMETER,
    SK_CLASS_TEMPLATE_SPECIALIZATION,
    SK_TEMPLATE_ARGUMENT_TYPE,
    SK_TEMPLATE_ARGUMENT_VALUE,
    SK_TEMPLATE_ARGUMENT_TEMPLATE,
    SK_FUNCTION_TEMPLATE_PREFIX
  };

  Kind kind = SK_NONE;
  std::uintptr_t id = 0;
  std::string payload;
  std::vector<SubstitutionKey> children;
  mutable std::size_t cached_hash = 0;

  static SubstitutionKey none()
  {
    return SubstitutionKey();
  }

  static SubstitutionKey named(std::string name)
  {
    return make(SK_NAMED, std::move(name));
  }

  static SubstitutionKey type(std::string key)
  {
    return make(SK_TYPE, std::move(key));
  }

  static SubstitutionKey template_entity(std::string key)
  {
    return make(SK_TEMPLATE_ENTITY, std::move(key));
  }

  static SubstitutionKey prefix(std::string key)
  {
    return make(SK_PREFIX, std::move(key));
  }

  static SubstitutionKey type_builtin(std::string code)
  {
    return make(SK_TYPE_BUILTIN, std::move(code));
  }

  static SubstitutionKey type_cv(bool is_const,
                                 bool is_volatile,
                                 SubstitutionKey inner)
  {
    std::string payload;
    if(is_const) {
      payload += 'K';
    }
    if(is_volatile) {
      payload += 'V';
    }
    return make_unary(SK_TYPE_CV, std::move(payload), std::move(inner));
  }

  static SubstitutionKey type_pointer(SubstitutionKey inner)
  {
    return make_unary(SK_TYPE_POINTER, std::string(), std::move(inner));
  }

  static SubstitutionKey type_lvalue_reference(SubstitutionKey inner)
  {
    return make_unary(SK_TYPE_LVALUE_REFERENCE, std::string(), std::move(inner));
  }

  static SubstitutionKey type_rvalue_reference(SubstitutionKey inner)
  {
    return make_unary(SK_TYPE_RVALUE_REFERENCE, std::string(), std::move(inner));
  }

  static SubstitutionKey type_array(const std::string & bound_key,
                                    SubstitutionKey inner)
  {
    return make_unary(SK_TYPE_ARRAY, bound_key, std::move(inner));
  }

  static SubstitutionKey type_function(SubstitutionKey result,
                                       std::vector<SubstitutionKey> params,
                                       bool variadic,
                                       bool lvalue_ref = false,
                                       bool rvalue_ref = false)
  {
    SubstitutionKey key;
    key.kind = SK_TYPE_FUNCTION;
    key.payload = variadic ? "z" : "v";
    if(lvalue_ref) {
      key.payload += "R";
    } else if(rvalue_ref) {
      key.payload += "O";
    } else {
      key.payload += "-";
    }
    key.children.reserve(params.size() + 1);
    key.children.push_back(std::move(result));
    for(std::size_t i = 0; i < params.size(); ++i) {
      key.children.push_back(std::move(params[i]));
    }
    return key;
  }

  static SubstitutionKey type_member_pointer(SubstitutionKey owner,
                                             SubstitutionKey member)
  {
    SubstitutionKey key;
    key.kind = SK_TYPE_MEMBER_POINTER;
    key.children.reserve(2);
    key.children.push_back(std::move(owner));
    key.children.push_back(std::move(member));
    return key;
  }

  static SubstitutionKey type_template_parameter(
      std::size_t index,
      std::uintptr_t scope_id = 0)
  {
    SubstitutionKey key;
    key.kind = SK_TYPE_TEMPLATE_PARAMETER;
    key.id = index;
    if(scope_id != 0) {
      key.payload = std::to_string(scope_id);
    }
    return key;
  }

  static SubstitutionKey class_template_specialization(
      std::uintptr_t template_id,
      std::string fallback_name,
      std::vector<SubstitutionKey> arguments)
  {
    SubstitutionKey key;
    key.kind = SK_CLASS_TEMPLATE_SPECIALIZATION;
    key.id = template_id;
    key.payload = std::move(fallback_name);
    key.children = std::move(arguments);
    return key;
  }

  static SubstitutionKey template_argument_type(SubstitutionKey type)
  {
    return make_unary(SK_TEMPLATE_ARGUMENT_TYPE, std::string(), std::move(type));
  }

  static SubstitutionKey template_argument_value(std::string value)
  {
    return make(SK_TEMPLATE_ARGUMENT_VALUE, std::move(value));
  }

  static SubstitutionKey template_argument_template(std::uintptr_t template_id,
                                                    std::string fallback_name)
  {
    SubstitutionKey key;
    key.kind = SK_TEMPLATE_ARGUMENT_TEMPLATE;
    key.id = template_id;
    key.payload = std::move(fallback_name);
    return key;
  }

  static SubstitutionKey function_template_prefix(std::string name)
  {
    return make(SK_FUNCTION_TEMPLATE_PREFIX, std::move(name));
  }

  bool empty() const
  {
    switch(kind) {
    case SK_NONE:
      return true;
    case SK_NAMED:
    case SK_TYPE:
    case SK_TEMPLATE_ENTITY:
    case SK_PREFIX:
    case SK_TYPE_BUILTIN:
    case SK_TEMPLATE_ARGUMENT_VALUE:
    case SK_FUNCTION_TEMPLATE_PREFIX:
      return payload.empty();
    case SK_CLASS_TEMPLATE_SPECIALIZATION:
    case SK_TEMPLATE_ARGUMENT_TEMPLATE:
      return id == 0 && payload.empty();
    case SK_TYPE_CV:
    case SK_TYPE_POINTER:
    case SK_TYPE_LVALUE_REFERENCE:
    case SK_TYPE_RVALUE_REFERENCE:
    case SK_TYPE_ARRAY:
    case SK_TEMPLATE_ARGUMENT_TYPE:
      return children.size() != 1 || children[0].empty();
    case SK_TYPE_FUNCTION:
      return children.empty() || children[0].empty() || payload.empty();
    case SK_TYPE_MEMBER_POINTER:
      return children.size() != 2 || children[0].empty() || children[1].empty();
    case SK_TYPE_TEMPLATE_PARAMETER:
      return false;
    }
    return true;
  }

  std::string structural_text() const
  {
    std::string out = std::to_string(static_cast<int>(kind));
    out += ':';
    out += std::to_string(id);
    out += ':';
    out += std::to_string(payload.size());
    out += ':';
    out += payload;
    out += '[';
    for(std::size_t i = 0; i < children.size(); ++i) {
      if(i != 0) {
        out += ',';
      }
      out += children[i].structural_text();
    }
    out += ']';
    return out;
  }

  bool operator<(const SubstitutionKey & rhs) const
  {
    if(kind != rhs.kind) {
      return kind < rhs.kind;
    }
    if(payload != rhs.payload) {
      return payload < rhs.payload;
    }
    if(id != rhs.id) {
      return id < rhs.id;
    }
    return children < rhs.children;
  }

  bool operator==(const SubstitutionKey & rhs) const
  {
    return kind == rhs.kind &&
           id == rhs.id &&
           payload == rhs.payload &&
           children == rhs.children;
  }

private:
  static SubstitutionKey make(Kind kind, std::string payload)
  {
    SubstitutionKey key;
    key.kind = kind;
    key.payload = std::move(payload);
    return key;
  }

  static SubstitutionKey make_unary(Kind kind,
                                    std::string payload,
                                    SubstitutionKey inner)
  {
    SubstitutionKey key;
    key.kind = kind;
    key.payload = std::move(payload);
    key.children.push_back(std::move(inner));
    return key;
  }
};

struct SubstitutionSlot
{
  SubstitutionKey ir_key;

  static SubstitutionSlot typed(SubstitutionKey key)
  {
    SubstitutionSlot slot;
    slot.ir_key = std::move(key);
    return slot;
  }

  bool empty() const
  {
    return ir_key.empty();
  }
};

struct OptionalSubstitutionKey
{
  OptionalSubstitutionKey() {}

  OptionalSubstitutionKey(const OptionalSubstitutionKey & rhs)
      : key(rhs.key ? new SubstitutionKey(*rhs.key) : nullptr)
  {
  }

  OptionalSubstitutionKey(OptionalSubstitutionKey && rhs) noexcept
      : key(std::move(rhs.key))
  {
  }

  OptionalSubstitutionKey & operator=(const OptionalSubstitutionKey & rhs)
  {
    if(this == &rhs) {
      return *this;
    }
    key.reset(rhs.key ? new SubstitutionKey(*rhs.key) : nullptr);
    return *this;
  }

  OptionalSubstitutionKey & operator=(OptionalSubstitutionKey && rhs) noexcept
  {
    if(this != &rhs) {
      key = std::move(rhs.key);
    }
    return *this;
  }

  OptionalSubstitutionKey & operator=(SubstitutionKey value)
  {
    if(value.empty()) {
      key.reset();
    } else {
      key.reset(new SubstitutionKey(std::move(value)));
    }
    return *this;
  }

  bool empty() const
  {
    return !key || key->empty();
  }

  const SubstitutionKey & get() const
  {
    return key ? *key : empty_key();
  }

private:
  std::unique_ptr<SubstitutionKey> key;

  static const SubstitutionKey & empty_key()
  {
    static const SubstitutionKey empty;
    return empty;
  }
};

struct SubstitutionSink
{
  virtual ~SubstitutionSink() {}
  virtual bool emit_substitution(const SubstitutionKey & key, std::string & out) = 0;
  virtual void register_substitution(const SubstitutionKey & key) = 0;
  virtual void register_substitution_owned(SubstitutionKey key)
  {
    register_substitution(key);
  }
  virtual bool emit_dependent_parameter_type(const Type & type,
                                             std::string & out);
  virtual bool suppress_template_parameter_type_substitution_in_template_argument()
      const
  {
    return false;
  }
};

struct Type
{
  enum Kind
  {
    TK_INVALID,
    TK_BUILTIN,
    TK_CV,
    TK_POINTER,
    TK_LVALUE_REFERENCE,
    TK_RVALUE_REFERENCE,
    TK_ARRAY,
    TK_FUNCTION,
    TK_MEMBER_POINTER,
    TK_VENDOR_QUALIFIED,
    TK_BUILTIN_TYPE_TRANSFORM,
    TK_PACK_EXPANSION,
    TK_TEMPLATE_PARAMETER,
    TK_NAMED,
    TK_CLASS_TEMPLATE_SPECIALIZATION,
    TK_DECLTYPE_EXPRESSION,
    TK_LAMBDA_CLOSURE
  };

  struct NameComponent
  {
    std::string source_name;
    std::string substitution_name;
    bool std_abbrev = false;

    static NameComponent source(const std::string & name,
                                const std::string & substitution)
    {
      NameComponent component;
      component.source_name = name;
      component.substitution_name = substitution;
      return component;
    }

    static NameComponent std_namespace()
    {
      NameComponent component;
      component.source_name = "std";
      component.std_abbrev = true;
      return component;
    }
  };

  struct ClassTemplateArgument
  {
    enum Kind
    {
      CTAK_INVALID,
      CTAK_TYPE,
      CTAK_INTEGRAL_VALUE,
      CTAK_DEPENDENT_INTEGRAL_VALUE,
      CTAK_DEPENDENT_EXPRESSION,
      CTAK_UNTYPED_INTEGRAL_VALUE,
      CTAK_TEMPLATE_ENTITY,
      CTAK_EXTERNAL_ENTITY,
      CTAK_ARGUMENT_PACK
    };

    struct Metadata
    {
      std::vector<NameComponent> prefix_components;
      std::shared_ptr<Type> template_owner_type;
      std::string template_name;
      std::string template_name_substitution;
      std::string external_entity_symbol;
      std::shared_ptr<Type> external_entity_owner_type;
      std::string external_entity_member_name;
      std::vector<Type> external_entity_parameter_types;
      std::vector<ClassTemplateArgument> pack_arguments;
      std::size_t template_parameter_index = 0;
      bool external_entity_address_of = false;
      bool external_entity_is_member = false;
      bool external_entity_is_function = false;
      bool external_entity_function_const = false;
      bool external_entity_function_volatile = false;
      bool external_entity_function_lvalue_ref = false;
      bool external_entity_function_rvalue_ref = false;
      bool external_entity_function_variadic = false;
      bool template_name_is_template_parameter = false;
    };

    Kind kind = CTAK_INVALID;
    std::shared_ptr<Type> type;
    std::shared_ptr<Type> parameter_type;
    std::shared_ptr<DependentExpression> expression;
    std::shared_ptr<Metadata> metadata;
    long long integral_value = 0;

    static Metadata & ensure_metadata(ClassTemplateArgument & argument)
    {
      if(!argument.metadata || !argument.metadata.unique()) {
        argument.metadata.reset(argument.metadata ?
            new Metadata(*argument.metadata) :
            new Metadata);
      }
      return *argument.metadata;
    }

    static ClassTemplateArgument type_arg(Type type)
    {
      ClassTemplateArgument argument;
      argument.kind = CTAK_TYPE;
      argument.type.reset(new Type(std::move(type)));
      return argument;
    }

    static ClassTemplateArgument integral_value_arg(Type type,
                                                    long long value)
    {
      ClassTemplateArgument argument;
      argument.kind = CTAK_INTEGRAL_VALUE;
      argument.type.reset(new Type(std::move(type)));
      argument.integral_value = value;
      return argument;
    }

    static ClassTemplateArgument dependent_integral_value_arg(
        Type parameter_type,
        Type value_type,
        long long value)
    {
      ClassTemplateArgument argument;
      argument.kind = CTAK_DEPENDENT_INTEGRAL_VALUE;
      argument.parameter_type.reset(new Type(std::move(parameter_type)));
      argument.type.reset(new Type(std::move(value_type)));
      argument.integral_value = value;
      return argument;
    }

    static ClassTemplateArgument dependent_untyped_integral_value_arg(
        Type parameter_type,
        long long value)
    {
      ClassTemplateArgument argument;
      argument.kind = CTAK_DEPENDENT_INTEGRAL_VALUE;
      argument.parameter_type.reset(new Type(std::move(parameter_type)));
      argument.integral_value = value;
      return argument;
    }

    static ClassTemplateArgument untyped_integral_value_arg(long long value)
    {
      ClassTemplateArgument argument;
      argument.kind = CTAK_UNTYPED_INTEGRAL_VALUE;
      argument.integral_value = value;
      return argument;
    }

    static ClassTemplateArgument dependent_expression_arg(
        DependentExpression expression);

    static ClassTemplateArgument template_entity_arg(
        std::vector<NameComponent> prefix_components,
        std::string name,
        std::string substitution)
    {
      ClassTemplateArgument argument;
      argument.kind = CTAK_TEMPLATE_ENTITY;
      Metadata & metadata = ensure_metadata(argument);
      metadata.prefix_components = std::move(prefix_components);
      metadata.template_name = std::move(name);
      metadata.template_name_substitution = std::move(substitution);
      return argument;
    }

    static ClassTemplateArgument member_template_entity_arg(
        Type owner,
        std::string name,
        std::string substitution)
    {
      ClassTemplateArgument argument;
      argument.kind = CTAK_TEMPLATE_ENTITY;
      Metadata & metadata = ensure_metadata(argument);
      metadata.template_owner_type.reset(new Type(std::move(owner)));
      metadata.template_name = std::move(name);
      metadata.template_name_substitution = std::move(substitution);
      return argument;
    }

    static ClassTemplateArgument template_parameter_template_arg(
        std::size_t template_parameter_index)
    {
      ClassTemplateArgument argument;
      argument.kind = CTAK_TEMPLATE_ENTITY;
      Metadata & metadata = ensure_metadata(argument);
      metadata.template_name_is_template_parameter = true;
      metadata.template_parameter_index = template_parameter_index;
      return argument;
    }

    static ClassTemplateArgument external_entity_arg(std::string symbol,
                                                     bool address_of)
    {
      ClassTemplateArgument argument;
      argument.kind = CTAK_EXTERNAL_ENTITY;
      Metadata & metadata = ensure_metadata(argument);
      metadata.external_entity_symbol = std::move(symbol);
      metadata.external_entity_address_of = address_of;
      return argument;
    }

    static ClassTemplateArgument external_member_entity_arg(
        std::string symbol,
        bool address_of,
        Type owner_type,
        std::string member_name,
        std::vector<Type> parameter_types,
        bool is_function,
        bool function_const,
        bool function_volatile,
        bool function_lvalue_ref,
        bool function_rvalue_ref,
        bool function_variadic)
    {
      ClassTemplateArgument argument = external_entity_arg(symbol, address_of);
      Metadata & metadata = ensure_metadata(argument);
      metadata.external_entity_owner_type.reset(new Type(std::move(owner_type)));
      metadata.external_entity_member_name = std::move(member_name);
      metadata.external_entity_parameter_types = std::move(parameter_types);
      metadata.external_entity_is_member = true;
      metadata.external_entity_is_function = is_function;
      metadata.external_entity_function_const = function_const;
      metadata.external_entity_function_volatile = function_volatile;
      metadata.external_entity_function_lvalue_ref = function_lvalue_ref;
      metadata.external_entity_function_rvalue_ref = function_rvalue_ref;
      metadata.external_entity_function_variadic = function_variadic;
      return argument;
    }

    static ClassTemplateArgument argument_pack(
        std::vector<ClassTemplateArgument> arguments)
    {
      ClassTemplateArgument argument;
      argument.kind = CTAK_ARGUMENT_PACK;
      Metadata & metadata = ensure_metadata(argument);
      metadata.pack_arguments = std::move(arguments);
      return argument;
    }
  };

  struct SubstitutionMetadata
  {
    SubstitutionKey key;
  };

  struct LambdaMetadata
  {
    std::string context_fragment;
    std::vector<SubstitutionSlot> context_substitution_slots;
    std::shared_ptr<FunctionEncoding> context_function;
    std::string source_name;
    std::vector<std::string> namespace_qualifiers;
    std::string discriminator;
  };

  struct NameMetadata
  {
    std::size_t template_name_parameter_index = 0;
    std::vector<NameComponent> prefix_components;
    std::string template_name;
    std::string template_name_substitution;
    OptionalSubstitutionKey template_name_ir_substitution;
    std::vector<ClassTemplateArgument> template_arguments;
    std::string standard_substitution;
    bool standard_substitution_includes_arguments = false;
    bool register_member_expression_template_name = false;
    bool template_name_is_template_parameter = false;
  };

  Kind kind = TK_INVALID;
  char builtin_code[3] = {0, 0, 0};
  std::string vendor_qualifier_name;
  std::string builtin_transform_name;
  std::string array_bound;
  std::string array_substitution_bound_key;
  std::size_t template_parameter_index = 0;
  bool cv_const = false;
  bool cv_volatile = false;
  bool variadic = false;
  bool function_lvalue_ref = false;
  bool function_rvalue_ref = false;
  std::shared_ptr<Type> inner;
  std::shared_ptr<Type> owner;
  std::shared_ptr<Type> name_owner;
  std::shared_ptr<DependentExpression> expression;
  std::vector<Type> params;
  std::shared_ptr<SubstitutionMetadata> substitution;
  std::shared_ptr<LambdaMetadata> lambda;
  std::shared_ptr<NameMetadata> name;

  static NameMetadata & ensure_name_metadata(Type & type)
  {
    if(!type.name || !type.name.unique()) {
      type.name.reset(type.name ? new NameMetadata(*type.name) :
                                  new NameMetadata);
    }
    return *type.name;
  }

  static Type builtin(const std::string & code)
  {
    Type type;
    type.kind = TK_BUILTIN;
    const std::size_t count = code.size() < 2 ? code.size() : 2;
    for(std::size_t i = 0; i < count; ++i) {
      type.builtin_code[i] = code[i];
    }
    return type;
  }

  static Type cv(bool is_const, bool is_volatile, Type inner)
  {
    Type type;
    type.kind = TK_CV;
    type.cv_const = is_const;
    type.cv_volatile = is_volatile;
    type.inner.reset(new Type(std::move(inner)));
    return type;
  }

  static Type pointer(Type inner)
  {
    Type type;
    type.kind = TK_POINTER;
    type.inner.reset(new Type(std::move(inner)));
    return type;
  }

  static Type lvalue_reference(Type inner)
  {
    Type type;
    type.kind = TK_LVALUE_REFERENCE;
    type.inner.reset(new Type(std::move(inner)));
    return type;
  }

  static Type rvalue_reference(Type inner)
  {
    Type type;
    type.kind = TK_RVALUE_REFERENCE;
    type.inner.reset(new Type(std::move(inner)));
    return type;
  }

  static Type array(std::string bound, Type inner)
  {
    Type type;
    type.kind = TK_ARRAY;
    type.array_bound = std::move(bound);
    type.inner.reset(new Type(std::move(inner)));
    return type;
  }

  static Type array(std::string bound,
                    std::string substitution_bound_key,
                    Type inner)
  {
    Type type = array(std::move(bound), std::move(inner));
    type.array_substitution_bound_key = std::move(substitution_bound_key);
    return type;
  }

  static Type function(Type result,
                       std::vector<Type> params,
                       bool variadic,
                       bool lvalue_ref = false,
                       bool rvalue_ref = false)
  {
    Type type;
    type.kind = TK_FUNCTION;
    type.inner.reset(new Type(std::move(result)));
    type.params = std::move(params);
    type.variadic = variadic;
    type.function_lvalue_ref = lvalue_ref;
    type.function_rvalue_ref = rvalue_ref;
    return type;
  }

  static Type member_pointer(Type owner, Type member)
  {
    Type type;
    type.kind = TK_MEMBER_POINTER;
    type.owner.reset(new Type(std::move(owner)));
    type.inner.reset(new Type(std::move(member)));
    return type;
  }

  static Type vendor_qualified(std::string name, Type inner)
  {
    Type type;
    type.kind = TK_VENDOR_QUALIFIED;
    type.vendor_qualifier_name = std::move(name);
    type.inner.reset(new Type(std::move(inner)));
    return type;
  }

  static Type builtin_type_transform(std::string name,
                                     Type argument)
  {
    Type type;
    type.kind = TK_BUILTIN_TYPE_TRANSFORM;
    type.builtin_transform_name = std::move(name);
    type.inner.reset(new Type(std::move(argument)));
    return type;
  }

  static Type pack_expansion(Type pattern)
  {
    Type type;
    type.kind = TK_PACK_EXPANSION;
    type.inner.reset(new Type(std::move(pattern)));
    return type;
  }

  static Type template_parameter(std::size_t index)
  {
    Type type;
    type.kind = TK_TEMPLATE_PARAMETER;
    type.template_parameter_index = index;
    return type;
  }

  static Type named_type(std::vector<NameComponent> prefix_components,
                         std::string name,
                         std::string substitution)
  {
    Type type;
    type.kind = TK_NAMED;
    NameMetadata & metadata = ensure_name_metadata(type);
    metadata.prefix_components = std::move(prefix_components);
    metadata.template_name = std::move(name);
    metadata.template_name_substitution = std::move(substitution);
    return type;
  }

  static Type member_named_type(Type owner,
                                std::string name,
                                std::string substitution)
  {
    Type type;
    type.kind = TK_NAMED;
    type.name_owner.reset(new Type(std::move(owner)));
    NameMetadata & metadata = ensure_name_metadata(type);
    metadata.template_name = std::move(name);
    metadata.template_name_substitution = std::move(substitution);
    return type;
  }

  static Type class_template_specialization(
      std::vector<NameComponent> prefix_components,
      std::string template_name,
      std::string template_name_substitution,
      std::vector<ClassTemplateArgument> arguments,
      std::string standard_substitution,
      bool standard_substitution_includes_arguments)
  {
    Type type;
    type.kind = TK_CLASS_TEMPLATE_SPECIALIZATION;
    NameMetadata & metadata = ensure_name_metadata(type);
    metadata.prefix_components = std::move(prefix_components);
    metadata.template_name = std::move(template_name);
    metadata.template_name_substitution = std::move(template_name_substitution);
    metadata.template_arguments = std::move(arguments);
    metadata.standard_substitution = std::move(standard_substitution);
    metadata.standard_substitution_includes_arguments =
        standard_substitution_includes_arguments;
    return type;
  }

  static Type template_parameter_class_template_specialization(
      std::size_t template_parameter_index,
      std::vector<ClassTemplateArgument> arguments)
  {
    Type type;
    type.kind = TK_CLASS_TEMPLATE_SPECIALIZATION;
    NameMetadata & metadata = ensure_name_metadata(type);
    metadata.template_name_is_template_parameter = true;
    metadata.template_name_parameter_index = template_parameter_index;
    metadata.template_arguments = std::move(arguments);
    return type;
  }

  static Type member_class_template_specialization(
      Type owner,
      std::string template_name,
      std::string template_name_substitution,
      std::vector<ClassTemplateArgument> arguments)
  {
    Type type;
    type.kind = TK_CLASS_TEMPLATE_SPECIALIZATION;
    type.name_owner.reset(new Type(std::move(owner)));
    NameMetadata & metadata = ensure_name_metadata(type);
    metadata.template_name = std::move(template_name);
    metadata.template_name_substitution = std::move(template_name_substitution);
    metadata.template_arguments = std::move(arguments);
    return type;
  }

  static Type lambda_closure(
      const std::string & context_fragment,
      const std::vector<SubstitutionSlot> & context_substitution_slots,
      const std::shared_ptr<FunctionEncoding> & context_function,
      const std::vector<Type> & signature_parameter_types,
      const std::string & discriminator,
      const std::string & source_name = std::string(),
      const std::vector<std::string> & namespace_qualifiers =
          std::vector<std::string>())
  {
    Type type;
    type.kind = TK_LAMBDA_CLOSURE;
    type.lambda.reset(new LambdaMetadata());
    type.lambda->context_fragment = context_fragment;
    type.lambda->context_substitution_slots = context_substitution_slots;
    type.lambda->context_function = context_function;
    type.lambda->source_name = source_name;
    type.lambda->namespace_qualifiers = namespace_qualifiers;
    type.params = signature_parameter_types;
    type.lambda->discriminator = discriminator;
    return type;
  }
};

struct TemplateArgument
{
  enum Kind
  {
    TAK_INVALID,
    TAK_TYPE,
    TAK_INTEGRAL_VALUE,
    TAK_DEPENDENT_INTEGRAL_VALUE,
    TAK_DEPENDENT_EXPRESSION,
    TAK_UNTYPED_INTEGRAL_VALUE,
    TAK_TEMPLATE_ENTITY,
    TAK_EXTERNAL_ENTITY,
    TAK_ARGUMENT_PACK
  };

  struct Metadata
  {
    std::vector<Type::NameComponent> prefix_components;
    std::shared_ptr<Type> template_owner_type;
    std::string template_name;
    std::string template_name_substitution;
    std::string external_entity_symbol;
    std::shared_ptr<Type> external_entity_owner_type;
    std::string external_entity_member_name;
    std::vector<Type> external_entity_parameter_types;
    std::vector<TemplateArgument> pack_arguments;
    std::size_t template_parameter_index = 0;
    bool external_entity_address_of = false;
    bool external_entity_is_member = false;
    bool external_entity_is_function = false;
    bool external_entity_function_const = false;
    bool external_entity_function_volatile = false;
    bool external_entity_function_lvalue_ref = false;
    bool external_entity_function_rvalue_ref = false;
    bool external_entity_function_variadic = false;
    bool template_name_is_template_parameter = false;
  };

  Kind kind = TAK_INVALID;
  std::shared_ptr<Type> value_type;
  std::shared_ptr<Type> parameter_type;
  std::shared_ptr<DependentExpression> expression;
  std::shared_ptr<Metadata> metadata;
  long long integral_value = 0;

  static Metadata & ensure_metadata(TemplateArgument & argument)
  {
    if(!argument.metadata || !argument.metadata.unique()) {
      argument.metadata.reset(argument.metadata ?
          new Metadata(*argument.metadata) :
          new Metadata);
    }
    return *argument.metadata;
  }

  static TemplateArgument type_arg(Type type)
  {
    TemplateArgument argument;
    argument.kind = TAK_TYPE;
    argument.value_type.reset(new Type(std::move(type)));
    return argument;
  }

  static TemplateArgument integral_value_arg(Type type, long long value)
  {
    TemplateArgument argument;
    argument.kind = TAK_INTEGRAL_VALUE;
    argument.value_type.reset(new Type(std::move(type)));
    argument.integral_value = value;
    return argument;
  }

  static TemplateArgument dependent_integral_value_arg(
      Type parameter_type,
      Type value_type,
      long long value)
  {
    TemplateArgument argument;
    argument.kind = TAK_DEPENDENT_INTEGRAL_VALUE;
    argument.parameter_type.reset(new Type(std::move(parameter_type)));
    argument.value_type.reset(new Type(std::move(value_type)));
    argument.integral_value = value;
    return argument;
  }

  static TemplateArgument dependent_untyped_integral_value_arg(
      Type parameter_type,
      long long value)
  {
    TemplateArgument argument;
    argument.kind = TAK_DEPENDENT_INTEGRAL_VALUE;
    argument.parameter_type.reset(new Type(std::move(parameter_type)));
    argument.integral_value = value;
    return argument;
  }

  static TemplateArgument untyped_integral_value_arg(long long value)
  {
    TemplateArgument argument;
    argument.kind = TAK_UNTYPED_INTEGRAL_VALUE;
    argument.integral_value = value;
    return argument;
  }

  static TemplateArgument dependent_expression_arg(
      DependentExpression expression);

  static TemplateArgument template_entity_arg(
      std::vector<Type::NameComponent> prefix_components,
      std::string name,
      std::string substitution)
  {
    TemplateArgument argument;
    argument.kind = TAK_TEMPLATE_ENTITY;
    Metadata & metadata = ensure_metadata(argument);
    metadata.prefix_components = std::move(prefix_components);
    metadata.template_name = std::move(name);
    metadata.template_name_substitution = std::move(substitution);
    return argument;
  }

  static TemplateArgument member_template_entity_arg(
      Type owner,
      std::string name,
      std::string substitution)
  {
    TemplateArgument argument;
    argument.kind = TAK_TEMPLATE_ENTITY;
    Metadata & metadata = ensure_metadata(argument);
    metadata.template_owner_type.reset(new Type(std::move(owner)));
    metadata.template_name = std::move(name);
    metadata.template_name_substitution = std::move(substitution);
    return argument;
  }

  static TemplateArgument template_parameter_template_arg(
      std::size_t template_parameter_index)
  {
    TemplateArgument argument;
    argument.kind = TAK_TEMPLATE_ENTITY;
    Metadata & metadata = ensure_metadata(argument);
    metadata.template_name_is_template_parameter = true;
    metadata.template_parameter_index = template_parameter_index;
    return argument;
  }

  static TemplateArgument external_entity_arg(std::string symbol,
                                              bool address_of)
  {
    TemplateArgument argument;
    argument.kind = TAK_EXTERNAL_ENTITY;
    Metadata & metadata = ensure_metadata(argument);
    metadata.external_entity_symbol = std::move(symbol);
    metadata.external_entity_address_of = address_of;
    return argument;
  }

  static TemplateArgument external_member_entity_arg(
      std::string symbol,
      bool address_of,
      Type owner_type,
      std::string member_name,
      std::vector<Type> parameter_types,
      bool is_function,
      bool function_const,
      bool function_volatile,
      bool function_lvalue_ref,
      bool function_rvalue_ref,
      bool function_variadic)
  {
    TemplateArgument argument = external_entity_arg(symbol, address_of);
    Metadata & metadata = ensure_metadata(argument);
    metadata.external_entity_owner_type.reset(new Type(std::move(owner_type)));
    metadata.external_entity_member_name = std::move(member_name);
    metadata.external_entity_parameter_types = std::move(parameter_types);
    metadata.external_entity_is_member = true;
    metadata.external_entity_is_function = is_function;
    metadata.external_entity_function_const = function_const;
    metadata.external_entity_function_volatile = function_volatile;
    metadata.external_entity_function_lvalue_ref = function_lvalue_ref;
    metadata.external_entity_function_rvalue_ref = function_rvalue_ref;
    metadata.external_entity_function_variadic = function_variadic;
    return argument;
  }

  static TemplateArgument argument_pack(
      std::vector<TemplateArgument> arguments)
  {
    TemplateArgument argument;
    argument.kind = TAK_ARGUMENT_PACK;
    Metadata & metadata = ensure_metadata(argument);
    metadata.pack_arguments = std::move(arguments);
    return argument;
  }
};

struct DependentExpression
{
  enum Kind
  {
    EK_INVALID,
    EK_TEMPLATE_PARAMETER,
    EK_FUNCTION_PARAMETER,
    EK_LITERAL,
    EK_INTEGRAL_VALUE,
    EK_MEMBER,
    EK_OBJECT_MEMBER,
    EK_UNARY,
    EK_BINARY,
    EK_CONDITIONAL,
    EK_PACK_EXPANSION,
    EK_CALL,
    EK_CONVERSION,
    EK_TEMPLATE_ID,
    EK_TYPE_TRAIT,
    EK_SIZEOF_TYPE,
    EK_EXTERNAL_ENTITY
  };

  Kind kind = EK_INVALID;
  std::string text;
  std::string op_code;
  std::shared_ptr<Type> owner_type;
  bool close_member_owner = false;
  std::shared_ptr<DependentExpression> inner;
  std::vector<DependentExpression> arguments;
  std::vector<TemplateArgument> template_arguments;
  std::vector<Type> type_arguments;
  std::size_t template_parameter_index = 0;
  long long integral_value = 0;
  bool suppress_template_prefix_substitution = false;
  bool suppress_member_owner_prefix = false;
  bool external_entity_address_of = false;

  static DependentExpression template_parameter(std::size_t index)
  {
    DependentExpression expression;
    expression.kind = EK_TEMPLATE_PARAMETER;
    expression.template_parameter_index = index;
    return expression;
  }

  static DependentExpression function_parameter(std::size_t index)
  {
    DependentExpression expression;
    expression.kind = EK_FUNCTION_PARAMETER;
    expression.template_parameter_index = index;
    return expression;
  }

  static DependentExpression literal(std::string text)
  {
    DependentExpression expression;
    expression.kind = EK_LITERAL;
    expression.text = std::move(text);
    return expression;
  }

  static DependentExpression typed_integral_value(Type type,
                                                  long long value)
  {
    DependentExpression expression;
    expression.kind = EK_INTEGRAL_VALUE;
    expression.owner_type.reset(new Type(std::move(type)));
    expression.integral_value = value;
    return expression;
  }

  static DependentExpression member(Type owner,
                                    bool close_owner,
                                    std::string name)
  {
    DependentExpression expression;
    expression.kind = EK_MEMBER;
    expression.owner_type.reset(new Type(std::move(owner)));
    expression.close_member_owner = close_owner;
    expression.text = std::move(name);
    return expression;
  }

  static DependentExpression object_member(
      std::string op_code,
      DependentExpression object,
      std::string name,
      std::vector<TemplateArgument> template_arguments)
  {
    DependentExpression expression;
    expression.kind = EK_OBJECT_MEMBER;
    expression.op_code = std::move(op_code);
    expression.inner.reset(new DependentExpression(std::move(object)));
    expression.text = std::move(name);
    expression.template_arguments = std::move(template_arguments);
    return expression;
  }

  static DependentExpression unary(std::string op_code,
                                   DependentExpression inner)
  {
    DependentExpression expression;
    expression.kind = EK_UNARY;
    expression.op_code = std::move(op_code);
    expression.inner.reset(new DependentExpression(std::move(inner)));
    return expression;
  }

  static DependentExpression pack_expansion(DependentExpression inner)
  {
    DependentExpression expression;
    expression.kind = EK_PACK_EXPANSION;
    expression.inner.reset(new DependentExpression(std::move(inner)));
    return expression;
  }

  static DependentExpression binary(std::string op_code,
                                    DependentExpression left,
                                    DependentExpression right)
  {
    DependentExpression expression;
    expression.kind = EK_BINARY;
    expression.op_code = std::move(op_code);
    expression.inner.reset(new DependentExpression(std::move(left)));
    expression.arguments.push_back(std::move(right));
    return expression;
  }

  static DependentExpression conditional(DependentExpression condition,
                                         DependentExpression true_expr,
                                         DependentExpression false_expr)
  {
    DependentExpression expression;
    expression.kind = EK_CONDITIONAL;
    expression.inner.reset(new DependentExpression(std::move(condition)));
    expression.arguments.push_back(std::move(true_expr));
    expression.arguments.push_back(std::move(false_expr));
    return expression;
  }

  static DependentExpression call(DependentExpression callee,
                                  std::vector<DependentExpression> arguments)
  {
    DependentExpression expression;
    expression.kind = EK_CALL;
    expression.inner.reset(new DependentExpression(std::move(callee)));
    expression.arguments = std::move(arguments);
    return expression;
  }

  static DependentExpression conversion(
      Type type,
      std::vector<DependentExpression> arguments)
  {
    DependentExpression expression;
    expression.kind = EK_CONVERSION;
    expression.op_code = "cv";
    expression.owner_type.reset(new Type(std::move(type)));
    expression.arguments = std::move(arguments);
    return expression;
  }

  static DependentExpression cast(
      std::string op_code,
      Type type,
      DependentExpression argument)
  {
    DependentExpression expression;
    expression.kind = EK_CONVERSION;
    expression.op_code = std::move(op_code);
    expression.owner_type.reset(new Type(std::move(type)));
    expression.arguments.push_back(std::move(argument));
    return expression;
  }

  static DependentExpression template_id(
      std::string name,
      std::vector<TemplateArgument> arguments)
  {
    DependentExpression expression;
    expression.kind = EK_TEMPLATE_ID;
    expression.text = std::move(name);
    expression.template_arguments = std::move(arguments);
    return expression;
  }

  static DependentExpression type_trait(
      std::string name,
      std::vector<Type> arguments)
  {
    DependentExpression expression;
    expression.kind = EK_TYPE_TRAIT;
    expression.text = std::move(name);
    expression.type_arguments = std::move(arguments);
    return expression;
  }

  static DependentExpression sizeof_type(Type type)
  {
    DependentExpression expression;
    expression.kind = EK_SIZEOF_TYPE;
    expression.owner_type.reset(new Type(std::move(type)));
    return expression;
  }

  static DependentExpression external_entity(std::string symbol,
                                             bool address_of)
  {
    DependentExpression expression;
    expression.kind = EK_EXTERNAL_ENTITY;
    expression.text = std::move(symbol);
    expression.external_entity_address_of = address_of;
    return expression;
  }
};

inline Type::ClassTemplateArgument
Type::ClassTemplateArgument::dependent_expression_arg(
    DependentExpression expression)
{
  ClassTemplateArgument argument;
  argument.kind = CTAK_DEPENDENT_EXPRESSION;
  argument.expression.reset(new DependentExpression(std::move(expression)));
  return argument;
}

inline TemplateArgument TemplateArgument::dependent_expression_arg(
    DependentExpression expression)
{
  TemplateArgument argument;
  argument.kind = TAK_DEPENDENT_EXPRESSION;
  argument.expression.reset(new DependentExpression(std::move(expression)));
  return argument;
}

struct FunctionNameComponent
{
  std::string source_name;
  std::string substitution_name;
  std::string complete_substitution_name;
  OptionalSubstitutionKey ir_substitution_key;
  OptionalSubstitutionKey complete_ir_substitution_key;
  std::string standard_substitution;
  bool std_abbrev = false;
  bool standard_substitution_includes_arguments = false;
  std::vector<TemplateArgument> template_arguments;

  static FunctionNameComponent source(std::string name,
                                      std::string substitution)
  {
    FunctionNameComponent component;
    component.source_name = std::move(name);
    component.substitution_name = std::move(substitution);
    return component;
  }

  static FunctionNameComponent std_namespace()
  {
    FunctionNameComponent component;
    component.source_name = "std";
    component.std_abbrev = true;
    return component;
  }

  static FunctionNameComponent template_component(
      std::string name,
      std::string substitution,
      std::string complete_substitution,
      std::vector<TemplateArgument> arguments,
      std::string standard_substitution,
      bool standard_substitution_includes_arguments)
  {
    FunctionNameComponent component;
    component.source_name = std::move(name);
    component.substitution_name = std::move(substitution);
    component.complete_substitution_name = std::move(complete_substitution);
    component.template_arguments = std::move(arguments);
    component.standard_substitution = std::move(standard_substitution);
    component.standard_substitution_includes_arguments =
        standard_substitution_includes_arguments;
    return component;
  }
};

enum FunctionOperatorTerminal
{
  FUNCTION_OPERATOR_NONE,
  FUNCTION_OPERATOR_NEW,
  FUNCTION_OPERATOR_NEW_ARRAY,
  FUNCTION_OPERATOR_DELETE,
  FUNCTION_OPERATOR_DELETE_ARRAY,
  FUNCTION_OPERATOR_UNARY_PLUS,
  FUNCTION_OPERATOR_PLUS,
  FUNCTION_OPERATOR_UNARY_MINUS,
  FUNCTION_OPERATOR_MINUS,
  FUNCTION_OPERATOR_ADDRESS_OF,
  FUNCTION_OPERATOR_BIT_AND,
  FUNCTION_OPERATOR_DEREFERENCE,
  FUNCTION_OPERATOR_MULTIPLY,
  FUNCTION_OPERATOR_DIVIDE,
  FUNCTION_OPERATOR_REMAINDER,
  FUNCTION_OPERATOR_BIT_OR,
  FUNCTION_OPERATOR_BIT_XOR,
  FUNCTION_OPERATOR_ASSIGN,
  FUNCTION_OPERATOR_PLUS_ASSIGN,
  FUNCTION_OPERATOR_MINUS_ASSIGN,
  FUNCTION_OPERATOR_MULTIPLY_ASSIGN,
  FUNCTION_OPERATOR_DIVIDE_ASSIGN,
  FUNCTION_OPERATOR_REMAINDER_ASSIGN,
  FUNCTION_OPERATOR_BIT_AND_ASSIGN,
  FUNCTION_OPERATOR_BIT_OR_ASSIGN,
  FUNCTION_OPERATOR_BIT_XOR_ASSIGN,
  FUNCTION_OPERATOR_SHIFT_LEFT,
  FUNCTION_OPERATOR_SHIFT_RIGHT,
  FUNCTION_OPERATOR_SHIFT_LEFT_ASSIGN,
  FUNCTION_OPERATOR_SHIFT_RIGHT_ASSIGN,
  FUNCTION_OPERATOR_EQUAL,
  FUNCTION_OPERATOR_NOT_EQUAL,
  FUNCTION_OPERATOR_LESS,
  FUNCTION_OPERATOR_GREATER,
  FUNCTION_OPERATOR_LESS_EQUAL,
  FUNCTION_OPERATOR_GREATER_EQUAL,
  FUNCTION_OPERATOR_LOGICAL_NOT,
  FUNCTION_OPERATOR_BIT_NOT,
  FUNCTION_OPERATOR_LOGICAL_AND,
  FUNCTION_OPERATOR_LOGICAL_OR,
  FUNCTION_OPERATOR_INCREMENT,
  FUNCTION_OPERATOR_DECREMENT,
  FUNCTION_OPERATOR_COMMA,
  FUNCTION_OPERATOR_MEMBER_POINTER,
  FUNCTION_OPERATOR_ARROW,
  FUNCTION_OPERATOR_CALL,
  FUNCTION_OPERATOR_INDEX,
  FUNCTION_OPERATOR_LITERAL
};

struct FunctionEncoding
{
  struct LambdaMetadata
  {
    std::string context_fragment;
    std::vector<SubstitutionSlot> context_substitution_slots;
    std::shared_ptr<FunctionEncoding> context_function;
    std::string source_name;
    std::vector<std::string> namespace_qualifiers;
    std::vector<Type> signature_parameter_types;
    std::string discriminator;
  };

  std::string name_fragment;
  std::vector<FunctionNameComponent> name_components;
  std::string terminal_fragment;
  std::string terminal_source_name;
  FunctionOperatorTerminal operator_terminal = FUNCTION_OPERATOR_NONE;
  std::string operator_literal_suffix;
  std::shared_ptr<Type> conversion_type;
  bool has_conversion_type = false;
  std::shared_ptr<Type> result_type;
  bool has_result_type = false;
  std::vector<TemplateArgument> template_arguments;
  OptionalSubstitutionKey template_prefix_key;
  std::vector<std::string> abi_tags;
  std::vector<Type> parameter_types;
  std::shared_ptr<LambdaMetadata> lambda;
  bool variadic = false;
  bool nested_const = false;
  bool nested_volatile = false;
  bool nested_lvalue_ref = false;
  bool nested_rvalue_ref = false;

  static LambdaMetadata & ensure_lambda_metadata(FunctionEncoding & function)
  {
    if(!function.lambda || !function.lambda.unique()) {
      function.lambda.reset(function.lambda ?
          new LambdaMetadata(*function.lambda) :
          new LambdaMetadata);
    }
    return *function.lambda;
  }
};

struct LocalContext
{
  std::string context_fragment;
  std::vector<SubstitutionSlot> context_substitution_slots;
  std::shared_ptr<FunctionEncoding> context_function;

  static LocalContext raw(const std::string & fragment,
                          const std::vector<SubstitutionSlot> & slots =
                              std::vector<SubstitutionSlot>())
  {
    LocalContext context;
    context.context_fragment = fragment;
    context.context_substitution_slots = slots;
    return context;
  }

  static LocalContext function(const FunctionEncoding & function)
  {
    LocalContext context;
    context.context_function.reset(new FunctionEncoding(function));
    return context;
  }
};

enum AbiEntityKind
{
  ABI_ENTITY_FUNCTION,
  ABI_ENTITY_VARIABLE,
  ABI_ENTITY_SYMBOL
};

struct AbiEntity
{
  AbiEntityKind kind = ABI_ENTITY_VARIABLE;
  FunctionEncoding function;
  std::string qualified_name;
  bool internal_linkage = false;
};

enum AbiFactKind
{
  ABI_FACT_TYPE,
  ABI_FACT_TEMPLATE_ARGUMENT,
  ABI_FACT_EXPRESSION,
  ABI_FACT_LOCAL_CONTEXT,
  ABI_FACT_ENTITY
};

struct AbiFact
{
  AbiFactKind kind = ABI_FACT_TYPE;
  std::string id;
  Type type;
  TemplateArgument template_argument;
  DependentExpression expression;
  LocalContext context;
  FunctionEncoding context_function;
  AbiEntity entity;
};

enum AbiMangleTargetKind
{
  ABI_MANGLE_NONE,
  ABI_MANGLE_TYPE,
  ABI_MANGLE_FUNCTION,
  ABI_MANGLE_VARIABLE,
  ABI_MANGLE_TYPEINFO,
  ABI_MANGLE_VTABLE,
  ABI_MANGLE_VTT,
  ABI_MANGLE_CONSTRUCTION_VTABLE,
  ABI_MANGLE_THREAD_LOCAL_WRAPPER,
  ABI_MANGLE_THUNK,
  ABI_MANGLE_VIRTUAL_BASE_THUNK
};

struct AbiMangleTarget
{
  AbiMangleTargetKind kind = ABI_MANGLE_NONE;
  Type type;
  Type base_type;
  FunctionEncoding function;
  std::string qualified_name;
  unsigned long long base_offset = 0;
  long long this_adjust = 0;
  bool has_result_adjust = false;
  long long result_adjust = 0;
  long long vcall_offset = 0;
  bool c_linkage = false;
};

struct AbiFactCase
{
  std::string label;
  std::vector<AbiFact> facts;
  AbiMangleTarget target;
};

struct AbiFactFile
{
  std::vector<AbiFactCase> cases;
};

}  // namespace model

inline const char * unnamed_local_type_source_name()
{
  return "__cppgm_unnamed_local_type";
}

inline bool is_unnamed_local_type_source_name(const std::string & name)
{
  return name == unnamed_local_type_source_name();
}

using model::SubstitutionKey;
using model::SubstitutionSlot;
using model::OptionalSubstitutionKey;
using model::SubstitutionSink;
using model::Type;
using model::TemplateArgument;
using model::DependentExpression;
using model::FunctionNameComponent;
using model::FunctionEncoding;
using model::FunctionOperatorTerminal;
using model::AbiMangleTarget;
using model::AbiMangleTargetKind;
using model::LocalContext;
using model::AbiEntity;
using model::AbiEntityKind;
using model::AbiFactKind;
using model::ABI_ENTITY_FUNCTION;
using model::ABI_ENTITY_VARIABLE;
using model::ABI_ENTITY_SYMBOL;
using model::ABI_FACT_TYPE;
using model::ABI_FACT_TEMPLATE_ARGUMENT;
using model::ABI_FACT_EXPRESSION;
using model::ABI_FACT_LOCAL_CONTEXT;
using model::ABI_FACT_ENTITY;
using model::ABI_MANGLE_NONE;
using model::ABI_MANGLE_TYPE;
using model::ABI_MANGLE_FUNCTION;
using model::ABI_MANGLE_VARIABLE;
using model::ABI_MANGLE_TYPEINFO;
using model::ABI_MANGLE_VTABLE;
using model::ABI_MANGLE_VTT;
using model::ABI_MANGLE_CONSTRUCTION_VTABLE;
using model::ABI_MANGLE_THREAD_LOCAL_WRAPPER;
using model::ABI_MANGLE_THUNK;
using model::ABI_MANGLE_VIRTUAL_BASE_THUNK;
using model::FUNCTION_OPERATOR_NONE;
using model::FUNCTION_OPERATOR_NEW;
using model::FUNCTION_OPERATOR_NEW_ARRAY;
using model::FUNCTION_OPERATOR_DELETE;
using model::FUNCTION_OPERATOR_DELETE_ARRAY;
using model::FUNCTION_OPERATOR_UNARY_PLUS;
using model::FUNCTION_OPERATOR_PLUS;
using model::FUNCTION_OPERATOR_UNARY_MINUS;
using model::FUNCTION_OPERATOR_MINUS;
using model::FUNCTION_OPERATOR_ADDRESS_OF;
using model::FUNCTION_OPERATOR_BIT_AND;
using model::FUNCTION_OPERATOR_DEREFERENCE;
using model::FUNCTION_OPERATOR_MULTIPLY;
using model::FUNCTION_OPERATOR_DIVIDE;
using model::FUNCTION_OPERATOR_REMAINDER;
using model::FUNCTION_OPERATOR_BIT_OR;
using model::FUNCTION_OPERATOR_BIT_XOR;
using model::FUNCTION_OPERATOR_ASSIGN;
using model::FUNCTION_OPERATOR_PLUS_ASSIGN;
using model::FUNCTION_OPERATOR_MINUS_ASSIGN;
using model::FUNCTION_OPERATOR_MULTIPLY_ASSIGN;
using model::FUNCTION_OPERATOR_DIVIDE_ASSIGN;
using model::FUNCTION_OPERATOR_REMAINDER_ASSIGN;
using model::FUNCTION_OPERATOR_BIT_AND_ASSIGN;
using model::FUNCTION_OPERATOR_BIT_OR_ASSIGN;
using model::FUNCTION_OPERATOR_BIT_XOR_ASSIGN;
using model::FUNCTION_OPERATOR_SHIFT_LEFT;
using model::FUNCTION_OPERATOR_SHIFT_RIGHT;
using model::FUNCTION_OPERATOR_SHIFT_LEFT_ASSIGN;
using model::FUNCTION_OPERATOR_SHIFT_RIGHT_ASSIGN;
using model::FUNCTION_OPERATOR_EQUAL;
using model::FUNCTION_OPERATOR_NOT_EQUAL;
using model::FUNCTION_OPERATOR_LESS;
using model::FUNCTION_OPERATOR_GREATER;
using model::FUNCTION_OPERATOR_LESS_EQUAL;
using model::FUNCTION_OPERATOR_GREATER_EQUAL;
using model::FUNCTION_OPERATOR_LOGICAL_NOT;
using model::FUNCTION_OPERATOR_BIT_NOT;
using model::FUNCTION_OPERATOR_LOGICAL_AND;
using model::FUNCTION_OPERATOR_LOGICAL_OR;
using model::FUNCTION_OPERATOR_INCREMENT;
using model::FUNCTION_OPERATOR_DECREMENT;
using model::FUNCTION_OPERATOR_COMMA;
using model::FUNCTION_OPERATOR_MEMBER_POINTER;
using model::FUNCTION_OPERATOR_ARROW;
using model::FUNCTION_OPERATOR_CALL;
using model::FUNCTION_OPERATOR_INDEX;
using model::FUNCTION_OPERATOR_LITERAL;

std::vector<std::vector<std::string> > serialize_mangle_target_records(
    const AbiMangleTarget & target);

}  // namespace abi_mangle
