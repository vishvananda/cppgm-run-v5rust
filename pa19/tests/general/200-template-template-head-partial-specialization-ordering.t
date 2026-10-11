// Reduced from Boost.MPL lambda. A partial specialization with a concrete
// template-id head is more specialized than a generic template-template
// parameter pattern with the same arity.

template<int N>
struct int_ {};

template<class F, class T1>
struct bind1 {};

template<class T, class Tag, class Arity>
struct lambda {
  static const int value = 0;
};

template<template<class, class> class F, class T1, class T2, class Tag>
struct lambda<F<T1, T2>, Tag, int_<2> > {
  static const int value = 1;
};

template<class F, class T1, class Tag>
struct lambda<bind1<F, T1>, Tag, int_<2> > {
  static const int value = 2;
};

struct tag {};
struct item {};

template<class T, class U>
struct predicate {};

static_assert(lambda<bind1<predicate<item, item>, item>, tag, int_<2> >::value == 2,
              "concrete template head is more specialized");

int main()
{
  return lambda<bind1<predicate<item, item>, item>, tag, int_<2> >::value == 2 ? 0 : 1;
}
