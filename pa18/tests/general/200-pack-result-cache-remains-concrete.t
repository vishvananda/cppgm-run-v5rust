template<class T> struct identity {
  typedef T type;
};

template<class...> struct list {
};

template<class L, class Prefix> struct drop_impl;

template<template<class...> class L, class... T,
         template<class...> class Prefix, class... U>
struct drop_impl<L<T...>, Prefix<U...> > {
  template<class... W>
  static identity<L<W...> > f(U*..., identity<W>*...);

  typedef decltype(f(static_cast<identity<T>*>(0)...)) result;
  typedef typename result::type type;
};

struct first;
struct second;

typedef drop_impl<list<first, second>, list<> >::type actual;
typedef list<first, second> expected;

template<class A, class B> struct same {
  static const bool value = false;
};

template<class A> struct same<A, A> {
  static const bool value = true;
};

static_assert(same<actual, expected>::value, "");

int main() {
  return 0;
}
