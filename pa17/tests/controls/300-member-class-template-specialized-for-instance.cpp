// N3485 14.7.3p16 (example: template<> template<class X1> class A<int>::B
// {...}): a member class template may be explicitly specialized for an
// implicit instantiation of its enclosing class template; expect exit 0
template<class T> struct A { template<class U> struct B { int f() { return 0; } }; };
template<> template<class U> struct A<int>::B { int f() { return 1; } };
int main() { return (A<int>::B<char>().f() == 1 && A<long>::B<char>().f() == 0) ? 0 : 1; }
