// VALIDATION: compile-pass
// N3485 focus: 14.3.3 [temp.arg.template], 14.5.3 [temp.variadic]
// Applying a bound template-template parameter head must not be routed through
// ordinary template-id lookup.

#include "../support.h"

template<class... T>
struct list
{};

template<template<class...> class L>
struct make_empty
{
  typedef L<> type;
};

template<class L, class... U>
struct rebind;

template<template<class...> class L, class... T, class... U>
struct rebind<L<T...>, U...>
{
  typedef L<U...> type;
};

template<template<class...> class L>
struct delayed_apply
{
  template<class... T>
  struct apply
  {
    typedef L<T...> type;
  };
};

int main()
{
  typedef make_empty<list>::type empty;
  typedef rebind<list<int>, char, long>::type rebound;
  typedef delayed_apply<list>::apply<int, char>::type delayed;
  return is_same<empty, list<> >::value &&
         is_same<rebound, list<char, long> >::value &&
         is_same<delayed, list<int, char> >::value ? 0 : 1;
}
