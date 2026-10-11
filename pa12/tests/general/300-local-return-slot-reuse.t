struct S {
  int x;
  S() : x(7) {}
  S(const S& o) : x(o.x) {}
};

S f() { S s; return s; }

int main() {
  S s = f();
  return s.x == 7 ? 0 : 1;
}
