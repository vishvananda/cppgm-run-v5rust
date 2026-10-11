#if !__has_builtin(__builtin_popcountg)
#error missing popcountg builtin
#endif

static_assert(__builtin_popcountg(0x31u) == 3, "");

int f(unsigned long x)
{
  return __builtin_popcountg(x);
}
