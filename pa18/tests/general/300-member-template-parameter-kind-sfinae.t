// VALIDATION: compile-pass
struct A { template<int> struct member; };
template<template<class> class> struct helper {};
template<class T> char check(helper<T::template member>*);
template<class> long check(...);
static_assert(sizeof(check<A>(0)) == sizeof(long), "");
int main() {}
