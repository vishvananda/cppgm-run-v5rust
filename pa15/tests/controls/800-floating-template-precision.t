// expect exit 0
// N3485 4.8 and 5.19: retain each operand's target precision before integral conversion.
template<int N> struct Value { static const int value = N; };
static_assert(Value<int(float(16777217))>::value == 16777216, "single rounding");
static_assert(Value<int(double(9007199254740993LL) - 9007199254740992.0)>::value == 0, "double rounding");
static_assert(Value<(1.0 < 2.0 && !0.0 && 1.0 != 2.0)>::value == 1, "comparison and truth");
static_assert(Value<int((1.0L + 0.000000000000000000108420217248550443400745280086994171142578125L - 1.0L) * 9223372036854775808.0L)>::value == 1, "extended significand");
int main() { return Value<int((1.0, 2.5))>::value - 2; }
