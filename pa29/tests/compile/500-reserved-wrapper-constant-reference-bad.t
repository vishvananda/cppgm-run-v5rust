// Binding a deferred reference in constant evaluation has no fabricated referent.
constexpr int wrapper() {
  const int& value = (__builtin_unprovided_reference());
  return 0;
}
constexpr int value = wrapper();
