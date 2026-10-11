// N3485 10.3p7: E::f returning E* overrides A::f (A*), B::f (B*) and C::f
// (C*); E -> A is unambiguous (A is a single virtual base), so the covariant
// return is valid; expect exit 0
struct A { virtual A* f() { return this; } virtual ~A() {} };
struct B : virtual A { B* f() { return this; } };
struct C : virtual B { C* f() { return this; } int c; };
struct D : virtual A { int d; };
struct E : D, C { E* f() { return this; } int e; };
int main() { E e; A* a = &e; return a->f() == static_cast<A*>(&e) ? 0 : 1; }
