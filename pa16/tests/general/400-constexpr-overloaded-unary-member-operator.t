struct duration
{
  int rep;

  constexpr explicit duration(int value) : rep(value) {}

  constexpr int count() const
  {
    return rep;
  }

  constexpr duration operator-() const
  {
    return duration(-rep);
  }
};

constexpr duration source(3);
constexpr duration negated = -source;

static_assert(negated.count() == -source.count(),
              "constexpr overloaded unary member operator");

int main()
{
  return negated.count() == -3 ? 0 : 1;
}
