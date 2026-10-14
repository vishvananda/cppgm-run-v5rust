// N3485 13.3.3.2p4 (last bullet group): converting int A::* to int B::* is
// better than to int C::* when C derives from B derives from A; expect exit 0
struct A { int a; };
struct B : A { };
struct C : B { };
int f(int B::*) { return 1; }
int f(int C::*) { return 2; }
int main() { return f(&A::a) == 1 ? 0 : 1; }
