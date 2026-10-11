typedef unsigned long size_t;

template<class T, T V>
struct integral_constant {
  static const T value = V;
};

template<int I>
using int_constant = integral_constant<int, I>;

template<bool B>
using bool_constant = integral_constant<bool, B>;

template<size_t I>
using size_constant = integral_constant<size_t, I>;

typedef bool_constant<true> true_type;
typedef bool_constant<false> false_type;

template<class... T>
struct type_list {};

template<class L>
struct size_of;

template<template<class...> class L, class... T>
struct size_of<L<T...> > {
  typedef size_constant<sizeof...(T)> type;
};

template<class L>
using size = typename size_of<L>::type;

template<bool C, class T, class E>
struct if_c_impl {
  typedef T type;
};

template<class T, class E>
struct if_c_impl<false, T, E> {
  typedef E type;
};

template<bool C, class T, class E>
using if_c = typename if_c_impl<C, T, E>::type;

template<class L, size_t I>
struct at_impl;

template<class A0, class... Rest>
struct at_impl<type_list<A0, Rest...>, 0> {
  typedef A0 type;
};

template<class L, size_t I>
using at_c = typename if_c<(I < size<L>::value), at_impl<L, I>, void>::type;

template<class L, class I>
using at = at_c<L, size_t{I::value}>;

struct base {};
struct super {};

struct spec {
  template<class Super>
  struct index_class {
    typedef Super type;
  };
};

template<class I, class L, class Super>
using layer_index = typename at<L, I>::template index_class<Super>::type;

template<template<class...> class F, class... T>
struct valid_impl {
  template<template<class...> class G, class = G<T...> >
  static true_type check(int);

  template<template<class...> class>
  static false_type check(...);

  typedef decltype(check<F>(0)) type;
};

template<template<class...> class F, class... T>
using valid = typename valid_impl<F, T...>::type;

template<bool B, class T, class E>
struct if_type_impl {
  typedef T type;
};

template<class T, class E>
struct if_type_impl<false, T, E> {
  typedef E type;
};

template<class C, class T, class E>
using if_type = typename if_type_impl<static_cast<bool>(C::value), T, E>::type;

template<template<class...> class F, class... T>
struct defer_impl {
  typedef F<T...> type;
};

struct no_type {};

template<template<class...> class F, class... T>
using defer = if_type<valid<F, T...>, defer_impl<F, T...>, no_type>;

template<bool C, class T, template<class...> class F, class... U>
struct eval_if_c_impl;

template<class T, template<class...> class F, class... U>
struct eval_if_c_impl<true, T, F, U...> {
  typedef T type;
};

template<class T, template<class...> class F, class... U>
struct eval_if_c_impl<false, T, F, U...>: defer<F, U...> {};

template<bool C, class T, template<class...> class F, class... U>
using eval_if_c = typename eval_if_c_impl<C, T, F, U...>::type;

template<class A, class B>
struct is_same {
  static const bool value = false;
};

template<class A>
struct is_same<A, A> {
  static const bool value = true;
};

typedef eval_if_c<false, base, layer_index, int_constant<0>, type_list<spec>, super> result;

static_assert(is_same<result, super>::value, "");

int main() {
  return 0;
}
