// N3485 5.18p1 (also 5.16p4): a comma expression whose result is a bit-field
// lvalue designates that bit-field, not the unit's first field; expect exit 0
// (the conditional form `(in ? s.a : s.c) = 2` with in==0 also writes s.a; g++
// agrees it must write s.c, clang cannot codegen it)
struct S { int c : 5; unsigned b : 3; };
S s;
int main() { (0, s.b) = 7; return (s.c == 0 && s.b == 7) ? 0 : 1; }
