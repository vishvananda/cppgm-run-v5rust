// Itanium ABI 5.2.3: the key function is the first non-pure virtual function
// declared non-inline in the class, overriders included: for C { int g();
// virtual int key(); } with g overriding B::g it is C::g, so this TU (defining
// C::g) owns C's vtable; expect link ok and exit 0 under every mix of units
// built by cppgm++ and by the host compiler
struct B { int b; B() : b(2) {} virtual int g(); };
struct C : B { int c; C(); int g(); virtual int key(); };
int C::g() { return 30 + c; }
int B::g() { return 2; }
int callG(B* p) { return p->g(); }
