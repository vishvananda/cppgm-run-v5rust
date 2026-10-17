// N3485 14.3.3, 14.5.3: an unresolved expansion leaves fixed argument
// positions unknown; template-name checking waits for substitution.
// expect exit 0
template<template<class> class> struct Target;
template<class... T> struct UnusedClass {
    using type = Target<T...>;
};
template<class... T> using UnusedAlias = Target<T...>;
int main() { return 0; }
