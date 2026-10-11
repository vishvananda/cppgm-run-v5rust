// Itanium ABI 3.1.1/5.1.4.6: a this-adjusting thunk for C::virt(P) forwards a
// by-value argument of a class non-trivial for calls (passed by invisible
// reference) unchanged; expect exit 0
struct P { int id; P(int i) : id(i) {} P(const P& o) : id(o.id) {} ~P() {} };
struct A { virtual ~A() {} long pad; };
struct B { virtual int virt(P p) { return p.id; } virtual ~B() {} };
struct C : A, B { int virt(P p) { return p.id * 2; } };
int main() { C c; B& b = c; return b.virt(P(9)) == 18 ? 0 : 1; }
