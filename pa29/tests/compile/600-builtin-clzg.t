static_assert(__builtin_clzg(1u, 32) == 31, "");
static_assert(__builtin_clzg(0u, 32) == 32, "");

int f(unsigned x) {
  return __builtin_clzg(x, 32);
}

int g(unsigned long long x) {
  return __builtin_clzg(x, 64);
}

int main() {
  return f(1u) + g(0ull);
}
