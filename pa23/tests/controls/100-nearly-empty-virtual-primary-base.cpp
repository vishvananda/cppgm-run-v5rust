// Itanium ABI 2.4 (II/III: a nearly empty virtual base may be the primary
// base): struct Z : virtual N { int z; } with N {vptr} has N as primary at
// offset 0 and sizeof 16; expect exit 0
struct N { virtual int f() { return 1; } };
struct Z : virtual N { int z; };
int main() { Z z; return sizeof(Z) == 16 && (char*)static_cast<N*>(&z) == (char*)&z ? 0 : 1; }
