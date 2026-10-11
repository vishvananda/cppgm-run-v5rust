// N3485 5.16p3 (class bullet): in c ? D() : B(), the D prvalue is converted to
// a B prvalue by copy-initializing a B temporary, so B's copy constructor
// runs; expect exit 0
struct B { int v; B() : v(1) {} B(const B& o) : v(o.v + 10) {} };
struct D : B { D() { v = 2; } };
volatile bool c = true;
int main() { return (c ? D() : B()).v == 12 ? 0 : 1; }
