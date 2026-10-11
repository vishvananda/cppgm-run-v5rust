// N3485 10.2p6: B::f hides A::f only in B's A subobject; C's A::f is still
// found, so the lookup of d.f is ambiguous (also operators/conversion
// functions/allocation functions declared in two different bases). Must
// reject.
struct A { int f() { return 1; } };
struct B : A { int f() { return 2; } };
struct C : A { };
struct D : B, C { };
int main() { D d; return d.f(); }
