struct S {
  constexpr S() {}
};

constexpr int S(int a, int b, int c) { return a + b + c; }

constexpr int k = S(1, 2, 3);
static_assert(k == 6, "");

int main() { return k - 6; }
