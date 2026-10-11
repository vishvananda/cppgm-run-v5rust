// N3485 5.2.3p1, 12.8p7: K(k) for an lvalue k of a class whose implicit copy
// constructor is non-trivial (member M has a user copy ctor) is
// direct-initialization by that copy constructor; expect exit 0
struct M { int v; M() : v(1) {} M(const M& o) : v(o.v + 1) {} };
struct K { M m; };
int main() { K k; K c = K(k); return c.m.v >= 2 ? 0 : 1; }
