struct Analyzer {
  int f() {
    auto helper = [](int x) -> int { return x + 1; };
    auto outer = [&](int z) -> int {
      auto inner = [&](int y) -> int { return helper(y); };
      return inner(z);
    };
    return outer(4);
  }
};

int main() {
  Analyzer a;
  return a.f();
}
