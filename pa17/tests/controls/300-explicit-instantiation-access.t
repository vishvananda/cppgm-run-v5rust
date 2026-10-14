// N3485 14.7.2p12: access checking does not apply to names in an explicit
// instantiation (private A::I as template argument, private member template
// A::M); expect exit 0
class A { typedef int I; template<class T> struct M { static int f() { return 1; } }; };
template<class T> struct X { static int f() { return 2; } };
template struct X<A::I>;
template struct A::M<int>;
int main() { return X<int>::f() == 2 ? 0 : 1; }
