// N3485 15.1p4, 15.2p1: the exception object destructor throwing at the end of
// a handler propagates like any throw there: locals alive around the handler
// (L a) are destroyed; expect exit 0
int h;
struct E { int e; E() : e(1) {} E(const E& o) : e(o.e + 1) {} ~E() noexcept(false) { h = h * 10 + e; if (e == 1) throw 5; } };
struct L { int l; L(int v) : l(v) {} ~L() { h = h * 10 + l; } };
void f0() { L a(2); try { throw E(); } catch (const E&) { h = h * 10 + 3; } h = h * 10 + 4; }
int main() { try { f0(); } catch (int e) { h = h * 10 + e; } return h == 3125 ? 0 : 1; }
