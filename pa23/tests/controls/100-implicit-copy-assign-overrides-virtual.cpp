// N3485 12.8p18/p24 + 10.3p2: D's implicitly-declared copy assignment D&
// operator=(const D&) overrides B's virtual operator=(const D&) (covariant D&
// for B&), so r = y through B& runs D's memberwise copy (b becomes 5) and
// returns x as B&; expect exit 0
struct D;
struct A { long a; A() : a(3) {} virtual long g() { return 3; } };
struct B { long b; B() : b(1) {} virtual B& operator=(const D&); virtual long f() { return 1; } };
struct D : A, B { long d; D() : d(2) {} };
B& B::operator=(const D&) { b = 99; return *this; }
int main() { D x, y; y.b = 5; B& r = x; B& q = (r = y); return x.b == 5 && &q == &r ? 0 : 1; }
