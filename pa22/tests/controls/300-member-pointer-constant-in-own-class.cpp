// N3485 4.11p2, 9.4.2p3: a static constexpr int D::* initialized in D's
// definition from &T::m designates the member of D's T subobject (offset after
// C), not offset 0; expect exit 0
struct B { int m; };
struct C { int c; };
template<class T> struct D : C, T { static constexpr int D::* p = &T::m; };
template<class T> constexpr int D<T>::* D<T>::p;
int main() { D<B> d; d.c = 3; d.m = 7; return d.*D<B>::p == 7 ? 0 : 1; }
