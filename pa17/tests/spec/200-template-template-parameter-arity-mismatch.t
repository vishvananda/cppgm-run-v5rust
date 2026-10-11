// VALIDATION: compile-fail
// N3485 focus: 14.3.3 [temp.arg.template]

template<typename T, typename U>
struct pair_box
{
};

template<template<typename> class TT>
struct use
{
};

use<pair_box> bad_use;

int main()
{
  return 0;
}
