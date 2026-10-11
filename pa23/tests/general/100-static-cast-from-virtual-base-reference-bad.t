// N3485 5.2.9p2: static_cast from a reference to a virtual base B to D& is
// ill-formed; must be rejected
struct B {}; struct D : virtual B {};
int main() { D d; B& b = d; D& r = static_cast<D&>(b); return &r != &d; }
