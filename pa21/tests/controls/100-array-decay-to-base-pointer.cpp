// N3485 4.2 + 4.10p3: an array of D decays to D* which converts to B* (B a
// base of D), also as a dynamic_cast operand (5.2.7p2); expect exit 0
struct B { virtual ~B() {} int v; };
struct D : B { };
int main() { D ds[2]; B* p = ds; int ok = p == ds; ok &= dynamic_cast<B*>(ds) == ds; return ok ? 0 : 1; }
