// VALIDATION: compile-pass
// An ordinary function and a tag may share a name regardless of declaration
// order, and a qualified call resolves the ordinary function.

constexpr int item(int first, int second)
{
  return first + second;
}

struct item
{
  constexpr item()
  {
  }
};

constexpr int value = ::item(2, 4);
static_assert(value == 6, "qualified call selects the function");

int main()
{
  return value - 6;
}
