template<class...> struct X {};
template<class, template<class...> class> struct S;

template<template<class...> class L, class A, class... T, template<class...> class F>
struct S<L<A, T...>, F> { enum { value = 1 }; };

template<template<class...> class L, class A, class B, class... T, template<class...> class F>
struct S<L<A, B, T...>, F> { enum { value = 2 }; };

static_assert(S<X<int, int>, X>::value == 2, "");
int main() { return 0; }
