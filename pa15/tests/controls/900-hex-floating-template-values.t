// expect exit 0
// Frozen source-compiler dialect: hexadecimal values use shared target rounding.
#if 0b101 != 5
#error binary conditional value
#endif
template<int N> struct Value { static const int value = N; };
static_assert(Value<int(0x1.fp+2)>::value == 7, "positive exponent");
static_assert(Value<int(0X.8P+3f)>::value == 4, "fraction and uppercase exponent");
static_assert(Value<int(0x1p-1L * 8)>::value == 4, "negative exponent");
static_assert(Value<0b101>::value == 5, "binary integral argument");
int main() { return Value<int(0x1.8p+1)>::value - 3; }
