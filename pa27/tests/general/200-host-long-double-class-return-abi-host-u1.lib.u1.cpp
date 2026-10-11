// SysV x86-64 ABI 3.2.3: struct { long double x; } is classified X87/X87UP:
// passed in memory, returned in st0; cppgm++ and host objects must agree;
// expect exit 0 under every mix of units built by cppgm++ and by the host
// compiler
struct LDO { long double x; };
double take_ldo(LDO a, int i, LDO b, double d);
LDO make_ldo(double v);
int main() { LDO a = make_ldo(1.0), b = make_ldo(0.25); return take_ldo(a, 1, b, 0.5) == 8.25 ? 0 : 1; }
