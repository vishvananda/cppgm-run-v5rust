int main() {
  int x = 1;
  auto f = [&x]() -> int { return x; };
  return f();
}
