// If-conversion shapes (min/max/abs/clamp, counting, two-value merges, narrow
// and pointer selects, a selected zero): LowIR select lowered to cmov at -O2+.
#include <cstdio>
__attribute__((noinline)) int imin(int a, int b) { return a < b ? a : b; }
__attribute__((noinline)) unsigned long umax(unsigned long a, unsigned long b) { return a > b ? a : b; }
__attribute__((noinline)) int iabs(int a) { return a < 0 ? -a : a; }
__attribute__((noinline)) const char* pick(bool c, const char* x, const char* y) { return c ? x : y; }
__attribute__((noinline)) int clampi(int v, int lo, int hi) { if (v < lo) v = lo; if (v > hi) v = hi; return v; }
__attribute__((noinline)) int cnt(const int* p, int n, int k) { int c = 0; for (int i = 0; i < n; ++i) if (p[i] == k) ++c; return c; }
__attribute__((noinline)) long two(long a, long b, bool c) { long x = 1, y = 2; if (c) { x = a + b; y = a - b; } return x * 10 + y; }
__attribute__((noinline)) unsigned char bsel(unsigned char a, unsigned char b, int c) { return c > 3 ? a : b; }
__attribute__((noinline)) short ssel(short a, int c) { return c ? a : (short)-5; }
__attribute__((noinline)) long zsel(long a, long c) { return c == 7 ? 0 : a; }
int main() {
  int arr[50]; for (int i = 0; i < 50; ++i) arr[i] = (i * 7) % 5;
  long s = 0;
  for (int i = -20; i < 20; ++i) for (int j = -3; j < 4; ++j) {
    s += imin(i, j) * 3 + iabs(i * j) + clampi(i, -5, 7) + two(i, j, (i ^ j) & 1) + bsel((unsigned char)i, (unsigned char)j, j) + ssel((short)i, j & 1) + zsel(i, j + 7);
    s += (long)umax((unsigned long)i, (unsigned long)j) & 0xffff;
  }
  printf("%ld %d %s %s\n", s, cnt(arr, 50, 3), pick(true, "a", "b"), pick(false, "a", "b"));
}
