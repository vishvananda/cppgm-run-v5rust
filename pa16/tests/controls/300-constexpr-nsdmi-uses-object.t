// N3485 7.1.5p4, 5.19p2, 12.6.2p8: in a constexpr constructor call, a
// brace-or-equal-initializer may read an earlier member of the object under
// construction (b = a + 1, n = get()); expect exit 0
struct K { int a; int b = a + 1; constexpr K(int x) : a(x) {} };
constexpr K k1(7);
static_assert(k1.b == 8, "");
int main() { return 0; }
