constexpr unsigned short s16 = __builtin_bswap16((unsigned short)0x1122u);
constexpr unsigned int s32 = __builtin_bswap32(0x11223344u);
constexpr unsigned long long s64 =
    __builtin_bswap64(0x0102030405060708ull);

static_assert(s16 == (unsigned short)0x2211u, "");
static_assert(s32 == 0x44332211u, "");
static_assert(s64 == 0x0807060504030201ull, "");

unsigned short h(unsigned short x) {
  return __builtin_bswap16(x);
}

unsigned int f(unsigned int x) {
  return __builtin_bswap32(x);
}

unsigned long long g(unsigned long long x) {
  return __builtin_bswap64(x);
}

int main() {
  return (int)h((unsigned short)0x0102u) +
         (int)f(0x01020304u) +
         (int)g(0x0102030405060708ull);
}
