// expect exit 0
int value = 3;
int count;
int fetch() { ++count; return value; }
auto identity(int x) -> decltype(x) { return x; }
int main() {
  int* pointer = &value;
  auto*& alias = pointer;
  const auto* const& qualified = pointer;
  auto&& forwarded = value;
  auto copy = fetch();
  forwarded = 7;
  auto casted = static_cast<long>(copy);
  return *alias == 7 && *qualified == 7 && copy == 3 && casted == 3 &&
         count == 1 && identity(2) == 2 ? 0 : 1;
}
