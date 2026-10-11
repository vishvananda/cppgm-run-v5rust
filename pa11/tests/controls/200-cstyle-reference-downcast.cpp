// N3485 5.4p4, 5.2.9p2: a C-style cast (D&)b of a base lvalue to a derived
// reference is a static_cast downcast; expect exit 0
struct NP { long a; };
struct D : NP { long b; };
int main() { D d; d.b = 22; NP& nr = d; return ((D&)nr).b == 22 ? 0 : 1; }
