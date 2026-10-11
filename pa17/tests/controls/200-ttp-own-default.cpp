// N3485 14.1p9, 14.3.3: TT<char> inside W uses the template
// template-parameter's own default (int), not the argument template's (short);
// expect exit 0
template<class A, class B = short> struct P { static const int v = sizeof(B); };
template<template<class, class = int> class TT> struct W { static const int v = TT<char>::v; };
int main() { return W<P>::v == sizeof(int) ? 0 : 1; }
