// N3485 focus: 5.19 [expr.const], invalid pointer arithmetic

constexpr int values[2] = {1, 2};
constexpr const int * invalid = values + 3;

int main()
{
  return invalid != 0;
}
