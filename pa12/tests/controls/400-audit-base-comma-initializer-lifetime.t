// A comma's effects precede base construction and end before the body; expect exit 0
int live;
int made;
int destroyed;
int bad;
struct T {
  T() { ++live; ++made; }
  ~T() { --live; }
};
struct A {
  int n;
  A() : n(live) {}
  ~A() { ++destroyed; }
};
struct B : A {
  B() : A((T(), A())) { if (live || n != 1) ++bad; }
};
struct C : A {
  C() : A{(T(), A())} { if (live || n != 1) ++bad; }
};
int main() {
  {
    B b;
    C c;
    if (live || made != 2 || destroyed || bad) return 1;
  }
  return live || destroyed != 2 || bad;
}
