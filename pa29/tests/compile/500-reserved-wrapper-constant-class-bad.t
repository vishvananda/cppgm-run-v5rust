// A deferred class argument has no selected constructor or expression value.
struct Value { constexpr Value(int) {} };
constexpr int wrapper() {
  Value value((__builtin_unprovided_class()));
  return 0;
}
constexpr int value = wrapper();
