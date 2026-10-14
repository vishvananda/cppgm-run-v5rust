// N3485 6.6.3p2, 12.2, SysV ABI 3.2.3: return R(p, len); of a memory-class R
// built from two locals must pass the right constructor arguments;
// expect exit 0
struct R { long b[3]; int n; R(int s, int k) : n(k) { } };
R g(int len) { int p = 3; return R(p, len); }
int main() { return g(7).n == 7 ? 0 : 1; }
