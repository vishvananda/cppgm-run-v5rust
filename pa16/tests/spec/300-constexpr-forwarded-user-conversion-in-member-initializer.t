// N3485 focus: 5.19 [expr.const], 12.6.2 [class.base.init]
struct X { constexpr operator int() const { return 2; } };
struct storage { int value; constexpr storage(X&& x): value(static_cast<X&&>(x)) {} };
static_assert(storage(X{}).value == 2, "value");
int main() {}
