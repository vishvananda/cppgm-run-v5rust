// N3485 12.9p1, 7.3.3p1, 14.6.2: using Base::Base in a class template names
// the (template) constructors of the dependent base once instantiated;
// expect exit 0
struct B { int b; template<class T> B(T t) : b((int)t * 2) {} };
template<class Base> struct DB : Base { using Base::Base; };
int main() { DB<B> g(4); return g.b == 8 ? 0 : 1; }
