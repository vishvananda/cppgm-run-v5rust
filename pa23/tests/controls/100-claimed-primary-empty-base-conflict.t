// Itanium ABI 2.4 (empty-subobject conflicts include those of a virtual
// primary base claimed by a base): in X1 : K1, K0, K2 the K2 subobject cannot
// sit at 8 (its claimed primary K1 would put an empty K0 on X1's K0);
// g++/clang: sizeof 24, K2@16; expect exit 0
struct K0 {};
struct K1 : K0 { virtual void f1() {} };
struct K2 : virtual K1 {};
struct X1 : K1, K0, K2 {};
int main() { X1 x; return sizeof(X1) == 24 && (char*)static_cast<K2*>(&x) - (char*)&x == 16 ? 0 : 1; }
