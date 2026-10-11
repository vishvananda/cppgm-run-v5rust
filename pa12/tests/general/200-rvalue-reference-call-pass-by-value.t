struct S {
  int x;
  S(int v) : x(v) {}
  S(const S&) = default;
};

int take(S s) { return s.x; }

S&& pass(S&& s) { return static_cast<S&&>(s); }

int main() { return take(pass(S(7))) - 7; }
