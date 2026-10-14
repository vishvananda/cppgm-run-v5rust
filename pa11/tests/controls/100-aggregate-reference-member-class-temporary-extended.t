// N3485 12.2p5, 8.5.1p2: a class temporary bound to a reference member in
// aggregate initialization of a local persists for the aggregate's lifetime;
// expect exit 0
int live;
struct A { int val; A(int v) : val(v) { ++live; } ~A() { --live; } };
struct R { const A& r; int x; };
int main() { int res = 0; { R r = { A(3), 1 }; if (live != 1 || r.r.val != 3) res = 1; } return (res == 0 && live == 0) ? 0 : 1; }
