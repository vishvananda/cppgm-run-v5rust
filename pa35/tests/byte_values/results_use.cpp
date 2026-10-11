#include <cstdio>
bool ext_b(long); unsigned char ext_u(long); signed char ext_s(long);
static unsigned char sink[16]; static int si;
__attribute__((noinline)) long g(long x) {
  long s = 0;
  if (ext_b(x)) s += 1;
  unsigned char u = ext_u(x); if (u) s += 2; sink[si++ & 15] = u;
  signed char c = ext_s(x); if (c == 5) s += 4; if (c) s += 8;
  bool b2 = ext_b(x + 1); bool b3 = ext_b(x - 1); if (b2 && b3) s += 16; if (b2 != b3) s += 32;
  s += ext_u(x + 2) & 3; s += ext_s(x - 2) * 2;
  return s + sink[(si - 1) & 15];
}
int main() { long t = 0; for (long x = -40; x < 300; ++x) t = t * 31 + g(x); printf("%ld\n", t); }
