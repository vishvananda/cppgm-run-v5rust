// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], constructor-template deduction through a
// derived class whose base explicitly carries an enclosing default binding.

template<class> struct alloc;

template<class Sub, template<class> class A = alloc>
struct base {};

struct derived : base<derived, ::alloc> {};

template<template<class> class A = alloc>
struct target {
  template<class Sub>
  target(const base<Sub, A> &) {}
};

int main() {
  derived source;
  target<> result(source);
}
