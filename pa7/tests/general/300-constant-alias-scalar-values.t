using integer = int;
using boolean = bool;
int object;
constexpr int value = integer{3};
constexpr int zero = integer();
constexpr bool truth = boolean(&object);
int result()
{
  return value + zero + truth;
}
