// N3485 12.8/31, 6.6.3: return L(l, 10) returns a new object built by L(const
// L&, int); the local l is destroyed separately and must not share the return
// slot; expect exit 0
struct L { int v; L(int x) : v(x) {} L(const L& o) : v(o.v) {} L(const L& o, int add) : v(o.v + add) {} ~L() { v = -1; } };
L mk() { L l(1); return L(l, 10); }
int main() { L x = mk(); return x.v == 11 ? 0 : 1; }
