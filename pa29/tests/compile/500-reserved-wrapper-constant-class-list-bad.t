// An untyped constructor list retains its distinct interpreter diagnostic.
struct Value { constexpr Value(int) {} };
constexpr int wrapper() {
  Value value{__builtin_unprovided_class()};
  return 0;
}
constexpr int value = wrapper();
