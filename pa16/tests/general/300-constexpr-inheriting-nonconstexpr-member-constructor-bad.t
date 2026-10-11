// N3485 12.9p6, 7.1.5p4: an inheriting constructor whose member's
// brace-or-equal-initializer selects a constructor that is not constexpr (of a
// literal class) is no constexpr constructor; the constexpr object it would
// initialize is ill-formed.
struct B { int v; constexpr B(int x) : v(x) {} };
struct P { int k; constexpr P() : k(0) {} P(int x) : k(x) {} };
struct D : B { using B::B; P p{3}; };
D runtime(1);
constexpr D d(2);
int main() { return 0; }
