// expect exit 0
// A parameter list constructs its final object even when copying is deleted.
int constructions;
int destructions;
struct Value {
  int n;
  const void* self;
  Value(int v) : n(v), self(this) { ++constructions; }
  Value(const Value&) = delete;
  ~Value() { ++destructions; }
};
int take(Value value) { return value.self==&value ? value.n : -100; }
struct Consumer {
  int take(Value value) { return value.self==&value ? value.n : -100; }
  int operator()(Value value) { return value.self==&value ? value.n : -100; }
};
template<class T> int read(T value) { return value.self==&value ? value.n : -100; }
struct Holder { int n; Holder(Value value, int) : n(value.self==&value ? value.n : -100) {} };
int defaulted(Value value={6}) { return value.self==&value ? value.n : -100; }
int copies;
struct Copyable {
  int n;
  const void* self;
  Copyable(int v) : n(v), self(this) {}
  Copyable(const Copyable& other) : n(other.n), self(this) { ++copies; }
};
int copy_take(Copyable value) { return value.self==&value ? value.n : -100; }
int main() {
  Consumer consumer;
  int (*pointer)(Value)=take;
  int sum=take({1})+consumer.take({2})+consumer({3})+pointer({4})+read<Value>({5})+defaulted();
  Holder holder({7},0);
  Copyable source(8);
  int copied=copy_take({source});
  return copied==8 && copies==1 && sum==21 && holder.n==7 && constructions==7 && destructions==7 ? 0 : 1;
}
