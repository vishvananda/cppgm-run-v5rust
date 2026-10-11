// N3485 7.2p10, 5.1.1p8: an unscoped enumerator may be named qualified by its
// enumeration, E::A (full program: exit 23)
enum E { A = 3 };
struct S { enum { K = 2 }; };
int b[E::A];
int c[S::K];
int main() { return sizeof(b) + sizeof(c) + E::A; } // 23
