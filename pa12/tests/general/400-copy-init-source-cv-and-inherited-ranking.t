// Independent namespaces check selection without sharing candidate inventories.
namespace case0 {
struct X; struct A { int value; A(int n): value(n) {}  A(X const &); }; struct X {  operator A()  { return A(2); } }; A::A(X const &): value(1) {}  int check() { X x; A a = x; return a.value == 2 ? 0 : 1; }
}
namespace case1 {
struct X; struct A { int value; A(int n): value(n) {}  A(X &); }; struct X {  operator A() const { return A(2); } }; A::A(X &): value(1) {}  int check() { X x; A a = x; return a.value == 1 ? 0 : 1; }
}
namespace case2 {
struct X; struct A { int value; A(int n): value(n) {}  A(X const &); }; struct X {  operator A()  { return A(2); } }; A::A(X const &): value(1) {}  int check() {  A a = X(); return a.value == 2 ? 0 : 1; }
}
namespace case3 {
struct X; struct A { int value; A(int n): value(n) {}  A(X const &); }; struct X {  operator A() & { return A(2); } }; A::A(X const &): value(1) {}  int check() { X x; A a = x; return a.value == 2 ? 0 : 1; }
}
namespace case4 {
struct Base;struct Mid;struct X;struct A{int value;A(int n):value(n){} A(Mid &);};struct Base {operator A()  {return A(2);}};struct Mid:Base {};struct X:Mid{};A::A(Mid &):value(1){}int check(){X x;A a=x;return a.value == 2 ? 0 : 1;}
}
namespace case5 {
struct Base;struct Mid;struct X;struct A{int value;A(int n):value(n){} A(Base &);};struct Base {};struct Mid:Base {operator A()  {return A(2);}};struct X:Mid{};A::A(Base &):value(1){}int check(){X x;A a=x;return a.value == 2 ? 0 : 1;}
}
namespace case6 {
struct Base;struct Mid;struct X;struct A{int value;A(int n):value(n){} A(Base &);};struct Base {operator A()  {return A(2);}};struct Mid:Base {};struct X:Mid{};A::A(Base &):value(1){}int check(){X x;A a=x;return a.value == 2 ? 0 : 1;}
}
namespace case7 {
struct Base;struct Mid;struct X;struct A{int value;A(int n):value(n){} A(Base &);};struct Base {operator A() const {return A(2);}};struct Mid:Base {};struct X:Mid{};A::A(Base &):value(1){}int check(){X x;A a=x;return a.value == 2 ? 0 : 1;}
}
namespace case8 {
struct Base;struct Mid;struct X;struct A{int value;A(int n):value(n){} A(Base const &);};struct Base {operator A()  {return A(2);}};struct Mid:Base {};struct X:Mid{};A::A(Base const &):value(1){}int check(){X x;A a=x;return a.value == 2 ? 0 : 1;}
}
namespace case9 {
struct A { int value; A(int n):value(n){} }; struct D:A { D():A(3){} }; struct X { operator A()const{return A(2);} operator D(){return D();} }; int check(){X x;A a=x;return a.value == 3 ? 0 : 1;}
}
namespace case10 {
struct A{int value;A(int n):value(n){}};A result(2);struct X{operator A &(){return result;}};int check(){X x;A a=x;return a.value == 2 ? 0 : 1;}
}
int main() { return case0::check() || case1::check() || case2::check() || case3::check() || case4::check() || case5::check() || case6::check() || case7::check() || case8::check() || case9::check() || case10::check(); }
