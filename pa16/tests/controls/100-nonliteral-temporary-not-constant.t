// N3485 5.19p2, 3.6.2p2: T(5).v with ~T() non-trivial invokes a non-constexpr
// destructor, so the initializer is dynamic and the destructor runs;
// expect exit 0
int count;
struct T { int v; constexpr T(int x) : v(x) {} ~T() { ++count; } };
int u = T(5).v;
int main() { return (u == 5 && count == 1) ? 0 : 1; }
