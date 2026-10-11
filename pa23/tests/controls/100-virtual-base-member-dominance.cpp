// N3485 10.2p10: B::m hides V::m in every subobject through the shared virtual
// base, so the lookup of m in D is not ambiguous (full program: exit 8)
// expect exit 8
struct V { typedef char m; };
struct B : virtual V { typedef int m; };
struct C : virtual V { };
struct D : B, C { m n; };
struct L0 { };
struct M1a : virtual L0 { }; struct M1b : virtual L0 { }; struct L1 : M1a, M1b { };
struct M2a : virtual L1 { }; struct M2b : virtual L1 { }; struct L2 : M2a, M2b { };
struct P : virtual L2 { typedef char k; };
struct Q : virtual P { typedef int k; };
struct R : Q, virtual P { k n; };
int main() { return sizeof(D().n) + sizeof(R().n); } // 8
