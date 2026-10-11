// VALIDATION: compile-fail
// A constexpr scalar object may not be initialized by a runtime call.

int runtime_value();

constexpr int value = runtime_value();

int main()
{
  return value;
}
