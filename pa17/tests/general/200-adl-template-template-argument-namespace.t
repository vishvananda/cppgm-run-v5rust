template<template<class> class TT>
struct Wrap {};

namespace N {
template<class>
struct Alloc {};

int adl_probe(Wrap<Alloc>) {
  return 0;
}
}

int main() {
  Wrap<N::Alloc> w;
  return adl_probe(w);
}
