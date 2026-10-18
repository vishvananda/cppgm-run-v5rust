// N3485 14.3.3, 14.6.2: template template-parameter qualifier roots keep
// their argument packs, default arguments and nested member order; expect exit 0
template<class...> struct P { static const int value = 1; };
template<class, class...> struct Q { static const int value = 2; };
template<class...> struct R { struct B { static const int value = 3; }; };
template<class, class> struct S { static const int value = 4; };
template<template<class...> class TT, class... T>
auto a(TT<T...>) -> decltype(TT<T...>::value) { return 1; }
template<template<class, class...> class TT, class... T>
auto b(TT<int, T...>) -> decltype(TT<int, T...>::value) { return 2; }
template<template<class...> class TT, class... T>
auto c(TT<T...>) -> decltype(TT<T...>::B::value) { return 3; }
template<template<class A, class B = A> class TT>
auto d(TT<int, int>) -> decltype(TT<int>::value) { return 4; }
int main() {
    return a(P<>()) + a(P<int, long>()) + b(Q<int, long>())
        + c(R<int, long>()) + d(S<int, int>()) == 11 ? 0 : 1;
}
