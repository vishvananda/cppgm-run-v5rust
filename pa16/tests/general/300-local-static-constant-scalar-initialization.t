// VALIDATION: compile-pass
// A function-local static with a constant initializer needs neither a dynamic
// guard nor the otherwise-unused constexpr initializer function at runtime.

inline constexpr int seed()
{
  return 7;
}

int read()
{
  static int value = seed();
  return value;
}

int main()
{
  return read() == 7 ? 0 : 1;
}
