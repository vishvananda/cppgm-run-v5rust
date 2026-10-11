// N3485 10.3p2, 12.3.2, 13.3.1.5: a virtual conversion function used for an
// implicit conversion (int x = a; if (a)) is called virtually; expect exit 0
struct A { virtual operator int() const { return 1; } virtual ~A() {} };
struct B : A { operator int() const { return 7; } };
int main() { B b; const A& a = b; int x = a; return x == 7 ? 0 : 1; }
