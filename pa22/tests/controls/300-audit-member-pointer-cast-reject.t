// expect reject
struct A { int x; };
struct B { int y; };
struct D : A, B {};
constexpr int D::*p = &B::y;
constexpr int A::*q = static_cast<int A::*>(p);
