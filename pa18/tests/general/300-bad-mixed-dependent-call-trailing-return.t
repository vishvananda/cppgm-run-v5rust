// VALIDATION: compile-fail
// The dependent call does not defer the separate non-dependent lookup.

template<class F, class... Ts>
auto skipped(F f, Ts... args) -> decltype(f(args...), missing());

int main()
{
  return 0;
}
