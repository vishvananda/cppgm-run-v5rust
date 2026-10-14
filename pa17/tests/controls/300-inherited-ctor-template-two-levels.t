// N3485 12.9p1-p3: R inherits Q's constructors, which include the constructor
// template Q inherited from P; R r1(3) uses it; expect exit 0
struct P { int v; template<class T> P(T a, int b = 4) : v(a * 10 + b) {} };
struct Q : P { using P::P; };
struct R : Q { using Q::Q; };
int main() { R r1(3); return r1.v == 34 ? 0 : 1; }
