// N3485 5.1.2p4, 6.6.3p2, 12.8p31: a lambda returning a by-reference captured
// class object copy-initializes the result; expect exit 0
struct D { int v; D(int x) : v(x) {} D(const D& o) : v(o.v) {} };
int main() { D d(3); auto l = [&] { return d; }; D e = l(); return e.v == 3 ? 0 : 1; }
