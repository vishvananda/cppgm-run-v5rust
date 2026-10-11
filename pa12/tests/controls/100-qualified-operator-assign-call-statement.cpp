// N3485 6.8/5.2.2: B::operator=(B()); is an expression statement calling the
// base assignment with a prvalue argument (libstdc++ <list>
// _List_size::operator=(_List_size())); expect exit 0
struct B { int v; B() : v(7) {} B& operator=(const B& o) { v = o.v + 1; return *this; } };
struct D : B { void reset() { B::operator=(B()); } };
int main() { D d; d.v = 0; d.reset(); return d.v == 8 ? 0 : 1; }
