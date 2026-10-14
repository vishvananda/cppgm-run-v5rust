// N3485 5.16/5.18/15.1: in f ? take(P(0)) : (take(P(0)), throw 3, 0) the
// by-value argument temporary must be cleaned up on the throwing branch; must
// compile and catch 3; expect exit 0
struct P { int id; P(int i) : id(i) {} P(const P& o) : id(o.id) {} ~P() {} };
static int take(P p) { return p.id; }
int main(int argc, char**) { bool f = argc < 0; try { int x = f ? take(P(0)) : (take(P(0)), throw 3, 0); return x; } catch (int e) { return e - 3; } }
