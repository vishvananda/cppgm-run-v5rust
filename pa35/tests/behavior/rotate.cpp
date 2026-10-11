// A std::rotate over vectors of several element types: an index of an
// index whose base stops being lazy after the first composed through it
// (find_addresses) once miscompiled at -O2/-O3 (p62..p105).
#include <algorithm>
#include <vector>
#include <cstdio>
#include <cstdint>
struct T { int a, b, c; };
int main() {
  unsigned long h = 0;
  for (int n = 1; n < 40; ++n) for (int k = 0; k <= n; ++k) {
    std::vector<unsigned> v(n); for (int i = 0; i < n; ++i) v[i] = i * 7 + 1;
    std::rotate(v.begin(), v.begin() + k, v.end());
    std::vector<T> w(n); for (int i = 0; i < n; ++i) w[i] = T{ i, i * 3, i * 5 };
    std::rotate(w.begin(), w.begin() + k, w.end());
    std::vector<uint64_t> x(n); for (int i = 0; i < n; ++i) x[i] = i * 11;
    std::rotate(x.begin(), x.begin() + (n - k), x.end());
    for (int i = 0; i < n; ++i) h = h * 31 + v[i] + w[i].b + x[i];
  }
  printf("%lu\n", h);
}
