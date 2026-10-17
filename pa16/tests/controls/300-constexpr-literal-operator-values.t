// N3485 2.14.8 and 7.1.5: literal operators participate in constant evaluation.
// expect exit 0
constexpr unsigned long long operator""_value(unsigned long long n) { return n + 1; }
constexpr long double operator""_value(long double n) { return n + 1; }
constexpr int operator""_value(char c) { return c + 1; }
constexpr int operator""_value(const char* p, unsigned long n) { return p[1] + n; }
constexpr int operator""_raw(const char* p) { return p[0] + p[2]; }
template<char... C> constexpr int operator""_count() { return sizeof...(C); }
struct Number { int n; constexpr Number(int n) : n(n) {} };
constexpr Number operator""_number(unsigned long long n) { return Number(n); }
constexpr auto integer = 3_value;
constexpr auto floating = 1.5_value;
constexpr int character = 'A'_value;
constexpr int string = "ABC"_value;
constexpr int raw = 123_raw;
constexpr int count = 123_count;
constexpr Number object = 7_number;
static_assert(integer == 4 && floating == 2.5L && character == 66, "scalar arguments");
static_assert(string == 69 && raw == 100 && count == 3 && object.n == 7, "literal shapes");
int main() { return integer + int(floating) + character + string + raw + count + object.n == 251 ? 0 : 1; }
