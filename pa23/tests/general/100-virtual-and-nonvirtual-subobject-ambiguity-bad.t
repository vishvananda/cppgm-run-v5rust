// N3485 10.2p6, 10.1p4: W is a virtual base of A and a non-virtual base of B,
// so D has two W subobjects and d.m is ambiguous (also A::m does not dominate
// the virtual W::m reached through B). Must reject.
struct W { int m; W() : m(1) {} };
struct A : virtual W { A() {} };
struct B : W { B() {} };
struct D : A, B { D() {} };
int get(D& d) { return d.m; }
int main() { D d; return get(d); }
