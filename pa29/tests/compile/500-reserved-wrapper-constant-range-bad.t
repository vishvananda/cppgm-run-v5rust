// A deferred range has no typed loop equivalent for constant evaluation.
constexpr int wrapper() {
  for (int value : __builtin_unprovided_range()) { return value; }
  return 0;
}
constexpr int value = wrapper();
