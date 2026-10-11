// VALIDATION: compile-pass
// N3485 focus: 7.1.5 [dcl.constexpr]

struct Source
{
  int value;

  constexpr explicit Source(int v) : value(v) {}
  constexpr int count() const { return value; }

  static constexpr Source zero() { return Source(0); }
  static constexpr Source min() { return Source(-1); }
};

struct Common
{
  int value;

  constexpr Common(const Source & other) : value(other.count()) {}
  constexpr int count() const { return value; }
};

constexpr bool less_than(const Source & lhs, const Source & rhs)
{
  return Common(lhs).count() < Common(rhs).count();
}

static_assert(less_than(Source::min(), Source::zero()), "");

int main()
{
  return 0;
}
