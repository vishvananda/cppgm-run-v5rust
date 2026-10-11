// VALIDATION: compile-pass
// N3485 focus: 7.1.5 [dcl.constexpr]

struct Point
{
  int value;

  constexpr Point(int v) : value(v) {}
  constexpr int get() const { return value; }
};

constexpr int square(int x)
{
  return x * x;
}

constexpr Point p(square(3));

static_assert(square(4) == 16, "constexpr function should evaluate in constant expression");
static_assert(p.get() == 9, "constexpr constructor should produce constant object");

int main()
{
  return 0;
}
