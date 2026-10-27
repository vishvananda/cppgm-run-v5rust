// N3485 5.2.2, 5.19: parameter conversions and defaults precede the
// function's result, and different functions/arguments stay distinct.
// expect exit 0
constexpr int value(unsigned char n = 3) { return n + 1; }
constexpr int other(unsigned char n) { return n + 2; }
constexpr int zero() { return 7; }
constexpr int result = value() + value(3) + value(259) + other(3) + zero() + zero();
static_assert(result == 31, "converted arguments and identities");
int main() { return result == 31 ? 0 : 1; }
