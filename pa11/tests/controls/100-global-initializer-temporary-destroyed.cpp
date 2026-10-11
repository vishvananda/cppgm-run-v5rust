// N3485 1.9p10, 12.2p3: a namespace-scope variable's dynamic initializer is a
// full-expression; a temporary created in it (not bound to a reference) is
// destroyed at its end; expect exit 0
int live;
struct A { int val; A(int v) : val(v) { ++live; } ~A() { --live; } };
int g(const A& a) { return a.val; }
int gk = g(A(6));
int snapshot_live = live;
int main() { return (gk == 6 && snapshot_live == 0) ? 0 : 1; }
