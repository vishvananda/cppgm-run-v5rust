// N3485 14.6.1p1: inside A, the injected-class-name A used as a template
// template-argument names the class template A; expect exit 0
template<template<class> class P> struct X { typedef P<char> type; };
template<class T> struct A { T t[3]; typedef typename X<A>::type other; };
int main() { return sizeof(A<int>::other) == 3 ? 0 : 1; }
