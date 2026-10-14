// N3485 7.1.5p4 (a union's constexpr constructor initializes exactly one
// non-static data member), 9.5p1: initializing the non-first member f is a
// constant initialization; expect exit 0
union K1 { int i; float f; constexpr K1(float y) : f(y) {} };
constexpr K1 u(2.5f);
int main() { return u.f == 2.5f ? 0 : 1; }
