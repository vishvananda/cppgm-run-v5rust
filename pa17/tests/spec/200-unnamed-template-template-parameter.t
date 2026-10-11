// VALIDATION: compile-pass
// N3485 focus: 14.1 [temp.param], 14.3.3 [temp.arg.template]

template<class T>
struct box
{
};

template<template<class> class = box>
struct holder
{
};

int main()
{
  return 0;
}
