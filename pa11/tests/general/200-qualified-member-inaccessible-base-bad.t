// N3485 11.2p5-p6, 5.2.5p5: s.B::x needs s converted to the naming class B, a
// protected (inaccessible here) base of S: ill-formed (s.x is already
// rejected). Must reject.
struct B { int x; };
struct S : protected B { };
int main() { S s; s.B::x = 1; return s.B::x; }
