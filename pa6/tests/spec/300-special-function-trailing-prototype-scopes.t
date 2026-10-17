// Trailing return prototypes retain their parameters and terminal name in the type dump.
struct A {
  auto operator+(int) -> int;
};
auto operator-(A, int) -> int;
auto A::operator+(int n) -> int { return n; }
auto operator "" _audit(unsigned long long) -> unsigned;
