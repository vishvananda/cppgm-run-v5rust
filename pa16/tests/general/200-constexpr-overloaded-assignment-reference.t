// VALIDATION: compile-pass

struct value {
  int n;
  constexpr value() : n(7) {}
};

struct factory {
  constexpr const value& operator=(const value& v) const { return v; }
};

static_assert((factory() = value()).n == 7, "");

int main() { return 0; }
