struct S {
  static constexpr int one = 1;
  static constexpr int two = one + one;
};
static_assert(S::two == 2, "bad");
int main() { return 0; }
