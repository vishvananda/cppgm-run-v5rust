// VALIDATION: compile-pass
template<class A, class B> struct F { using type = A; };
template<template<class...> class F> struct step {
  template<class V, class T, class N = F<typename V::type, T>> using fn = N;
};
template<class V, template<class...> class F> struct twice {
  template<class A, class B> using fn = F<F<V, A>, B>;
};
using R = twice<F<void, void>, step<F>::template fn>::template fn<int, char>;
int main() {}
