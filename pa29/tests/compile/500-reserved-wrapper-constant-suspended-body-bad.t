// Statements suspended by an undeducible initializer have no interpreter facts.
constexpr int wrapper() {
  auto value = __builtin_unprovided_auto();
  return 1;
}
constexpr int value = wrapper();
