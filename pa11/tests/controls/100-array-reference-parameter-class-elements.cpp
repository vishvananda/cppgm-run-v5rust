// N3485 8.5.4p3 (reference bullet), 8.5.1p2, 13.3.3.1.5: ssum({7, 8})
// list-initializes a temporary S[2] whose elements are copy-initialized from 7
// and 8 through S(int); expect exit 0
struct S { int v; S(int x) : v(x) {} };
int ssum(const S (&a)[2]) { return a[0].v + a[1].v; }
int main() { return ssum({7, 8}) == 15 ? 0 : 1; }
