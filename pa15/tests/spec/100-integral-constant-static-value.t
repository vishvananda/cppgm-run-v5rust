// VALIDATION: compile-pass
// N3485 focus: 14.3.2 [temp.arg.nontype]

template<class T, T v>
struct integral_constant
{
  static const T value = v;
};

static_assert(integral_constant<int, 3>::value == 3, "integral_constant");

int main()
{
  return 0;
}
