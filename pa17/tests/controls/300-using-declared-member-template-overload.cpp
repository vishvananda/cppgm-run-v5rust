// N3485 7.3.3p15, 14.5.6.2: a using-declared base template f(T*) with a
// different signature overloads the derived f(T); partial ordering picks f(T*)
// for &x; expect exit 0
struct B { template<class T> int f(T*) { return 1; } };
struct D : B { using B::f; template<class T> int f(T) { return 2; } };
int main() { D d; int x; return d.f(&x) * 10 + d.f(x) == 12 ? 0 : 1; }
