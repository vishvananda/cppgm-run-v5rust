// N3485 3.6.2p2 (bullet 1), 12.2p5: a static reference bound to a temporary
// whose initializer is a constant expression is constant-initialized, so a
// dynamic initializer earlier in the unit already sees it; expect exit 0
struct K2 { int v; constexpr K2(int x) : v(x) {} };
extern const K2& r4;
int reads() { return r4.v == 9; }
int early = reads();
const K2& r4 = K2(9);
int main() { return early == 1 ? 0 : 1; }
