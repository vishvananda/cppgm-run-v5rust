// N3485 7.1.5p4, 12.1p6, 8.4.2p2: a constexpr constructor must initialize
// every base and member; B's trivial default constructor leaves x
// uninitialized; must be rejected
struct B { int x; };
struct D : B { int y; constexpr D() : y(0) {} };
int main() { return 0; }
