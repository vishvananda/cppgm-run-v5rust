// Deduced initialization must reuse original expression facts and conversions.
int value = 3;
int fetch() { return value; }
auto identity(int x) -> decltype(x) { return x; }
int main() {
  int* pointer = &value;
  auto*& alias = pointer;
  const auto* const& qualified = pointer;
  auto&& forwarded = value;
  auto copy = fetch();
  auto casted = static_cast<long>(copy);
  return identity(*alias + *qualified + forwarded + copy + casted);
}
