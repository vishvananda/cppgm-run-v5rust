// N3485 8.5p16, 13.3.1.3, 8.5.3p5: copy-initializing a B from a D prvalue
// returned by a call runs B(const B&) bound to the base subobject of the D
// temporary; expect exit 0
struct B { long b; B(long x) : b(x) {} B(const B& o) : b(o.b + 10) {} };
struct D : B { D(long x) : B(x) {} };
D makeD(long x) { return D(x); }
long base_value(B b) { return b.b; }
int main() { return base_value(makeD(4)) == 14 ? 0 : 1; }
