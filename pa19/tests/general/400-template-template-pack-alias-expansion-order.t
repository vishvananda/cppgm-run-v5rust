template<class> struct A {};
template<class> struct B {};
template<template<class> class...> struct L {};
template<template<class> class... X> struct C { using R = L<X...>; };
using R = decltype(true ? L<A, B>() : C<A, B>::R());
int main() {}
