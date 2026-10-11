// N3485 5.4p4, 5.2.10p10: a C-style cast between pointers to data members of
// unrelated classes is a reinterpret_cast whose round trip yields the original
// value; expect exit 0
struct A { int a; };
struct B { long b; };
int main() { A x; x.a = 7; long B::*p = (long B::*)&A::a; int A::*q = (int A::*)p; return x.*q == 7 ? 0 : 1; }
