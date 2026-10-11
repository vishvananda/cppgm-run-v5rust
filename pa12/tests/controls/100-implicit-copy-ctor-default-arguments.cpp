// N3485 12.8p15, 8.3.6p1: X's implicit copy constructor calls M(const M&, int
// = f()), supplying the default argument (evaluated once); expect exit 0
int n;
int f() { return ++n; }
struct M { int v; M() : v(0) {} M(const M& o, int k = f()) : v(o.v + k) {} };
struct X { M m; };
int main() { X x; X y(x); return (y.m.v == 1 && n == 1) ? 0 : 1; }
