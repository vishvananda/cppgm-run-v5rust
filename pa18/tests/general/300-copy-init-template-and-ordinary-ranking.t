// Independent namespaces check selection without sharing candidate inventories.
namespace case0 {
struct A { int value; A(int n):value(n){} template<class T> A(T &):value(1){} }; struct X { operator A() { return A(2); } }; int check(){X x; A a=x; return a.value == 2 ? 0 : 1;}
}
namespace case1 {
struct X; struct A { int value; A(int n):value(n){} A(X &); }; struct X { template<class T> operator T() { return T(2); } }; A::A(X &):value(1){} int check(){X x; A a=x; return a.value == 1 ? 0 : 1;}
}
int main() { return case0::check() || case1::check(); }
