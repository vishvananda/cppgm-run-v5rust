// N3485 8.5.1p2, 12.8p31: an aggregate with two members of a class with a
// non-trivial destructor, initialized from prvalues; expect exit 0
struct M { int v; M(int x) : v(x) {} ~M() {} };
struct Agg { M m; M w; };
int main() { Agg a = { M(1), M(3) }; return a.m.v + a.w.v == 4 ? 0 : 1; }
