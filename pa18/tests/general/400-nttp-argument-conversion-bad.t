// N3485 14.3.2p5: a pointer-to-object template argument takes only
// qualification and array-to-pointer conversions (no int*->void*, no
// derived-to-base, no integral 0); must be rejected
int x;
template<void* P> struct A { static void* get() { return P; } };
int main() { return A<&x>::get() == &x ? 0 : 1; }
