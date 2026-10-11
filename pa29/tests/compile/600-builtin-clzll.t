constexpr int top_bit = __builtin_clzll(1ull);
static_assert(top_bit == 63, "");

int f(unsigned long long x) {
  return __builtin_clzll(x);
}

int main() {
  return f(8ull);
}
