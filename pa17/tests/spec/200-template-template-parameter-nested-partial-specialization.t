// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec], 14.8.2.5 [temp.deduct.type]

template<class T>
struct static_cast_action {};

template<class T>
struct cast_action {};

template<class Act, class Args>
struct return_type_N;

template<template<class> class cast_type, class T, class A>
struct return_type_N<cast_action<cast_type<T> >, A> {
  typedef T type;
};

struct marker {};

typedef return_type_N<cast_action<static_cast_action<int> >, marker>::type result;

int main()
{
  result value = 0;
  return value;
}
