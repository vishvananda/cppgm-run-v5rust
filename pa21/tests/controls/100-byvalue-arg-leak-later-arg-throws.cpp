// N3485 15.2p1/5.2.2p4: a by-value class argument already constructed (P(0))
// is destroyed when evaluating a later argument throws (the call never
// happens); expect exit 0
static int live;
struct P { int id; explicit P(int) : id(1) { ++live; } P(const P& o) : id(o.id) { ++live; } ~P() { --live; } };
static int thr(int t) { if (t) throw t; return 0; }
struct Two { int s; Two(P a, int b) : s(a.id + b) {} };
int main() { try { Two t(P(0), thr(1)); } catch (int) {} return live; }
