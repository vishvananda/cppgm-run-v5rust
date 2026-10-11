// VALIDATION: compile-fail
// A constexpr scalar object requires an initializer.

constexpr int value;

int main()
{
  return value;
}
