// VALIDATION: compile-pass
// The parameter pack in a template-template parameter belongs to the nested
// template declaration and does not leak into the enclosing template scope.

template<class... T>
struct list
{
};

template<template<class... Inner> class Container, class T>
struct apply
{
  Container<T> value;
};

int main()
{
  apply<list, int> value;
  (void)&value;
  return 0;
}
