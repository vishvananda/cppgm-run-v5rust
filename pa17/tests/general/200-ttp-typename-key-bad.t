// N3485 14.1p1 (shared/source.gram type-parameter: template <...> class only):
// typename in a template template-parameter is C++17; expect reject
template<template<class> typename TT> struct W {};
int main() { return 0; }
