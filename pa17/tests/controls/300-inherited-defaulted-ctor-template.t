// N3485 12.9p3: a constructor template whose parameters all have defaults is
// inherited as a constructor template (only non-template nullary constructors
// are excluded), so D d; uses it; expect exit 0
struct B { int b; template<class T = int> B(T x = T(8)) : b(x) {} };
struct D : B { using B::B; D(double) : B(9) {} };
int main() { D d; return d.b == 8 ? 0 : 1; }
