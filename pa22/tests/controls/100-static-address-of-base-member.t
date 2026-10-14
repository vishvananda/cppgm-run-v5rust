// N3485 5.19p3 (address constant), 5.2.5: a namespace-scope int* = &m.y where
// y is a member of the second base B2 of M must include B2's offset (also &g.v
// for a virtual base member, dynamically initialized); expect exit 0
struct B1 { int x; B1() : x(1) {} };
struct B2 { int y; B2() : y(3) {} };
struct M : B1, B2 { };
M m; int* py = &m.y;
int main() { return *py == 3 ? 0 : 1; }
