static_assert(__builtin_clz(0x31) == 26, "");
static_assert(__builtin_clzl(0x31ul) == (int)(sizeof(unsigned long) * 8 - 6), "");
static_assert(__builtin_clzll(0x31ull) == 58, "");
static_assert(__builtin_popcount((unsigned)(signed char)-1) == 32, "");
int main() { return 0; }
