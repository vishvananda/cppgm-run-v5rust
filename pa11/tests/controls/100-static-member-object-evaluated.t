// N3485 5.2.5p1, 9.4p2: the object expression of E.m is evaluated even when m
// is a static member, static member function or enumerator; expect exit 0
// (also: the expression statement `get().E;` is rejected with "member
// expression is not a data-member lvalue")
int calls;
struct S { enum { E = 3 }; static const int K = 4; static int f() { return 5; } };
S s;
S& get() { ++calls; return s; }
int main() { int v = get().E + get().K + get().f(); return (v == 12 && calls == 3) ? 0 : 1; }
