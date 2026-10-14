// N3485 8.5p7, 7.1.5p4, 5.19: a base value-initialized by the mem-initializer
// A() (trivial default ctor) is zero-initialized, so constexpr B bb is a
// constant; expect exit 0
struct A { int a; };
struct B : A { int b; constexpr B() : A(), b(1) {} };
constexpr B bb;
int main() { return bb.a + bb.b == 1 ? 0 : 1; }
