// N3485 12.2p5, 8.5.3p5: in a namespace-scope reference initializer only the
// temporary bound to the reference persists; the argument temporary A(9) and
// the object of A(12).val (converted to long) end with the full-expression;
// expect exit 0
int live;
struct A { int val; A(int v) : val(v) { ++live; } ~A() { --live; } };
int g(const A& a) { return a.val; }
const int& gi = g(A(9));
const long& gl = A(12).val;
int snapshot_live = live;
int main() { return (snapshot_live == 0 && gi == 9 && gl == 12) ? 0 : 1; }
