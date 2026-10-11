// VALIDATION: compile-pass
// N3485 focus: 14.3.2 [temp.arg.nontype], 14.5.7 [temp.alias]
// A class specialization substituted from a non-type template parameter must
// carry the substituted value in its specialization metadata so later qualified
// lookup on that type can resolve members such as I::value.

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

template<unsigned long I>
using size_constant = integral_constant<unsigned long, (unsigned long)I>;

template<unsigned long I>
struct iota_one
{
  typedef list<size_constant<I> > type;
};

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
using value_pair = list<N, integral_constant<unsigned long, I::value> >;

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

struct local_use
{
  void run()
  {
    typedef typename iota_one<0>::type inputs;
    typedef transform_q<bind_front<value_pair, mp_int<4> >, inputs> result;
    typedef list<list<mp_int<4>, size_constant<0> > > expected;
    typedef char check[is_same<result, expected>::value ? 1 : -1];
    (void)sizeof(check);
  }
};

int main()
{
  local_use value;
  value.run();
  return 0;
}
