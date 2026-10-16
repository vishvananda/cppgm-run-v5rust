// expect exit 7
template<class T> struct A { typename T::missing bad; };
using Before = A<int>;
template<> struct A<int>;
template<> struct A<int> { struct B { int x; }; int f() { return 3; } };
using After = A<int>;
int main() { Before a; return a.f() + sizeof(After::B); }
