// N3485 12.9p6, 7.1.5p9: an inheriting constructor whose inherited constructor
// is not constexpr is no constexpr constructor; its class is no literal type.
struct B { int v; B(int x) : v(x) {} };
struct D : B { using B::B; int m = 1; };
constexpr D d(2);
int main() { return 0; }
