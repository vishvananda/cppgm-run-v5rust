// N3485 10.3p2: V::f in D's shared virtual V has two final overriders (C::f
// and the direct B's B::f, not dominated by C): ill-formed. Must reject.
struct V { long v; V() : v(1) {} virtual long f() { return 1; } };
struct B : virtual V { long b; B() : b(2) {} long f() { return 2; } };
struct C : B { long c; C() : c(3) {} long f() { return 3; } };
struct D : C, B { long d; D() : d(4) {} };
int main() { D x; return (int)x.d; }
