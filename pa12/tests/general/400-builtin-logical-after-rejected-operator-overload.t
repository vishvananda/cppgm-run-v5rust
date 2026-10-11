// VALIDATION: compile-pass

namespace library
{
struct parser {};

int operator&&(parser const&, parser const&);

struct match
{
  bool state;

  explicit match(bool value) : state(value) {}

  operator bool() const
  {
    return state;
  }
};

bool both(match const& left, match const& right)
{
  return left && right;
}
}

int main()
{
  return library::both(library::match(true), library::match(true)) ? 0 : 1;
}
