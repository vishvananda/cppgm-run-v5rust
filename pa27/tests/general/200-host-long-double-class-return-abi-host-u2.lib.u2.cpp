// callee TU
struct LDO { long double x; };
double take_ldo(LDO a, int i, LDO b, double d) { return (double)(a.x * 2 + b.x) + i + d; }
LDO make_ldo(double v) { LDO r; r.x = v * 3; return r; }
