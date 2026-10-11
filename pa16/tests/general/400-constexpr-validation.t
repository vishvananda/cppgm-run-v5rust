constexpr int factorial(int n) { return n <= 1 ? 1 : n * factorial(n - 1); }

static_assert(factorial(5) == 120, "");

constexpr int x = factorial(6);
static_assert(x == 720, "");

struct Lit {
  int value;
  constexpr Lit(int v) : value(v) {}
};

constexpr Lit lit(42);
static_assert(lit.value == 42, "");

int main() { return 0; }
