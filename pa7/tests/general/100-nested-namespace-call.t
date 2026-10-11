namespace A {
  int g(int x) { return x; }
  namespace B {
    int f() { return g(0); }
  }
}
