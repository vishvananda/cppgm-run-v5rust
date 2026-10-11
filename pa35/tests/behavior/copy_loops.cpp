// Forward copy loops of 1-, 4- and 8-byte elements at every overlap
// (loop_passes.cpp copy idiom: memmove unless the destination starts
// strictly inside the source range), and vector growth through them.
#include <cstdio>
#include <vector>
#include <cstring>
__attribute__((noinline)) char* fwd(char* d, const char* s, const char* e) { for (; s != e; ++s, ++d) *d = *s; return d; }
__attribute__((noinline)) unsigned* fwd32(unsigned* d, const unsigned* s, const unsigned* e) { while (s != e) *d++ = *s++; return d; }
__attribute__((noinline)) long* fwd64(long* d, long* s, long* e) { for (; s != e; ++s, ++d) *d = *s; return d; }
int main() {
  unsigned long h = 0;
  for (int n = 0; n < 40; ++n) for (int off = -6; off <= 6; ++off) {
    char buf[128]; for (int i = 0; i < 128; ++i) buf[i] = (char)(i * 7 + 3);
    char* s = buf + 40; char* d = buf + 40 + off;
    char* r = fwd(d, s, s + n);
    for (int i = 0; i < 128; ++i) h = h * 31 + (unsigned char)buf[i];
    h = h * 31 + (unsigned long)(r - buf);
    unsigned u[128]; for (int i = 0; i < 128; ++i) u[i] = i * 11;
    unsigned* ru = fwd32(u + 40 + off, u + 40, u + 40 + n);
    for (int i = 0; i < 128; ++i) h = h * 31 + u[i];
    h = h * 31 + (unsigned long)(ru - u);
    long l[128]; for (int i = 0; i < 128; ++i) l[i] = i * 13;
    long* rl = fwd64(l + 40 + off, l + 40, l + 40 + n);
    for (int i = 0; i < 128; ++i) h = h * 31 + l[i];
    h = h * 31 + (unsigned long)(rl - l);
  }
  std::vector<char> v; for (int i = 0; i < 10000; ++i) { char t[7] = { 'a', 'b', (char)i, 'd', 'e', 'f', 'g' }; v.insert(v.end(), t, t + (i % 7)); }
  for (char c : v) h = h * 31 + c;
  std::vector<unsigned char> w; for (int i = 0; i < 20000; ++i) w.push_back((unsigned char)i);
  for (unsigned char c : w) h = h * 31 + c;
  printf("%lu\n", h);
}
