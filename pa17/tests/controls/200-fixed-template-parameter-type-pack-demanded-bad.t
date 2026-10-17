// N3485 14.3.3, 14.5.3: after substitution a type argument cannot name
// the template required by the fixed template-template parameter.
// expect reject
template<template<class> class> struct Target;
template<class... T> struct Deferred {
    using type = Target<T...>;
};
Deferred<int>::type* value;
