// helper TU
struct S { int a; int b; };
typedef int S::* DM;
int read(S* o, DM d) { return o->*d; }
bool is_null(DM d) { return d == 0; }
DM get_b() { return &S::b; }
