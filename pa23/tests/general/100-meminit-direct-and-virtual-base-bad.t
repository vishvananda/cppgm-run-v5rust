// N3485 12.6.2p2: a mem-initializer-id designating both a direct non-virtual
// base and an inherited virtual base is ill-formed. Must reject.
struct A { A(int) {} A() {} };
struct B : virtual A { };
struct C : B, A { C() : A(1) {} };
int main() { C c; (void)c; return 0; }
