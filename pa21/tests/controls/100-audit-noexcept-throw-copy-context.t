// expect exit 0
// A throw operand's initialization has no hidden return/condition EH context.
int copies, moves, drops;
struct Value {
  Value() {}
  Value(const Value&) noexcept { ++copies; }
  Value(Value&&) noexcept { ++moves; }
  ~Value() { ++drops; }
};
struct Base { virtual ~Base() {} };
struct Derived : Base {};
void local_copy() {
  try {
    const Value value;
    throw (value);
  } catch (const Value&) {}
}
void local_move() {
  try {
    Value value;
    throw value;
  } catch (Value&) {}
}
void parameter_copy(Value value) {
  try { throw value; } catch (Value&) {}
}
void polymorphic_parameter(Derived derived) {
  try { throw derived; } catch (Base&) {}
}
int main() {
  local_copy();
  local_move();
  Value value;
  parameter_copy(value);
  try {
    Derived derived;
    throw derived;
  } catch (Base&) {}
  return copies != 3 || moves != 1 || drops != 6;
}
