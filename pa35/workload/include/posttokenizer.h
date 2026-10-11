#pragma once
#include <cstddef>
#include <iostream>
#include <deque>
#include <string>
#include <utility>
#include <vector>

#include "encoding.h"
#include "pptokenizer.h"
#include "source_location.h"
#include "types.h"

// token type enum for `simples`
enum ETokenType
{
#define CPPGM_SIMPLE_KEYWORD_TOKEN(token, spelling) token,
#define CPPGM_SIMPLE_OPERATOR_TOKEN(token, spelling) token,
#define CPPGM_SIMPLE_TOKEN_ALIAS(token, spelling)
#include "simple_tokens.inc"
#undef CPPGM_SIMPLE_TOKEN_ALIAS
#undef CPPGM_SIMPLE_OPERATOR_TOKEN
#undef CPPGM_SIMPLE_KEYWORD_TOKEN
};

inline const char * token_type_to_string(ETokenType token_type)
{
  static const char * const names[] =
  {
#define CPPGM_SIMPLE_KEYWORD_TOKEN(token, spelling) #token,
#define CPPGM_SIMPLE_OPERATOR_TOKEN(token, spelling) #token,
#define CPPGM_SIMPLE_TOKEN_ALIAS(token, spelling)
#include "simple_tokens.inc"
#undef CPPGM_SIMPLE_TOKEN_ALIAS
#undef CPPGM_SIMPLE_OPERATOR_TOKEN
#undef CPPGM_SIMPLE_KEYWORD_TOKEN
  };
  return names[static_cast<std::size_t>(token_type)];
}


struct IPostTokenOutputStream
{
  virtual void emit_invalid(const std::string& source) = 0;
  virtual void emit_simple(const std::string& source,
                           ETokenType token_type) = 0;
  virtual void emit_identifier(const std::string& source) = 0;
  virtual void emit_literal(const std::string& source, EFundamentalType type,
      const void* data, std::size_t nbytes) = 0;
  virtual void emit_literal_array(const std::string& source,
      std::size_t num_elements, EFundamentalType type, const void* data,
      std::size_t nbytes) = 0;
  virtual void emit_user_defined_literal_character(const std::string& source,
      const std::string& ud_suffix, EFundamentalType type, const void* data,
      std::size_t nbytes) = 0;
  virtual void emit_user_defined_literal_string_array(
      const std::string& source, const std::string& ud_suffix,
      std::size_t num_elements, EFundamentalType type, const void* data,
      std::size_t nbytes) = 0;
  virtual void emit_user_defined_literal_integer(const std::string& source,
      const std::string& ud_suffix, const std::string& prefix) = 0;
  virtual void emit_user_defined_literal_floating(const std::string& source,
      const std::string& ud_suffix, const std::string& prefix) = 0;
  virtual void emit_eof() = 0;
};

enum EPostTokenKind
{
  PT_INVALID,
  PT_SIMPLE,
  PT_IDENTIFIER,
  PT_LITERAL,
  PT_LITERAL_ARRAY,
  PT_USER_DEFINED_LITERAL_CHARACTER,
  PT_USER_DEFINED_LITERAL_STRING_ARRAY,
  PT_USER_DEFINED_LITERAL_INTEGER,
  PT_USER_DEFINED_LITERAL_FLOATING,
  PT_EOF
};

struct PostToken
{
  PostToken()
    : kind(PT_INVALID),
      token_type(static_cast<ETokenType>(0)),
      type(static_cast<EFundamentalType>(0)),
      num_elements(0),
      location_id(0)
  {}

  PostToken(EPostTokenKind kind, const std::string & source)
    : kind(kind),
      source(source),
      token_type(static_cast<ETokenType>(0)),
      type(static_cast<EFundamentalType>(0)),
      num_elements(0),
      location_id(0)
  {}

  PostToken(EPostTokenKind kind,
            const std::string & source,
            ETokenType token_type,
            EFundamentalType type,
            const std::vector<unsigned char> & data,
            std::size_t num_elements,
            const std::string & ud_suffix,
            const std::string & prefix,
            uint32_t location_id)
    : kind(kind),
      source(source),
      token_type(token_type),
      type(type),
      data(data),
      num_elements(num_elements),
      ud_suffix(ud_suffix),
      prefix(prefix),
      location_id(location_id)
  {}

