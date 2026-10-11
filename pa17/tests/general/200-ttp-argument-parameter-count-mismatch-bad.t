// N3485 14.3.3p3 (example X<B>): B<T, U = T> does not match template<class>
// class P; default arguments are ignored (g++ rejects; clang accepts per
// P0522); expect reject
template<class T, class U = T> struct B {};
template<template<class> class P> struct X { static const int v = 1; };
int main() { return X<B>::v - 1; }
