// N3485 12.6.2p10, 12.7p3: a base mem-initializer may read a member of the
// class's virtual base, which is constructed first (before the constructor
// stores the vtable pointers); expect exit 0
struct V { int v; V() : v(7) {} };
struct X { int x; X(int a) : x(a) {} };
struct B : X, virtual V { B() : X(v) {} };
int main() { B b; return b.x == 7 ? 0 : 1; }
