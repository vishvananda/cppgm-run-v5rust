// N3485 3.4.3.1p2 and its note: an elaborated-type-specifier, a nested-name-
// specifier, a base-specifier and a mem-initializer-id find the class; a
// member named through a derived class is the base's type. expect exit 0
struct K { int x; K() : x(1) {} K(int v); struct In { int y; }; };
K::K(int v) : x(v) {}
struct D : K::K { D() : K::K() {} };
struct E : K {};
struct K::K kk;
K::K::In in;
E::K* pk = &kk;
template<class T> struct S { S(); int v; typedef int I; S::I f(); };
template<class T> S<T>::S() : v(4) {}
template<class T> typename S<T>::I S<T>::f() { return v; }
int main() { D d; S<int> s; return kk.x + in.y + d.x + pk->x + s.f() - 7; }
