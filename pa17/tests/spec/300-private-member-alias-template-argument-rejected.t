// VALIDATION: compile-fail
// N3485 focus: 11 [class.access], 14.3.3 [temp.arg.template]

template<template<class> class F>
struct use
{
};

class owner
{
  template<class T>
  using hidden = T;
};

use<owner::hidden> rejected;

int main()
{
  return 0;
}
