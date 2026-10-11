#pragma once

#include <string>
#include <utility>
#include <vector>

#include "posttokenizer.h"
#include "text_intern.h"

bool PA6_IsClassName(const std::string& identifier);
bool PA6_IsTemplateName(const std::string& identifier);
bool PA6_IsTypedefName(const std::string& identifier);
bool PA6_IsEnumName(const std::string& identifier);
bool PA6_IsNamespaceName(const std::string& identifier);

enum ERecogTokenKind
{
  RT_INVALID,
  RT_SIMPLE,
  RT_IDENTIFIER,
  RT_LITERAL,
  RT_EOF,
  RT_RSHIFT_1,
  RT_RSHIFT_2
};

struct RecogToken
{
  RecogToken()
    : kind(RT_INVALID),
      source_atom(nullptr),
      simple_type(static_cast<ETokenType>(0)),
      location_id(0)
  {}

  RecogToken(ERecogTokenKind kind,
             std::string source_text,
             ETokenType simple_type,
             uint32_t location_id = 0)
    : kind(kind),
      source(std::move(source_text)),
      source_atom(nullptr),
      simple_type(simple_type),
      location_id(location_id)
  {}

  ERecogTokenKind kind;
  std::string source;
  mutable text_intern::Atom source_atom;
  ETokenType simple_type;
  uint32_t location_id = 0;

  text_intern::Atom cached_identifier_atom() const
  {
    if(kind != RT_IDENTIFIER) {
      return nullptr;
    }
    if(!source_atom) {
      source_atom = text_intern::find(source);
    }
    return source_atom;
  }

  bool is_invalid() const;
  bool is_simple(ETokenType type) const;
  bool is_identifier() const;
  bool is_literal() const;
  bool is_eof() const;
  bool is_rshift_piece() const;
  bool is_close_angle_bracket() const;
  bool is_nonparen() const;
  bool is_empty_string() const;
  bool is_zero() const;
  bool is_final() const;
  bool is_override() const;
  bool is_class_name() const;
  bool is_template_name() const;
  bool is_typedef_name() const;
  bool is_enum_name() const;
  bool is_namespace_name() const;
};

struct IRecogTokenSource
{
  virtual ~IRecogTokenSource() {}
  virtual RecogToken get() = 0;
  virtual void get_many(std::vector<RecogToken> & out, std::size_t max_tokens)
  {
    for(std::size_t i = 0; i < max_tokens; ++i) {
      RecogToken token = get();
      const bool done = token.is_eof() || token.is_invalid();
      out.push_back(std::move(token));
      if(done) {
        return;
      }
    }
  }
};

struct RecogTokenizer : IRecogTokenSource
{
  explicit RecogTokenizer(IPostTokenSource & input);
  RecogToken get() override;
  void get_many(std::vector<RecogToken> & out, std::size_t max_tokens) override;
  const std::string & error() const { return error_msg; }

protected:
  void append(PostToken & token, std::vector<RecogToken> & output);

  IPostTokenSource & input;
  std::string error_msg;
  std::vector<RecogToken> pending;
};
