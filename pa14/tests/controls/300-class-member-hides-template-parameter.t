// N3485 14.6.1p9 (example): in an out-of-class member definition of a class
// template, a class member (struct S) hides a template parameter named S after
// the declarator-id; expect exit 0 (also: typedef C member vs parameter C:
// "class template member definition has no declaration"; member class
// definition A<C>::Nested { C c; } uses the parameter)
template<class T> struct A { struct S { int x; }; int f(); };
template<class S> int A<S>::f() { S s; s.x = 7; return s.x; }
int main() { A<long> a; return a.f() == 7 ? 0 : 1; }
