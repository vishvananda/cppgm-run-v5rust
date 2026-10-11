// N3485 5.2.3p1 -> 5.4p4, 4.10p3: AP(&d) converts D* to A* where A is an
// ambiguous base of D: ill-formed. Must reject.
struct A { int v; };
struct I1 : A { };
struct I2 : A { };
struct D : I1, I2 { };
typedef A* AP;
int main() { D d; return AP(&d)->v; }
