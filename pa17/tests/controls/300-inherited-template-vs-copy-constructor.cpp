// N3485 12.9p3, 12.8p7: D's implicit copy constructor is not suppressed by an
// inherited constructor template; D d2(d1) copies; expect exit 0
struct B { int v; B() : v(3) {} template<class T> B(T) : v(99) {} };
struct D : B { using B::B; D() {} };
int main() { D d1; D d2(d1); return d2.v == 3 ? 0 : 1; }
