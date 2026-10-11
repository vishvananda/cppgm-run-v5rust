// VALIDATION: compile-pass
// N3485 focus: 14.3.3 [temp.arg.template], 14.5.7 [temp.alias]
// A member alias template applied through a bound template-template parameter
// must resolve the member alias owner's parameters before any outer parameter
// with the same name.

template<class... T>
struct list
{};

template<class T, T V>
struct integral_constant
{
  static const T value = V;
};

template<int I>
using mp_int = integral_constant<int, I>;

template<template<class...> class F, class... T>
struct defer_impl
{
  typedef F<T...> type;
};

template<template<class...> class F, class... T>
using defer = defer_impl<F, T...>;

template<template<class...> class F, class... T>
struct bind_front
{
  template<class... U>
  using fn = typename defer<F, T..., U...>::type;
};

template<class N, class I>
using to_bits_t = list<N, I>;

template<template<class...> class F, class L>
struct transform_impl;

template<template<class...> class F, template<class...> class L, class T>
struct transform_impl<F, L<T> >
{
  typedef L<F<T> > type;
};

template<template<class...> class F, class L>
using transform = typename transform_impl<F, L>::type;

template<class Q, class L>
using transform_q = transform<Q::template fn, L>;

template<class A, class B>
struct is_same
{
  static const bool value = false;
};

template<class A>
struct is_same<A, A>
{
  static const bool value = true;
};

typedef list<integral_constant<unsigned long, 0> > inputs;
typedef transform_q<bind_front<to_bits_t, mp_int<4> >, inputs> result;
typedef list<list<mp_int<4>, integral_constant<unsigned long, 0> > > expected;

int main()
{
  return is_same<result, expected>::value ? 0 : 1;
}
