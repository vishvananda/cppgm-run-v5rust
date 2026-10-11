// N3485 5.19p2, Annex B: a constexpr evaluation exceeding the implementation's
// recursion limit is not a constant expression, so the static_assert is
// ill-formed; must be rejected
constexpr long deep(long n) { return n == 0 ? 0 : 1 + deep(n - 1); }
static_assert(deep(100000) == 100000, "");
int main() { return 0; }
