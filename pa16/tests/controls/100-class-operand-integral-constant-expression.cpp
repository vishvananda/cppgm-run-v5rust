// N3485 5.19p5 (contextual conversion of a literal class via its constexpr
// conversion function), 7.2p5, 6.4.2p2: Two() is an integral constant
// expression in an enumerator and a case label; expect exit 0
struct Two { constexpr operator int() const { return 2; } };
enum E { e = Two() };
int pick(long v) { switch (v) { case Two(): return 20; } return 0; }
int main() { return (e == 2 && pick(2) == 20) ? 0 : 1; }
