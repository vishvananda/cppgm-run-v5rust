// N3485 11.4p1, 11.2p5: inside D, &D::f forms a pointer to member to the
// protected B::f named through the derived class D (allowed); expect exit 0
struct B { protected: void f() {} };
struct D : B { static int g() { void (B::*p)() = &D::f; return p != 0 ? 0 : 1; } };
int main() { return D::g(); }
