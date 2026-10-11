// VALIDATION: compile-fail

int runtime_value();

constexpr int evaluate()
{
  int ignored = runtime_value();
  return 1;
}

static_assert(evaluate() == 1, "");

int main()
{
  return 0;
}
