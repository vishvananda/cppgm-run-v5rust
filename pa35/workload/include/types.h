#pragma once

#include <cstddef>
#include <string>
#include <vector>

// See 3.9.1: Fundamental Types
enum EFundamentalType
{
  // 3.9.1.2
  FT_SIGNED_CHAR,
  FT_SHORT_INT,
  FT_INT,
  FT_LONG_INT,
  FT_LONG_LONG_INT,
  FT_INT128,

  // 3.9.1.3
  FT_UNSIGNED_CHAR,
  FT_UNSIGNED_SHORT_INT,
  FT_UNSIGNED_INT,
  FT_UNSIGNED_LONG_INT,
  FT_UNSIGNED_LONG_LONG_INT,
  FT_UINT128,

  // 3.9.1.1 / 3.9.1.5
  FT_WCHAR_T,
  FT_CHAR,
  FT_CHAR16_T,
  FT_CHAR32_T,

  // 3.9.1.6
  FT_BOOL,

  // 3.9.1.8
  FT_FLOAT,
  FT_DOUBLE,
  FT_LONG_DOUBLE,

  // 3.9.1.9
  FT_VOID,

  // 3.9.1.10
  FT_NULLPTR_T
};

std::string type_to_string(EFundamentalType type);

std::size_t type_to_size(EFundamentalType type);

bool type_is_signed(EFundamentalType type);

EFundamentalType classify_int(const std::string& data,
                              unsigned long long& result,
                              std::string& ud_suffix);

bool split_floating_literal(const std::string& text,
                            std::string& value,
                            EFundamentalType& type,
                            std::string& ud_suffix);

struct QuoteLiteralData {
  char quote;
  char enc;
  std::u32string contents;
  std::vector<unsigned long long> string_units;
  std::string ud_suffix;
  bool operator==(const QuoteLiteralData &other) const
  {
    return (quote == other.quote &&
            enc == other.enc &&
            contents == other.contents &&
            string_units == other.string_units &&
            ud_suffix == other.ud_suffix);
  }
};

void parse_quote_literal(const std::string& data, QuoteLiteralData& out);

inline QuoteLiteralData parse_quote_literal(const std::string& data)
{
  QuoteLiteralData out;
  parse_quote_literal(data, out);
  return out;
}

EFundamentalType string_literal_element_type(const QuoteLiteralData & literal);

EFundamentalType character_literal_type(const QuoteLiteralData & literal);

const std::vector<unsigned long long> &
quote_literal_string_units(const QuoteLiteralData & literal);

std::size_t quote_literal_string_unit_count(const QuoteLiteralData & literal);

// FundamentalTypeOf: convert fundamental type T to EFundamentalType
// for example: `FundamentalTypeOf<long int>()` will return `FT_LONG_INT`
template<typename T> constexpr EFundamentalType FundamentalTypeOf();
template<> constexpr EFundamentalType FundamentalTypeOf<signed char>() { return FT_SIGNED_CHAR; }
template<> constexpr EFundamentalType FundamentalTypeOf<short int>() { return FT_SHORT_INT; }
template<> constexpr EFundamentalType FundamentalTypeOf<int>() { return FT_INT; }
template<> constexpr EFundamentalType FundamentalTypeOf<long int>() { return FT_LONG_INT; }
template<> constexpr EFundamentalType FundamentalTypeOf<long long int>() { return FT_LONG_LONG_INT; }
template<> constexpr EFundamentalType FundamentalTypeOf<__int128_t>() { return FT_INT128; }
template<> constexpr EFundamentalType FundamentalTypeOf<unsigned char>() { return FT_UNSIGNED_CHAR; }
template<> constexpr EFundamentalType FundamentalTypeOf<unsigned short int>() { return FT_UNSIGNED_SHORT_INT; }
template<> constexpr EFundamentalType FundamentalTypeOf<unsigned int>() { return FT_UNSIGNED_INT; }
template<> constexpr EFundamentalType FundamentalTypeOf<unsigned long int>() { return FT_UNSIGNED_LONG_INT; }
template<> constexpr EFundamentalType FundamentalTypeOf<unsigned long long int>() { return FT_UNSIGNED_LONG_LONG_INT; }
template<> constexpr EFundamentalType FundamentalTypeOf<__uint128_t>() { return FT_UINT128; }
template<> constexpr EFundamentalType FundamentalTypeOf<wchar_t>() { return FT_WCHAR_T; }
template<> constexpr EFundamentalType FundamentalTypeOf<char>() { return FT_CHAR; }
template<> constexpr EFundamentalType FundamentalTypeOf<char16_t>() { return FT_CHAR16_T; }
template<> constexpr EFundamentalType FundamentalTypeOf<char32_t>() { return FT_CHAR32_T; }
template<> constexpr EFundamentalType FundamentalTypeOf<bool>() { return FT_BOOL; }
template<> constexpr EFundamentalType FundamentalTypeOf<float>() { return FT_FLOAT; }
template<> constexpr EFundamentalType FundamentalTypeOf<double>() { return FT_DOUBLE; }
template<> constexpr EFundamentalType FundamentalTypeOf<long double>() { return FT_LONG_DOUBLE; }
template<> constexpr EFundamentalType FundamentalTypeOf<void>() { return FT_VOID; }
template<> constexpr EFundamentalType FundamentalTypeOf<std::nullptr_t>() { return FT_NULLPTR_T; }
