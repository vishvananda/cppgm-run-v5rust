// Independent namespaces check selection without sharing candidate inventories.
namespace case0 {
struct X; struct A { int value; A(int n): value(n) {}  A(X &); }; struct X {  operator A()  { return A(2); } }; A::A(X &): value(1) {}  int check() { X x; A a(x); return a.value == 1 ? 0 : 1; }
}
namespace case1 {
struct X; struct A { int value; A(int n): value(n) {}  A(X &); }; struct X {  operator A()  { return A(2); } }; A::A(X &): value(1) {}  int check() { X x; A a{x}; return a.value == 1 ? 0 : 1; }
}
namespace case2 {
struct X; struct A { int value; A(int n): value(n) {}  A(X &); }; struct X {  operator A()  { return A(2); } }; A::A(X &): value(1) {}  int check() { X x; A a = {x}; return a.value == 1 ? 0 : 1; }
}
namespace case3 {
struct X; struct A { int value; A(int n): value(n) {} explicit A(X &); }; struct X {  operator A()  { return A(2); } }; A::A(X &): value(1) {}  int check() { X x; A a = x; return a.value == 2 ? 0 : 1; }
}
namespace case4 {
struct X; struct A { int value; A(int n): value(n) {}  A(X &); }; struct X { explicit operator A()  { return A(2); } }; A::A(X &): value(1) {}  int check() { X x; A a = x; return a.value == 1 ? 0 : 1; }
}
namespace case5 {
struct X; struct A { int value; A(int n): value(n) {} explicit A(X &); }; struct X { explicit operator A()  { return A(2); } }; A::A(X &): value(1) {}  int check() { X x; A a(x); return a.value == 1 ? 0 : 1; }
}
int main() { return case0::check() || case1::check() || case2::check() || case3::check() || case4::check() || case5::check(); }
