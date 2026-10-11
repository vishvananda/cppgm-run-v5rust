class Box { public: int x; };

int main() {
  Box b;
  b.x = 3;
  auto f = [b](int y) -> int { return b.x + y; };
  b.x = 9;
  return f(4);
}
