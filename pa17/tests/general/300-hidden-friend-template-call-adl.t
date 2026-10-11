struct Value {
  int n;
};

struct Adl {
  int delta;

  template<class T>
  friend int apply(T &value, const Adl &a) {
    value.n = value.n + a.delta;
    return value.n;
  }
};

int main() {
  Value value;
  Adl adl;
  value.n = 4;
  adl.delta = 5;
  return apply(value, adl) == 9 && value.n == 9 ? 0 : 1;
}
