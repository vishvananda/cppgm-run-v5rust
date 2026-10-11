#if !__has_builtin(__builtin_popcountg)
#error missing popcountg builtin
#endif
static_assert(__builtin_popcountg(~0ul) == sizeof(unsigned long) * 8, "");
static_assert(__builtin_popcountg(0x31ul) == 3, "");
int main() { return 0; }
