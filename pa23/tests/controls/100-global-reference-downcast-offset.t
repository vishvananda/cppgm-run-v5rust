// N3485 5.2.9p2, 3.6.2p2: a namespace-scope reference initialized by
// static_cast<D&> of a base subobject at offset 8 (D adds the vptr) refers to
// the complete D object; expect exit 0
struct NP { long a; NP() : a(11) {} };
struct D : NP { virtual int f() { return 2; } long b; D() : b(22) {} };
D gd;
D& gdown = static_cast<D&>(static_cast<NP&>(gd));
int main() { return (&gdown == &gd && gdown.b == 22) ? 0 : 1; }
