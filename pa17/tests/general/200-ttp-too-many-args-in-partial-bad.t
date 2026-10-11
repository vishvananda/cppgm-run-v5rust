// N3485 14.3p? / 14.3.3: TT<T, T> passes two arguments to a template
// template-parameter declared with one; expect reject
template<class T> struct Q {};
template<template<class> class TT, class T> struct Q<TT<T, T> > {};
int main() { return 0; }
