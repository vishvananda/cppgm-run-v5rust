// lowir.md no_inline, effects (compared here, in the -O0 driver lane, because
// PA27 and PA28 own the attributes): GNU noinline, pure and const on a
// lambda-declarator are its call operator's.
int main()
{
  int base = 1;
  auto kept = [&](int x) __attribute__((noinline)) { return x + base; };
  auto pure = [&](int x) __attribute__((pure)) { return x + base; };
  auto constant = [](int x) __attribute__((const)) { return x + 2; };
  return kept(0) + pure(0) + constant(0) == 4 ? 0 : 1;
}
