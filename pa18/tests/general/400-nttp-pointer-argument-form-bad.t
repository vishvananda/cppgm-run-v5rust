// N3485 14.3.2p1: a pointer template argument must be written & id-expression
// (or be a null pointer value); a constexpr pointer variable holding &x is not
// one; must be rejected
int x;
constexpr int* q = &x;
template<int* P> struct A { static int* get() { return P; } };
int main() { return A<q>::get() == &x ? 0 : 1; }
