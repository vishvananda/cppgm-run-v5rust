int main() {
  int x = 3;
  auto f = [x](int y) -> int { return x + y; };
  return f(4);
}
