struct Vec2 {
  int x, y;
  constexpr Vec2(int a, int b) : x(a), y(b) {}
  constexpr int sum() const { return x + y; }
  constexpr int dot(const Vec2 & other) const { return x * other.x + y * other.y; }
};

constexpr Vec2 v(3, 4);
constexpr Vec2 w(1, 2);
static_assert(v.sum() == 7, "");
static_assert(v.dot(w) == 11, "");

int main() { return 0; }
