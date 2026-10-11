struct A {
  bool dep(int) const { return true; }

  bool f() const {
    auto l = [&](int x) { return this->dep(x); };
    return l(0);
  }
};

int main() {
  A a;
  return a.f() ? 0 : 1;
}
