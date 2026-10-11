// VALIDATION: compile-pass
template<class...> struct V {};
namespace n { template<class...> struct B {}; struct P {}; }
template<template<template<class...> class, class = void> class F>
using A = V<F<n::B>, F<n::B, n::P> >;
template<class> struct first;
template<class T, class U> struct first<V<T, U> > { typedef T type; };
template<class> struct second;
template<class T, class U> struct second<V<T, U> > { typedef U type; };
namespace {
template<template<class...> class, class = void>
struct X {
  struct N {
    typedef int table;
  };
};
typedef second<A<X> >::type Y;
typedef first<A<X> >::type D;
}
typedef Y::N::table R;
template<class> struct I {};
template<class... T> struct Run {
  static void go() { int a[] = { (I<T>(), 0)... }; (void)a; }
};
template<class> struct Apply;
template<class... T> struct Apply<V<T...> > : Run<T...> {};
int main() { Apply<A<X> >::go(); return sizeof(R) != 1; }
