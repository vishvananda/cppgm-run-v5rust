#pragma once

template<typename A, typename B>
struct is_same
{
  static const bool value = false;
};

template<typename A>
struct is_same<A, A>
{
  static const bool value = true;
};

template<bool B, typename T = void>
struct enable_if
{
};

template<typename T>
struct enable_if<true, T>
{
  typedef T type;
};

template<typename... Ts>
struct make_void
{
  typedef void type;
};

template<typename... Ts>
using void_t = typename make_void<Ts...>::type;

template<typename T>
T && declval();

struct true_type
{
  static const bool value = true;
};

struct false_type
{
  static const bool value = false;
};
