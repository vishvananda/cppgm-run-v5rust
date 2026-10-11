struct V {};
struct A : virtual V {};
struct I { virtual void f(A &) = 0; };
I & i();
A & a();
void g() { i().f(a()); }
