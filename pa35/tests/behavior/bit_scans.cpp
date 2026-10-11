// __builtin_ctz/clz at 32 and 64 bits (LowIR ctz/clz, bsf/bsr), popcount,
// a set-bit walk, and constant folding of the scans.
#include <cstdio>
#include <cstdint>
__attribute__((noinline)) int c1(unsigned x) { return __builtin_ctz(x); }
__attribute__((noinline)) int c2(unsigned long x) { return __builtin_ctzl(x); }
__attribute__((noinline)) int l1(unsigned x) { return __builtin_clz(x); }
__attribute__((noinline)) int l2(unsigned long long x) { return __builtin_clzll(x); }
__attribute__((noinline)) int p1(unsigned x) { return __builtin_popcount(x); }
__attribute__((noinline)) int mix(uint32_t m) { int s = 0; while (m) { int r = __builtin_ctz(m); m &= m - 1; s = s * 3 + r; } return s; }
constexpr int k1 = __builtin_ctz(8u) + __builtin_clzll(1ull);
int main() {
  unsigned long h = k1;
  for (int i = 0; i < 64; ++i) {
    unsigned long v = (1ul << i) | (i > 3 ? (1ul << (i - 3)) : 0);
    unsigned u = (unsigned)(v | (v >> 32)) | 0x100000u;
    h = h * 31 + c1(u) + c2(v) * 7 + l1(u) * 13 + l2(v) * 17 + p1(u) * 19 + mix(u);
  }
  printf("%lu\n", h);
}
