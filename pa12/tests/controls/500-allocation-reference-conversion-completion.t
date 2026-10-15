// expect exit 0
// N3485 5.3.4p18: reference conversion arguments retain the allocation
// full-expression's completion region after the storage cleanup ends.
int copies;
int moves;
struct Value {
  int n;
  Value(int v) : n(v) {}
  Value(const Value& other) : n(other.n) { ++copies; }
  Value(Value&& other) : n(other.n) { ++moves; }
  ~Value() {}
};
struct Source {
  Value value;
  Source() : value(7) {}
  operator Value&() { return value; }
  ~Source() {}
};
struct ExplicitSource {
  Value value;
  ExplicitSource() : value(9) {}
  explicit operator Value&&() { return static_cast<Value&&>(value); }
  ~ExplicitSource() {}
};
int main() {
  Source source;
  ExplicitSource explicit_source;
  Value* copied = new Value(source);
  Value* moved = new Value{explicit_source};
  int result = copied->n != 7 || moved->n != 9 || copies != 1 || moves != 1;
  delete copied;
  delete moved;
  return result;
}
