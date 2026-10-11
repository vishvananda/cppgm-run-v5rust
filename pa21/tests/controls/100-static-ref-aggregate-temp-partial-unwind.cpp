// N3485 15.2p2, 12.2p5, 8.5.1p2: when member b's initializer (a conversion
// function returning E) throws while building the aggregate temporary bound to
// a static local reference, member a is destroyed; expect exit 0
static int live = 0;
struct E { int v; E(int x) : v(x) { if (x == 521) throw x; ++live; } E(const E& o) : v(o.v) { ++live; } ~E() { --live; } };
struct C { int v; C(int x) : v(x) {} operator E() const { return E(v + 500); } };
struct S { E a; E b; const E& r; };
static void f2() { static const S& r = S{20, C(21), E(22)}; (void)r; }
int main() { { E keep(1); } try { f2(); } catch (int) {} return live == 0 ? 0 : 10 + live; }
