template<class T> struct A { A() {} };
void f() { enum T { x }; A<T> a; }
void g() { enum T { x }; A<T> a; }
int main() { f(); g(); }
