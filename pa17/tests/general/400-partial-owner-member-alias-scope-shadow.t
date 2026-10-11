template<class> struct N {};
template<template<class> class, bool> struct O;
template<template<class> class F> struct O<F, false> {
  template<class T> using A = N<F<T>>;
};
template<template<class> class F> using X = F<char>;
using R = X<O<N, false>::template A>;
template<class, class> struct S;
template<class T> struct S<T, T> {};
struct Check : S<R, N<N<char>>> {};
int main() {}
