// A demanded copy default's class parameter needs its implicit destructor; expect exit 0
int live = 0;
struct A {
  A() { ++live; }
  ~A() { --live; }
};
struct B { A a, b; };
struct C {
  C() {}
  C(const C&, B = B()) {}
};
void copy(C& c) { C b(c); }
int main() {
  C c;
  copy(c);
  copy(c);
  return live == 0 ? 0 : 1;
}
