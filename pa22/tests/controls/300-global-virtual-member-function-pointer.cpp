// N3485 5.5p4, 4.11, Itanium 2.3: a namespace-scope pointer to a virtual
// member function (&B::g) dispatches virtually when called through a D object;
// expect exit 0
struct B { virtual int g(int x) { return x; } };
struct D : B { int g(int x) { return x * 3; } };
int (B::*gp)(int) = &B::g;
int main() { D d; B* bp = &d; return (bp->*gp)(2) == 6 ? 0 : 1; }
