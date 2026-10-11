// N3485 6.5.1p2, 6.5.3p1: a class variable declared in a while/for condition
// is created and destroyed on each iteration; expect exit 0
int live, maxlive;
struct B { int v; B(int x) : v(x) { ++live; } ~B() { --live; } explicit operator bool() const { return v < 3; } };
int main() { int k = 1; while (B b = B(k)) { if (live > maxlive) maxlive = live; ++k; } return (maxlive == 1 && live == 0) ? 0 : 1; }
