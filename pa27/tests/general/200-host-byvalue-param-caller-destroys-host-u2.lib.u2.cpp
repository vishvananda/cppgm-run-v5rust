// callee TU
extern int dtors;
struct P { int v; P(int x) : v(x) {} P(const P& o) : v(o.v + 100) {} ~P() { ++dtors; } };
int take(P p) { return p.v + 10; }
