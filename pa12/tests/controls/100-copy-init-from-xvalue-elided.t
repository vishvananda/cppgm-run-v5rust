// N3485 8.5p16/12.8p31: In g = static_cast<In&&>(f) copy-initializes from an
// xvalue: the move constructor runs (elision applies only to temporaries);
// expect exit 0
struct In { int v; In(int x) : v(x) {} In(const In& o) : v(o.v + 1) {} In(In&& o) : v(o.v + 100) { o.v = -1; } };
int main() { In f(2); In g = static_cast<In&&>(f); return g.v == 102 && f.v == -1 ? 0 : 1; }
