// VALIDATION: compile-pass

int f(int);
struct twice {
  constexpr int operator()(int x) const { return x * 2; }
};
template<class F> constexpr int invoke(F f, int x) { return f(x); }
static_assert(invoke(twice(), 3) == 6, "");
int main() { return 0; }
