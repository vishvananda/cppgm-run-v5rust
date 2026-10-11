// VALIDATION: compile-fail

int runtime_value();

struct box
{
  constexpr explicit box(int input) : value(input) {}
  constexpr int constant_member() const { return 7; }

  int value;
};

static_assert(box(runtime_value()).constant_member() == 7, "");

int main()
{
  return 0;
}
