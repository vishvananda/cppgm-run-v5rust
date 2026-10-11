// VALIDATION: compile-pass
// A floating initializer is rounded to the target object precision.

constexpr float value = 16777217.0;
static_assert(value == 16777216.0f, "initializer rounds to float");

int main()
{
  return 0;
}
