// VALIDATION: compile-pass
// A non-type argument retained by a member of a recursive partial
// specialization resolves template-template bindings in its instance scope.
template<bool B> struct result { static const bool value = B; };
template<class> struct trait : result<true> {};
template<template<class> class...> struct constraints {
  template<class> struct apply : result<true> {};
};
template<template<class> class Trait, template<class> class... Traits>
struct constraints<Trait, Traits...> {
  template<class T> struct apply : result<
      Trait<T>::value && constraints<Traits...>::template apply<T>::value> {};
};
int main() { return constraints<trait>::apply<int>::value; }
