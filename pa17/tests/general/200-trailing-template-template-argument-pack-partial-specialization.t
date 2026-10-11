template<int, template<class> class...>
struct count;

template<template<class> class... F>
struct count<1, F...>
{
  static const int value = sizeof...(F);
};

template<class> struct first {};
template<class> struct second {};

static_assert(count<1, first, second>::value == 2, "");

int main() {}
