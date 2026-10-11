template<template<class...> class F> struct quote {
  template<class... T> using fn = F<T...>;
};

template<template<class...> class F> using defer = quote<F>;

template<class Q> using apply = typename Q::template fn<int, int>;

template<class T> void check(T*) {}

template<class, class> struct F {};

int main() {
  using Q = quote<F>;
  check((apply<Q>*)0);
}
