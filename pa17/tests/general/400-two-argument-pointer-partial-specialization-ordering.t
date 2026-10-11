// VALIDATION: compile-pass
// A partial specialization constraining both arguments as pointers is more
// specialized than one constraining only the first argument.

template<class First, class Second>
struct select
{
  static const int value = 0;
};

template<class First, class Second>
struct select<First *, Second>
{
  static const int value = 1;
};

template<class First, class Second>
struct select<First *, Second *>
{
  static const int value = 2;
};

static_assert(select<int, int>::value == 0, "primary");
static_assert(select<int *, int>::value == 1, "one pointer");
static_assert(select<int *, int *>::value == 2, "two pointers");

int main()
{
  return 0;
}
