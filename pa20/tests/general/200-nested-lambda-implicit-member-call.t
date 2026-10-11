struct E {
  int x;
  E() : x(0) {}
  E(int v) : x(v) {}
};

struct A {
  E make_e() { return E(1); }
  int f() {
    auto outer = [&]() -> int {
      auto inner = [&]() -> int {
        return make_e().x;
      };
      return inner();
    };
    return outer();
  }
};

int main() {
  A a;
  return a.f();
}
