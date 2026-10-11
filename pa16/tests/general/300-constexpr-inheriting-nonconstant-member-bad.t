// N3485 12.9p6, 7.1.5p4, 7.1.5p9: an inheriting constructor whose class's
// member initializer calls a non-constexpr function is no constexpr
// constructor (no error where the class is defined); the constexpr object it
// would initialize is ill-formed.
int h();
struct B { int v; constexpr B(int x) : v(x) {} };
struct D : B { using B::B; int m = h(); };
D runtime(1);
constexpr D d(2);
int main() { return 0; }
