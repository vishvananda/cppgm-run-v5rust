// N3485 5.10p2, 4.11: pointers to members of a base and derived class (int
// D::* vs int B::*; also const int C::* vs volatile int C::*) are converted to
// a common type and compared; expect exit 0
struct A { int a; };
struct B { int b; };
struct D : A, B { };
int main() { int D::*pd = &B::b; return pd == &B::b && pd != &A::a ? 0 : 1; }
