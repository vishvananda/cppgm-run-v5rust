template<class> struct N {};
template<template<class...> class G> using A = G<char>;
template<template<class...> class F, class...> using B = A<F>;
template<class> struct O {
  template<class T> using M = N<T>;
  using R = B<M>;
};
using R = O<int>::R;
template<class, class> struct S;
template<class T> struct S<T, T> {};
struct Check : S<R, N<char>> {};
int main() {}
