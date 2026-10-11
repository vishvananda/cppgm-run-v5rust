// VALIDATION: compile-fail
// A non-dependent class-scope static_assert is checked even when the class is unused.

struct X
{
  static_assert(sizeof(char[2]) != 2, "bad");
};

int main()
{
  return 0;
}
