// Itanium C++ ABI 3.1.1 (and N3485 5.2.2p4 caller-side lifetime): a by-value
// parameter of class type with a non-trivial destructor is destroyed by the
// caller, not the callee; cppgm++/host mixes must destroy it exactly once;
// expect exit 0 under every mix of units built by cppgm++ and by the host
// compiler
int dtors;
struct P { int v; P(int x) : v(x) {} P(const P& o) : v(o.v + 100) {} ~P() { ++dtors; } };
int take(P p);
int main() { int r; { P a(1); r = take(a); } return r == 111 && dtors == 2 ? 0 : 10 + dtors; }
