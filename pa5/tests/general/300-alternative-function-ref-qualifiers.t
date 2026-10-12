struct A {
  void l() bitand;
  void r() and;
  int f() bitand { return 1; }
  operator int() and { return 2; }
};
void g() noexcept([] { struct B { void f() and; }; return true; }());
