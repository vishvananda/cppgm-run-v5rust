// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec], 14.6.2.1 [temp.dep.type],
// 14.8 [temp.fct.spec]
// A substituted qualified template-id must retain its original qualified
// syntax when local member-scope lookup does not resolve it.

#include "../support.h"

struct context
{
  template<class T>
  class allocator;
};

template<class T>
class context::allocator
{
public:
  template<class U>
  struct rebind
  {
    typedef allocator<U> other;
  };
};

template<>
class context::allocator<void>
{
public:
  template<class U>
  struct rebind
  {
    typedef allocator<U> other;
  };
};

template<class T, class U, class = void>
struct has_rebind_other
{
  static const bool value = false;
};

template<class T, class U>
struct has_rebind_other<
    T, U, void_t<typename T::template rebind<U>::other> >
{
  static const bool value = true;
};

template<class Alloc, class U, bool = has_rebind_other<Alloc, U>::value>
struct allocator_traits_rebind;

template<template<class, class...> class Alloc,
         class T, class... Args, class U>
struct allocator_traits_rebind<Alloc<T, Args...>, U, true>
{
  using type = typename Alloc<T, Args...>::template rebind<U>::other;
};

template<template<class, class...> class Alloc,
         class T, class... Args, class U>
struct allocator_traits_rebind<Alloc<T, Args...>, U, false>
{
  using type = Alloc<U, Args...>;
};

template<class Alloc, class T>
using allocator_traits_rebind_t =
    typename allocator_traits_rebind<Alloc, T>::type;

int main()
{
  typedef allocator_traits_rebind_t<context::allocator<void>, char> rebound;
  rebound value;
  context::allocator<char> * expected = &value;
  return expected == &value ? 0 : 1;
}
