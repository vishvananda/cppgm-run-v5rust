static_assert(__builtin_clz(1u) == 31, "");
static_assert(__builtin_clzl(1ul) == sizeof(unsigned long) * 8 - 1, "");
static_assert(__builtin_clzll(1ull) == 63, "");
static_assert(__builtin_clz((unsigned char)0x31) == 26, "");
static_assert(__builtin_clzl((unsigned char)0x31) == sizeof(unsigned long) * 8 - 6, "");
static_assert(__builtin_clzll((unsigned char)0x31) == 58, "");

static_assert(__builtin_ctz(8u) == 3, "");
static_assert(__builtin_ctzl(8ul) == 3, "");
static_assert(__builtin_ctzll(8ull) == 3, "");

static_assert(__builtin_popcount(0x31u) == 3, "");
static_assert(__builtin_popcountl(0x31ul) == 3, "");
static_assert(__builtin_popcountll(0x31ull) == 3, "");
static_assert(__builtin_popcount((signed char)-1) == 32, "");

int f(unsigned x, unsigned long y, unsigned long long z) {
  return __builtin_clz(x) +
         __builtin_clzl(y) +
         __builtin_clzll(z) +
         __builtin_ctz(x) +
         __builtin_ctzl(y) +
         __builtin_ctzll(z) +
         __builtin_popcount(x) +
         __builtin_popcountl(y) +
         __builtin_popcountll(z);
}

int main() {
  return f(8u, 8ul, 0x31ull);
}
