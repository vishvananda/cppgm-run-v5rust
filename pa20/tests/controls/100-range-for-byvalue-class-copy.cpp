// N3485 6.5.4p1: `for (V v : d)` copy-initializes v from *__begin, calling V's
// copy constructor each iteration; expect exit 0
int copies;
struct V { int n; V(int a) : n(a) {} V(const V& o) : n(o.n) { ++copies; } ~V() {} };
int main() { V d[2] = {V(1), V(2)}; int c0 = copies; for (V v : d) (void)v; return copies - c0 == 2 ? 0 : 1; }
