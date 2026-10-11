struct C {
  int n;
  constexpr C(int v) : n(v) {}
  friend constexpr C operator+(C const& a, C const& b) { return C(a.n + b.n); }
};
static_assert((C(1) + 2).n == 3, "");
int main() {}
