#pragma once

// A deduced return type without a function body is not C++11. PA29 accepts
// this C++14 form because newer hosted STL headers declare deleted overloads
// with placeholder return types.
inline constexpr auto blocked(long double) = delete;

inline constexpr int blocked(int value)
{
  return value;
}
