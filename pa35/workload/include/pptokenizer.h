#pragma once
#include <array>
#include <cstddef>
#include <iostream>
#include <iterator>
#include <memory>
#include <queue>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

enum EPPTokenType {
  PP_WHITESPACE,
  PP_NEW_LINE,
  PP_HEADER_NAME,
  PP_IDENTIFIER,
  PP_INT_LITERAL, // ppnumber
  PP_FLOAT_LITERAL, // ppnumber
  PP_QUOTE_LITERAL, // (user defined) character/string literal
  PP_PREPROCESSING_OP,
  PP_NON_WHITESPACE,
  PP_EOF
};

struct EPPToken {
  EPPTokenType type;
  std::string data;
};

struct IPPTokenStream
{
  virtual void emit(const EPPTokenType type,
                    const std::string & data = std::string()) = 0;
  virtual ~IPPTokenStream() {}
};

struct IPPTokenSource
{
  virtual EPPToken get() = 0;
  virtual ~IPPTokenSource() {}
};

inline void stream_pp_tokens(IPPTokenSource & source, IPPTokenStream & output)
{
  EPPToken next;
  do {
    next = source.get();
    output.emit(next.type, next.data);
  } while (next.type != PP_EOF);
}

struct CodePointIterator
{
  CodePointIterator(const unsigned long long ln,
                    const unsigned long long ch) :
      ln(ln), ch(ch) {}
  virtual int operator*() = 0;
  virtual CodePointIterator& operator++() = 0;

  unsigned long long ln;
  unsigned long long ch;
};

struct Normalizer : CodePointIterator
{
  Normalizer(std::streambuf * buf);
  int operator*() override;
  CodePointIterator& operator++() override;

protected:
  std::streambuf * buf;
  int value;
};

struct BufferedIterator : CodePointIterator
{
  struct Buffer {
    Buffer();
    bool empty() const;
    int front() const;
    std::size_t size() const;
    int operator[](std::size_t n) const;
    void push_back(int value);
    void push_front(int value);
    void set_front(int value);
    void pop_front();

  private:
    static constexpr std::size_t InlineCapacity = 32;

    std::size_t index(std::size_t n) const;
    std::size_t storage_capacity() const;
    void ensure_capacity(std::size_t required);

    std::array<int, InlineCapacity> inline_data;
    std::vector<int> overflow_data;
    int * data;
    std::size_t capacity;
    std::size_t mask;
    std::size_t start;
    std::size_t count;
  };

  BufferedIterator(CodePointIterator & source);
  int operator*() override;
  CodePointIterator& operator++() override;
  int peek(int n = 0);
  int next();
  int pop();
  std::u32string extract32(int n);
  std::string extract(int n);

  CodePointIterator & source;
  Buffer buffer;
};

struct UTF8Translator : BufferedIterator {
  UTF8Translator(CodePointIterator & source);
  int operator*() override;
protected:
  inline void translate_utf8();
  bool allow_initial_bom;
};

struct FullTranslator : UTF8Translator {
  FullTranslator(CodePointIterator & source);
  int operator*() override;
protected:
  inline void translate_trigraph_ucn_splice();

};

struct PPTokenizer : IPPTokenSource
{
  PPTokenizer(std::streambuf * buf);
  EPPToken get() override;
  void stream(IPPTokenStream & output);
  inline unsigned long long get_ch() {return buffer.ch;};
  inline unsigned long long get_ln() {return buffer.ln;};
  inline unsigned long long get_token_ch() const {return token_ch;};
  inline unsigned long long get_token_ln() const {return token_ln;};
  inline void set_ln(unsigned long long ln) {norm.ln = ln;};

protected:
  inline std::string get_raw_string(unsigned int i);
  inline std::string get_regular_string(unsigned int i, const char quote);
  inline bool get_stringlike(std::string & out);
  enum struct HeaderState {Start, Hash, Include, None};
  Normalizer norm;
  UTF8Translator raw;
  FullTranslator translator;
  BufferedIterator buffer;
  HeaderState header_state;
  unsigned long long token_ln = 0;
  unsigned long long token_ch = 0;
};

inline EPPToken token(const EPPTokenType & type,
                      std::string data = std::string()) {
  return {type, std::move(data)};
}

inline std::vector<EPPToken> tokenize(const std::string & data)
{
  std::istringstream is(data);
  PPTokenizer tokenizer(is.rdbuf());
  std::vector<EPPToken> tokens;
  try {
    EPPToken next;
    do {
      next = tokenizer.get();
      tokens.push_back(std::move(next));
    } while (next.type != PP_EOF);
  } catch (std::logic_error& e) {
    throw std::logic_error(std::string("Error tokenizing string: ") +
                           e.what());
  }
  return tokens;
}
