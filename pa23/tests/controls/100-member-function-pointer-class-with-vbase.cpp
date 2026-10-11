// N3485 5.5p3, 10.1p4: calling B::g through a pointer to member (no adjustment
// needed) where g reads a member of B's virtual base; expect exit 0
struct V { int v; };
struct B : virtual V { int b; int g() const { return b + v; } };
int main() { B o; o.b = 2; o.v = 3; int (B::*pg)() const = &B::g; return (o.*pg)() == 5 ? 0 : 1; }
