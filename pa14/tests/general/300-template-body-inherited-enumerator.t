struct base {
  enum part { none = 0, symbol = 7 };
};

template<class T>
struct derived : base {
  int f() { return symbol; }
};

int main() {
  derived<int> d;
  return d.f();
}
