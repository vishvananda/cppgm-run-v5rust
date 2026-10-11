// VALIDATION: compile-pass
template<class> struct P {};
struct B { template<class T> void f(P<T>) {} };
struct D : B { using B::f; template<class T> void f(T const &) { T::wrong(); } };
int main() { D d; d.f(P<int>()); }