  EPostTokenKind kind;
  std::string source;
  ETokenType token_type;
  EFundamentalType type;
  std::vector<unsigned char> data;
  std::size_t num_elements;
  std::string ud_suffix;
  std::string prefix;
  uint32_t location_id = 0;
};

struct IPostTokenSource
{
  virtual PostToken get() = 0;
  virtual void get_many(std::vector<PostToken> & out, std::size_t max_tokens)
  {
    for(std::size_t i = out.size(); i < max_tokens; ++i) {
      PostToken token = get();
      const EPostTokenKind kind = token.kind;
      out.push_back(std::move(token));
      if(kind == PT_EOF || kind == PT_INVALID) {
        return;
      }
    }
  }
  virtual ~IPostTokenSource() {}
};

inline void stream_post_tokens(IPostTokenSource & source,
                               IPostTokenOutputStream & output)
{
  for(;;) {
    auto token = source.get();
    switch(token.kind) {
    case PT_INVALID:
      output.emit_invalid(token.source);
      break;
    case PT_SIMPLE:
      output.emit_simple(token.source, token.token_type);
      break;
    case PT_IDENTIFIER:
      output.emit_identifier(token.source);
      break;
    case PT_LITERAL:
      output.emit_literal(token.source, token.type, token.data.data(),
                          token.data.size());
      break;
    case PT_LITERAL_ARRAY:
      output.emit_literal_array(token.source, token.num_elements, token.type,
                                token.data.data(), token.data.size());
      break;
    case PT_USER_DEFINED_LITERAL_CHARACTER:
      output.emit_user_defined_literal_character(token.source, token.ud_suffix,
                                                 token.type, token.data.data(),
                                                 token.data.size());
      break;
    case PT_USER_DEFINED_LITERAL_STRING_ARRAY:
      output.emit_user_defined_literal_string_array(token.source,
                                                    token.ud_suffix,
                                                    token.num_elements,
                                                    token.type,
                                                    token.data.data(),
                                                    token.data.size());
      break;
    case PT_USER_DEFINED_LITERAL_INTEGER:
      output.emit_user_defined_literal_integer(token.source, token.ud_suffix,
                                               token.prefix);
      break;
    case PT_USER_DEFINED_LITERAL_FLOATING:
      output.emit_user_defined_literal_floating(token.source, token.ud_suffix,
                                                token.prefix);
      break;
    case PT_EOF:
      output.emit_eof();
      return;
    }
  }
}

// DebugPostTokenOutputStream: helper class to produce PA2 output format
struct DebugPostTokenOutputStream : IPostTokenOutputStream
{
  // output: invalid <source>
  void emit_invalid(const std::string& source) override
  {
    std::cout << "invalid " << source << '\n';
  }

  // output: simple <source> <token_type>
  void emit_simple(const std::string& source, ETokenType token_type) override
  {
    std::cout << "simple " << source << " " <<
         token_type_to_string(token_type) << '\n';
  }

  // output: identifier <source>
  void emit_identifier(const std::string& source) override
  {
    std::cout << "identifier " << source << '\n';
  }

  // output: literal <source> <type> <hexdump(data,nbytes)>
  void emit_literal(const std::string& source, EFundamentalType type,
      const void* data, std::size_t nbytes) override
  {
    std::cout << "literal " << source << " " << type_to_string(type) <<
         " " << hex_dump(data, nbytes) << '\n';
  }

  // output: literal <source> array of <num_elements> <type>
  //         <hexdump(data,nbytes)>
  void emit_literal_array(const std::string& source,
      std::size_t num_elements, EFundamentalType type, const void* data,
      std::size_t nbytes) override
  {
    std::cout << "literal " << source << " array of " << num_elements <<
         " " << type_to_string(type) << " " << hex_dump(data, nbytes) <<
         '\n';
  }

  // output: user-defined-literal <source> <ud_suffix> character <type>
  //         <hexdump(data,nbytes)>
  void emit_user_defined_literal_character(const std::string& source,
      const std::string& ud_suffix, EFundamentalType type, const void* data,
      std::size_t nbytes) override
  {
    std::cout << "user-defined-literal " << source << " " << ud_suffix <<
         " character " << type_to_string(type) << " " <<
         hex_dump(data, nbytes) << '\n';
  }

