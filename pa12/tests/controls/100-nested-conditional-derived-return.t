// N3485 5.16p3-6, 6.6.3: return a ? B(1) : b ? D(2) : D(3); where the inner
// conditional is a D prvalue converted to B; expect exit 0
struct B { int v; B(int x) : v(x) {} B(const B& o) : v(o.v) {} ~B() {} };
struct D : B { D(int x) : B(x) {} };
B nested(int a, int b) { return a ? B(1) : b ? D(2) : D(3); }
int main() { return nested(0, 1).v == 2 ? 0 : 1; }
