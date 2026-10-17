// expect exit 0
// N3485 5.19 and 5.2.9: conversions feeding integral template arguments.
template<int N> struct Value { static const int value = N; };
static_assert(Value<int(1.5 + 2.5)>::value == 4, "arithmetic before truncation");
static_assert(Value<static_cast<int>(-3.75)>::value == -3, "truncate toward zero");
static_assert(Value<int(2.5f)>::value == 2, "single precision");
static_assert(Value<int(2.5L)>::value == 2, "extended precision");
static_assert(Value<bool(0.5)>::value == 1, "floating truth");
static_assert(Value<int(false ? 1.0 / 0.0 : 3.5)>::value == 3, "selected operand");
static_assert(Value<(true || int(1e300))>::value == 1, "short circuit");
static_assert(Value<int((unsigned char)(-0.9))>::value == 0, "truncate before range");
int main() { return Value<int(11.0 / 4.0)>::value - 2; }
