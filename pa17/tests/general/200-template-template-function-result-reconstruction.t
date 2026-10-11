// VALIDATION: compile-pass
namespace n {
template<class T>
struct wrapper {};
}

template<template<class> class F>
struct generator {};

template<class>
struct result_of;

template<template<class> class F, class T>
struct result_of<generator<F>(T)> {
  typedef F<T> type;
};

template<class Generator, class A>
typename result_of<Generator(A)>::type call(A);

template<class T>
using id_t = T;

template<class>
struct owner {
  typedef id_t<decltype(call<generator<n::wrapper> >(0))> result;
};

owner<int> value;

int main()
{
  return 0;
}
