struct Analyzer;

bool accept(Analyzer &);

struct Analyzer {
  int f() {
    auto outer = [&]() -> bool {
      auto inner = [&]() -> bool {
        return accept(*this);
      };
      return inner();
    };
    return outer() ? 1 : 0;
  }
};

bool accept(Analyzer &) { return true; }

int main() {
  Analyzer a;
  return a.f();
}
