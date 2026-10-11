// Template-template parameter matching compares parameter kinds and, for a
// non-type parameter, its declared type. Equal arity alone is not sufficient.

template<class T, T Value>
struct dependent_value {};

template<class T, class U>
struct type_pair {};

template<class T>
struct classification {
  static const int value = 0;
};

template<template<class, unsigned long> class L,
         class T,
         unsigned long N>
struct classification<L<T, N> > {
  static const int value = 1;
};

static_assert(classification<dependent_value<int, 0> >::value == 0,
              "a dependent int non-type parameter does not match unsigned long");
static_assert(classification<type_pair<int, int> >::value == 0,
              "a type parameter does not match a non-type parameter");

int main() { return 0; }
