// N3485 7.1.6.4p6-p7, 8.5p16: `auto b = a;` deduces C and copy-initializes b
// from the lvalue a by C's copy constructor; expect exit 0
int copies;
struct C { int v; C(int x) : v(x) {} C(const C& o) : v(o.v + 1) { ++copies; } };
int main() { C a(3); auto b = a; return copies == 1 && b.v == 4 ? 0 : 1; }
