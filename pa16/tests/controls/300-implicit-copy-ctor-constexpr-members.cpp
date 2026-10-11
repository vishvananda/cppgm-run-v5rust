// N3485 12.8p13/p15, 7.1.5: the implicit copy constructor of M copies member L
// with L's constexpr copy constructor during constant evaluation;
// expect exit 0
struct L { int v; constexpr L(int x) : v(x) {} constexpr L(const L& o) : v(o.v + 1) {} };
struct M { L a; constexpr M() : a(1) {} };
constexpr M t;
constexpr M u = t;
static_assert(u.a.v == 2, "member copy constructor must run");
int main() { return 0; }
