// N3485 13.3.1.4p1 (CWG 899), 5.2.3p1: Out(Explicit()) direct-initializes Out;
// the copy/move constructor reference parameter may bind the result of an
// explicit conversion function; expect exit 0
struct Out { int x; Out() : x(0) {} };
struct Explicit { explicit operator Out() const { Out o; o.x = 11; return o; } };
int main() { Out d = Out(Explicit()); return d.x == 11 ? 0 : 1; }
