// VALIDATION: compile-pass
// Repeating a using-declaration for the same template is harmless.

namespace library
{
template<class T>
struct box
{
  T value;
};
}

using library::box;
using library::box;

int main()
{
  box<int> value;
  value.value = 0;
  return value.value;
}
