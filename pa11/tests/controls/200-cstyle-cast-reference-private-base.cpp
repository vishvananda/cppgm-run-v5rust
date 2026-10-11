// N3485 5.4p4: a C-style cast to a reference to a private base class is valid
// (access is ignored, performed as static_cast); expect exit 0
struct B { int v; B() : v(2) {} };
struct D : private B { int w; D() : w(0) {} };
int main() { D d; return ((B&)d).v == 2 ? 0 : 1; }
