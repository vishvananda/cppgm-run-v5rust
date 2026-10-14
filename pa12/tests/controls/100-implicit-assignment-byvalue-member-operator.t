// N3485 12.8p28, 8.5p15, 5.2.2p4: X's implicit copy assignment calls
// M::operator=(M) with a parameter copy-constructed from b.m; the destructor
// runs on that parameter, not on b.m; expect exit 0
int copies;
struct M { int v; M() : v(5) {} M(const M& o) : v(o.v) { ++copies; } ~M() { v = 99; } M& operator=(M o) { v = o.v; return *this; } };
struct X { M m; };
int main() { X a, b; b.m.v = 7; a = b; return (a.m.v == 7 && b.m.v == 7 && copies == 1) ? 0 : 1; }
