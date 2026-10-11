// VALIDATION: compile-pass
namespace n {
template<template<class...> class> struct accepts {};
template<class> struct probe {
  template<class U> static void check(::n::accepts<U::template fn>*);
};
}
struct missing {};
n::probe<missing>* value();
int main() { return 0; }
