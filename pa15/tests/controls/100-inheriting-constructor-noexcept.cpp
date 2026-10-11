// N3485 15.4p14: an inheriting constructor's implicit exception-specification
// includes the member initializers it runs (M(int) may throw); expect exit 0
struct M { M(int) {} };
struct K1 { K1(int) noexcept {} };
struct K2 : K1 { using K1::K1; M m = M(1); };
static_assert(!noexcept(K2(1)), "");
int main() { return 0; }
