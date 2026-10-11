namespace N { int g(int x) { return x; } }
using N::g;
int f() { return g(0); }
