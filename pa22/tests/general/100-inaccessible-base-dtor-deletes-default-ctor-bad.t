// N3485 12.1p5 (last bullet), 8.4.3p2: a base whose destructor is inaccessible
// from D's defaulted default constructor makes it deleted, so `new D` is
// ill-formed. Must reject.
struct A { int a; A() : a(1) {} };
struct B { int b; B() : b(5) {} private: ~B() {} };
struct D : A, B {};
int main() { D* p = new D; (void)p; return 0; }
