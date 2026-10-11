namespace N { int g(int x) { return x; } }
namespace M = N;
int f() { return M::g(0); }
