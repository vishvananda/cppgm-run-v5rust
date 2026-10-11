// VALIDATION: compile-pass
// N3485 focus: 14.3.3 [temp.arg.template], 14.5.7 [temp.alias]
// Identically named member alias templates from different concrete owners are
// distinct template arguments and must not share an instantiation.

template<class...> struct one {};
template<class...> struct two {};

template<template<class...> class F>
struct quote
{
  template<class... T>
  using fn = F<T...>;
};

template<template<class...> class F>
struct apply
{
  using type = F<>;
};

template<class, class> struct is_same { static const bool value = false; };
template<class T> struct is_same<T, T> { static const bool value = true; };

using R1 = typename apply<quote<one>::fn>::type;
using R2 = typename apply<quote<two>::fn>::type;

static_assert(!is_same<R1, R2>::value,
              "member alias template owner identity must be preserved");

int main() { return 0; }
