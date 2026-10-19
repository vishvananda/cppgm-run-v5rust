// expect exit 0
struct A { int f() { return 1; } };
struct B : A {};
bool equal(int (A::*p)(), int (B::*q)()) { return p == q; }
bool unequal(int (A::*p)(), int (B::*q)()) { return q != p; }
int main() { return !equal(&A::f, &A::f) || unequal(&A::f, &A::f); }
