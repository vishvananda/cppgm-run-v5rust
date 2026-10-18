// expect exit 0
// PA18: retained invocation operands use ordinary callable deduction once.
int increment(int n) { return n + 1; }
constexpr int constant(int n) { return n + 2; }
constexpr int result = __builtin_invoke(constant, 3);
static_assert(result == 5, "constant invocation");

struct callable
{
  template<class T> T& operator()(T& value) & { return value; }
  template<class T> const T& operator()(const T& value) const & { return value; }
};

callable object;
callable& get() { return object; }

struct surrogate
{
  typedef int (*pointer)(int);
  operator pointer() { return increment; }
};

int main()
{
  int n = 3;
  const callable c = {};
  int (*p)(int) = increment;
  surrogate s;
  return &__builtin_invoke(get(), n) == &n
      && &__builtin_invoke(c, n) == &n
      && __builtin_invoke(increment, __builtin_invoke(p, 3)) == 5
      && ::__builtin_invoke(s, 3) == 4
      && (__builtin_invoke)(increment, 3) == 4 ? 0 : 1;
}
