// VALIDATION: compile-pass
// A qualified call through the current instantiation must collect member
// function templates before resolving a detector's decltype.

template<class T> T&& declval();

template<class Func>
struct detector
{
  template<class F = Func, class = decltype(declval<F>()())> static char test(int);
  static int test(...);
  using type = decltype(detector::test(0));
};

struct callable { bool operator()() const; };

static_assert(sizeof(detector<callable const&>::type) == 1,
              "the function-template probe must beat the fallback");

int main() { return 0; }
