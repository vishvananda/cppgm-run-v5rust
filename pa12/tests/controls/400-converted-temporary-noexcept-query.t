// Frozen may_throw treats a converted builtin/contextual operand as terminal:
// its converter and receiver destructor count; receiver construction does not.
// Ordinary scalar initialization retains the original expression query.
// expect exit 0
struct Value {
  Value();
  ~Value() noexcept;
  operator int() noexcept;
};
struct Destruction {
  Destruction() noexcept;
  ~Destruction() noexcept(false);
  operator int() noexcept;
};
struct Conversion {
  Conversion() noexcept;
  ~Conversion() noexcept;
  operator int();
};
void take(int) noexcept;
int main() {
  if (!noexcept(+Value())) return 1;
  if (!noexcept(!Value())) return 2;
  if (!noexcept(Value() + 1)) return 3;
  if (!noexcept(Value() && true)) return 4;
  if (noexcept(static_cast<int>(Value()))) return 5;
  if (noexcept(take(Value()))) return 6;
  if (noexcept(+Destruction())) return 7;
  if (noexcept(!Destruction())) return 8;
  if (noexcept(+Conversion())) return 9;
  if (noexcept(!Conversion())) return 10;
  return 0;
}
