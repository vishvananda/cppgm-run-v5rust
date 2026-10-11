// VALIDATION: compile-fail
// N3485 focus: 14.1 [temp.param], 14.3.3 [temp.arg.template]

template<template<class> class F>
struct holder;

template<template<int> class F>
struct holder
{
};

int main()
{
  return 0;
}
