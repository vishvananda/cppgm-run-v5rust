struct Vec2 {
  int x, y;
  constexpr Vec2(int a, int b) : x(a), y(b) {}
};

constexpr Vec2 v(1, 2);
static_assert(v.x == 1, "");
static_assert(v.y == 2, "");

int main() { return 0; }