  // output: user-defined-literal <source> <ud_suffix> std::string array of
  //         <num_elements> <type> <hexdump(data, nbytes)>
  void emit_user_defined_literal_string_array(const std::string& source,
      const std::string& ud_suffix, std::size_t num_elements,
      EFundamentalType type, const void* data, std::size_t nbytes) override
  {
    std::cout << "user-defined-literal " << source << " " << ud_suffix <<
         " string array of " << num_elements << " " <<
         type_to_string(type) << " " << hex_dump(data, nbytes) <<
         '\n';
  }

  // output: user-defined-literal <source> <ud_suffix> <prefix>
  void emit_user_defined_literal_integer(const std::string& source,
      const std::string& ud_suffix, const std::string& prefix) override
  {
    std::cout << "user-defined-literal " << source << " " << ud_suffix <<
         " integer " << prefix << '\n';
  }

  // output: user-defined-literal <source> <ud_suffix> <prefix>
  void emit_user_defined_literal_floating(const std::string& source,
      const std::string& ud_suffix, const std::string& prefix) override
  {
    std::cout << "user-defined-literal " << source << " " << ud_suffix <<
         " floating " << prefix << '\n';
  }

  // output : eof
  void emit_eof() override
  {
    std::cout << "eof" << '\n';
  }
};

struct PostTokenizer : IPostTokenSource
{
  IPPTokenSource & input;
  std::deque<PostToken> ready;
  std::string n_data;
  std::string n_suffix;
  std::u32string n_contents;
  char n_encoding;
  bool n_valid;
  SourceLocationTable * location_table;
  const ISourceLocationProvider * location_provider;
  uint32_t current_location_id;
  std::string cached_location_file;
  uint16_t cached_location_file_index;
  bool cached_location_file_valid;
  bool file_only_source_locations;

  PostTokenizer(IPPTokenSource & input,
                SourceLocationTable * location_table = nullptr,
                const ISourceLocationProvider * location_provider = nullptr,
                bool file_only_source_locations = false);
  PostToken get() override;
  void get_many(std::vector<PostToken> & out, std::size_t max_tokens) override;
protected:
  inline void process(const EPPTokenType type, const std::string& data);
  inline void capture_current_location();
  inline void push_invalid(const std::string& data);
  inline void push_simple(const std::string& data, ETokenType token_type);
  inline void push_identifier(const std::string& data);
  inline void push_literal(const std::string& data, EFundamentalType type,
                           const void * result, std::size_t size);
  inline void push_literal_array(const std::string& data,
                                 std::size_t num_elements,
                                 EFundamentalType type, const void * result,
                                 std::size_t size);
  inline void push_ud_character(const std::string& data,
                                const std::string& ud_suffix,
                                EFundamentalType type, const void * result,
                                std::size_t size);
  inline void push_ud_string_array(const std::string& data,
                                   const std::string& ud_suffix,
                                   std::size_t num_elements,
                                   EFundamentalType type,
                                   const void * result, std::size_t size);
  inline void push_ud_integer(const std::string& data,
                              const std::string& ud_suffix,
                              const std::string& prefix);
  inline void push_ud_floating(const std::string& data,
                               const std::string& ud_suffix,
                               const std::string& prefix);
  inline void emit_header_name(const std::string& data);
  inline void emit_identifier(const std::string& data);
  inline void emit_float_literal(const std::string& data);
  inline void emit_int_literal(const std::string& data);
  inline void emit_quote_literal(const std::string& data);
  inline void emit_preprocessing_op_or_punc(const std::string& data);
  inline void emit_non_whitespace_char(const std::string& data);
  inline void emit_eof();
  inline void emit_next_string();
  inline void emit_stringlike(char quote, const std::string& data,
      const std::string& ud_suffix, EFundamentalType type,
      const void * result, std::size_t size, std::size_t length);
  inline void emit_encoded(const std::string& data, char quote, char enc,
      const std::u32string& result, const std::string& ud_suffix);
};
