// Itanium ABI 5.2 (thunks are emitted with the function they adjust to, as
// global symbols): C::g overriding B::g (B at non-zero offset) is defined
// here, so this TU defines _ZThn8_N1C1gEv used by u2's vtable (key function
// C::key in u2); expect link ok and exit 0 under every mix of units built by
// cppgm++ and by the host compiler
struct A { virtual int f(); long a; };
struct B { virtual int g(); long b; };
struct C : A, B { int c; C(); virtual int key(); int g(); };
int C::g() { return 30 + c; }
int A::f() { return 1; }
int B::g() { return 2; }
int callG(B* p) { return p->g(); }
