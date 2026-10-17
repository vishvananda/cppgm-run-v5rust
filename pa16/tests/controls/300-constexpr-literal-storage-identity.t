// N3485 2.14.5p12, 2.14.8p3,p5: literal array identity is independent of its suffix.
// expect exit 0
constexpr const char* operator""_first(const char* p, unsigned long) { return p; }
constexpr const char* operator""_second(const char* p, unsigned long) { return p; }
constexpr const char* operator""_raw(const char* p) { return p; }
constexpr char16_t operator""_unit(const char16_t* p, unsigned long) { return p[1]; }
constexpr auto a = "123"_first;
constexpr auto b = "123"_second;
constexpr auto c = 123_raw;
constexpr auto d = 456_raw;
static_assert(a == b && b == c && c == "123" && c != d, "shared code units, distinct arrays");
static_assert(c[3] == 0 && u"AB"_unit == 66, "terminator and wide units");
int main() { return a == b && b == c && c[2] == '3' && d[0] == '4' ? 0 : 1; }
