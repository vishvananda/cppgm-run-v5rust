struct Value {
  int n;
  Value() : n(1) {}
  Value(const Value&, int n = 7) : n(n) {}
};
struct Source { Value* value; operator Value&() { return *value; } };
int consume(Value value) { return value.n; }
int main() { Value value; Source source = {&value}; return consume(source) - 7; }
