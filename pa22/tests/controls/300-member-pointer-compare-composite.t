// N3485 5.10p2: comparing int D::* with int S::* (S a base of D) converts to
// the composite type int D::*; expect exit 0
struct S { int a, b; };
struct X { long x; };
struct D : X, S { int d; };
typedef int S::*PM; typedef int D::*DPM;
int main() { PM p = &S::b; DPM d = p; return d == &S::b && !(d != p) ? 0 : 1; }
