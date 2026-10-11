// VALIDATION: compile-fail
// N3485 focus: 11.2 [class.access.base], 14.3.3 [temp.arg.template]

template<template<class> class F>
struct use
{
};

struct base
{
  template<class T>
  struct member
  {
  };
};

struct derived : private base
{
};

use<derived::member> rejected;

int main()
{
  return 0;
}
