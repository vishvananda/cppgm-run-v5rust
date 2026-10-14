auto identity(int x) -> decltype(x) { return x; }
int main() {
  auto first = static_cast<long>(3);
  auto second = (unsigned long)4;
  auto third = long(5);
  return identity(first + second + third);
}
