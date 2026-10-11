// SysV x86-64 ABI 3.2.3 + Itanium 2.3: a pointer to member function by value
// is a pair of INTEGER eightbytes; after six GPR args it goes on the stack
// (8-aligned, two slots); cppgm++ and host objects must agree; expect exit 0
// under every mix of units built by cppgm++ and by the host compiler
struct K { int v; int get() const { return v; } int twice() const { return 2 * v; } };
typedef int (K::*MF)() const;
long after_seven(long a, long b, long c, long d, long e, long f, long g, MF m, long h, const K& k);
int main() { K k; k.v = 3; return after_seven(1, 2, 3, 4, 5, 6, 7, &K::twice, 8, k) == 8691 ? 0 : 1; }
