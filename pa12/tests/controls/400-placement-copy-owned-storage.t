// expect exit 0
// The allocated copy/move owns construction. Its glvalue source does not
// create a second temporary; an initializer prvalue still does.
void* operator new(unsigned long, void* storage) noexcept { return storage; }
int copies;
int moves;
int destroyed;
struct Value {
  int n;
  Value(int value) : n(value) {}
  Value(const Value& other) : n(other.n) { ++copies; }
  Value(Value&& other) : n(other.n) { ++moves; }
  ~Value() { ++destroyed; }
};
Value& source(Value& value) { return value; }
Value* copy_at(void* storage, Value& value) {
  return new(storage) Value(source(value));
}
Value* move_at(void* storage, Value& value) {
  return new(storage) Value(static_cast<Value&&>(value));
}
Value* temporary_at(void* storage) { return new(storage) Value(Value(9)); }
int main() {
  Value original(7);
  alignas(Value) char storage[sizeof(Value)];
  Value* copied = copy_at(storage, original);
  if (copied->n != 7 || copies != 1 || moves != 0 || destroyed != 0) return 1;
  copied->~Value();
  Value* moved = move_at(storage, original);
  if (moved->n != 7 || copies != 1 || moves != 1 || destroyed != 1) return 2;
  moved->~Value();
  Value* temporary = temporary_at(storage);
  if (temporary->n != 9 || destroyed != 2) return 3;
  temporary->~Value();
  return destroyed == 3 ? 0 : 4;
}
