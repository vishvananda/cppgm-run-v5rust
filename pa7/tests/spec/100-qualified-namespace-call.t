// N3485 focus: 3.4.3 [basic.lookup.qual], 5.2.2 [expr.call]
namespace N { int g(int x) { return x; } }
int f() { return N::g(0); }
