struct A {
  bool test(int = 0) const volatile { return true; }
  bool test(int = 0) const { return false; }
};

bool f(const volatile A* a) {
  return a->test();
}

int main() {
  A a;
  return f(&a) ? 0 : 1;
}
