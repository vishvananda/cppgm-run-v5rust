// VALIDATION: compile-pass
// A cv-qualified parameter partial outranks a generic template-id partial.

template<class T> struct holder { static const int value = 0; };
template<class T> struct holder<T const> { static const int value = 1; };

template<template<class...> class L, class... T>
struct holder<L<T...> > { static const int value = 2; };

template<class T> struct box {};

static_assert(holder<box<int> const>::value == 1, "cv partial");

int main() { return 0; }
