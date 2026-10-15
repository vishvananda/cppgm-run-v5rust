// expect exit 0
// N3485 5.2.2p4, 12.2: indirect class results share the ordinary return
// destination; the conversion receiver survives its full expression.
int alive, destroyed;
struct Value {
  int n;
  Value(int v) : n(v) {}
  Value(const Value& v) : n(v.n) {}
  ~Value() { ++destroyed; }
};
Value make(int n) { return Value(alive == 1 ? n : -1); }
typedef Value (*Factory)(int);
struct Source {
  Source() { ++alive; }
  ~Source() { --alive; }
  operator Factory() { return make; }
};
Value forward(Source& s) { return s(7); }
int main() {
  {
    Source source;
    Value first = forward(source);
    if (first.n != 7 || alive != 1 || destroyed != 0) return 1;
  }
  if (alive || destroyed != 1) return 2;
  {
    Value second = Source()(9);
    if (second.n != 9 || alive || destroyed != 1) return 3;
  }
  return destroyed != 2;
}
