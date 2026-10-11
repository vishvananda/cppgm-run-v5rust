// VALIDATION: compile-fail
// Retaining a mutable static object's identity for address constant expressions
// must not make its stored value readable during constant evaluation.

constexpr int read_value(int &value)
{
  return value;
}

static int mutable_value = 7;
static_assert(read_value(mutable_value) == 7,
              "a mutable static object's value is not constant");

int main()
{
  return 0;
}
