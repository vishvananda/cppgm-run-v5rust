int main() {
  int value = 1;
  auto outer = [&]() {
    auto inner = [&]() { int value = 2; return value; };
    return value + inner();
  };
  return outer() - 3;
}
