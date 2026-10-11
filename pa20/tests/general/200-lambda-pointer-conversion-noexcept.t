// CWG 1722: conversion to a function pointer does not invoke the lambda body.
struct Wrapper { Wrapper(int (*)(int)) noexcept {} };
int main()
{
  auto closure = [](int n) { return n + 1; };
  static_assert(noexcept(+closure), "pointer conversion is nonthrowing");
  static_assert(!noexcept(closure(4)), "call operator remains potentially throwing");
  static_assert(noexcept(Wrapper(closure)), "constructor argument conversion is nonthrowing");
  return 0;
}
