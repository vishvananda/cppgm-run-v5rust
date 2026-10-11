// key-function TU emitting C's vtable
struct A { virtual int f(); long a; };
struct B { virtual int g(); long b; };
struct C : A, B { int c; C(); virtual int key(); int g(); };
int callG(B* p);
C::C() : c(3) {}
int C::key() { return 0; }
int main() { C x; return callG(&x) == 33 ? 0 : 1; }
