template<class...>
struct list {};

template<template<class...> class F>
struct quote {
  template<class... T>
  using fn = F<T...>;
};

template<template<class...> class F>
struct apply0 {
  using type = F<>;
};

using R = typename apply0<quote<list>::fn>::type;

int main() {
  return sizeof(R) == sizeof(list<>) ? 0 : 1;
}
