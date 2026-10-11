struct B {
  int x;
  B() : x(7) {}
};

struct A {
  B b;
  A() {}
};

int main() {
  A a;
  return a.b.x == 7 ? 0 : 1;
}
