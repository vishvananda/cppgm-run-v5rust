// N3485 15.2p1: a call after a new-expression (whose constructor may throw) in
// the same full-expression is still covered by the cleanup of the live local;
// expect exit 0
static int live, mode;
static int g(int x) { if (mode) throw x; return x; }
struct T { T() { ++live; } ~T() { --live; } };
struct N { N() { g(0); } };
static int keep(N* n) { delete n; mode = 1; return 1; }
static void s1() { T v; g(keep(new N)); }
int main() { try { s1(); } catch (int) {} return live; }
