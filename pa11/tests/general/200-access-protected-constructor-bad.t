// N3485 11.4p1, 12p4: B's protected constructor is usable in D only for D
// objects; creating a complete B in a member of D is ill-formed;
// must be rejected
struct B { protected: B() {} };
struct D : B { void f() { B b; } };
int main() { D d; return 0; }
