// An unreported interpreter failure is diagnosed at the required expression.
constexpr int wrapper() {
  auto value = __builtin_unprovided_auto();
  if (int condition = 1) return condition;
  return 0;
}
static_assert(wrapper(), "suspended condition");
