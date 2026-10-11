// VALIDATION: compile-pass
// When relaxed template-template matching lets multiple partial
// specializations omit trailing default arguments, the pattern covering all
// canonical arguments is more specialized.

template<class T, class Allocator = void, class Options = void>
struct vector {};

template<class Container, class U>
struct container_rebind;

template<template<class, class, class...> class Container,
         class V, class A, class... Rest, class U>
struct container_rebind<Container<V, A, Rest...>, U>
{
  static const int value = 1;
};

template<template<class, class> class Container,
         class V, class A, class U>
struct container_rebind<Container<V, A>, U>
{
  static const int value = 2;
};

template<template<class> class Container,
         class V, class U>
struct container_rebind<Container<V>, U>
{
  static const int value = 3;
};

typedef container_rebind<vector<int, void, void>, long> selected;
static_assert(selected::value == 1, "full canonical argument coverage");

int main()
{
  return selected::value == 1 ? 0 : 1;
}
