// expect reject
// A dependent template-name sees first-declaration defaults, but not defaults
// added after its instantiation context was recorded.
template<template<class...> class TT> struct Wrap { using type = TT<int>; };
template<class T, class U> struct A;
template<class T, class U = int> struct A {};
using R = Wrap<A>::type;
int main() { return 0; }
