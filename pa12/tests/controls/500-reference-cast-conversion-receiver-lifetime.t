// expect exit 0
// N3485 12.2p5: a reference cast extends the conversion result's lifetime;
// its receiver still ends with the declaration's full-expression.
int events;
struct Value {
  int n;
  Value(int value) : n(value) {}
  ~Value() noexcept(false) { events = events * 10 + n; }
};
struct Source {
  int n;
  Source() : n(7) {}
  operator Value() { return Value(n); }
  ~Source() noexcept(false) { events = events * 10 + 9; }
};
struct ReferenceSource {
  Value* value;
  ReferenceSource(Value* object) : value(object) {}
  operator Value&() { return *value; }
  ~ReferenceSource() noexcept(false) { events = events * 10 + 8; }
};
int main() {
  {
    const Value& value = (static_cast<const Value&>((Source())));
    if (events != 9 || value.n != 7) return 1;
  }
  if (events != 97) return 2;
  events = 0;
  {
    Value target(5);
    const Value& value = static_cast<const Value&>(ReferenceSource(&target));
    if (events != 8 || &value != &target) return 3;
  }
  return events != 85;
}
