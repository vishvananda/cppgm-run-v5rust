int main() {
  int x = 3;
  auto* p = &x;
  auto& y = x;
  auto&& z = x;
  y = 5;
  return *p + z - 10;
}
