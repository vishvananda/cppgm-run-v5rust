template<class...> struct L {};
template<class, class T> using I = T;
template<class...> using B = int;
template<class T, class> using A = typename T::type;
template<template<class...> class P> struct C { typedef I<P<int>, L<int> > type; };
template<class U> using Q = typename C<U::template fn>::type;
template<class... S> struct X { template<class> using fn = B<A<S, void>...>; };
template<class... S> using R = I<int, Q<X<S...> > >;
void f(L<int> *) {}
int main() { f((R<> *)0); }
