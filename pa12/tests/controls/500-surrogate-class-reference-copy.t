// expect exit 0
// A local constructor owns its copy destination. A surrogate returning a
// reference contributes only its source evaluation to the initializer scan.
int copies;
struct Value {
  int n;
  Value(int v) : n(v) {}
  Value(const Value& other) : n(other.n) { ++copies; }
  ~Value() {}
};
Value value(7);
Value& get(int) { return value; }
typedef Value& (*Getter)(int);
struct Source { operator Getter() { return get; } };
int main() {
  Source source;
  Value local = source(1);
  return local.n != 7 || copies != 1;
}
