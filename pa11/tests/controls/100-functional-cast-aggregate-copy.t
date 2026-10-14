// N3485 5.2.3p1, 5.4, 12.8: A(a0) with one expression is the cast (A)a0, i.e.
// copy construction, also for an aggregate A; expect exit 0
struct A { int x; };
int main() { A a0 = { 1 }; A b = A(a0); return b.x == 1 ? 0 : 1; }
