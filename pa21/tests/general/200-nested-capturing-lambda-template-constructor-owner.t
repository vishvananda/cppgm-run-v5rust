template<class Sig> struct Function;

template<class A>
struct Function<void(A)> {
  template<class F>
  Function(F f) { (void)f; }
  void operator()(A) {}
};

struct Scope {
  int value;
};

struct Analyzer {
  int lookup_functions(int base) {
    int result = 0;
    auto collect = [&base, &result]() {
      Function<void(Scope &)> visit = [&base, &result](Scope & current) {
        result = base + current.value;
      };
      Scope s{5};
      visit(s);
    };
    collect();
    return result;
  }
};

int main() {
  Analyzer a;
  return a.lookup_functions(7) - 12;
}
