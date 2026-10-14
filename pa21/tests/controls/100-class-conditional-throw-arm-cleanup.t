// N3485 5.16p2, 15.2p1: in `x > 0 ? local : throw x` with class type, taking
// the throw arm unwinds the already-live local (destroyed exactly once);
// expect exit 0
int live;
struct V { int v; V(int x) : v(x) { ++live; } V(const V& o) : v(o.v) { ++live; } ~V() { --live; } };
V make(int x) { V local(x); return x > 0 ? local : throw x; }
int main() { int r = 0; try { make(-1); } catch (int v) { r = v; } return r == -1 && live == 0 ? 0 : 1; }
