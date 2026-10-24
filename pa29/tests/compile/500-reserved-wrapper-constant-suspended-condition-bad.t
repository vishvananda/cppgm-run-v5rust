// A suspended condition has no declared variable and reports no invented value.
constexpr int wrapper() {
  auto value = __builtin_unprovided_auto();
  if (int condition = 1) return condition;
  return 0;
}
constexpr int value = wrapper();
