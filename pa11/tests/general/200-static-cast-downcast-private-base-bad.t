// N3485 5.2.9p11: static_cast<D*>(b) requires B to be an accessible
// non-virtual base of D; B is a private base. Must reject.
struct B { int v; };
struct D : private B { int w; };
int main() { D d; B* b = (B*)&d; return static_cast<D*>(b) == &d; }
