// N3485 7.1.5p4, 9.5p2: a union whose variant member has a
// brace-or-equal-initializer (int i = 9) is fully initialized by constexpr V()
// {}; expect exit 0
union V { int i = 9; char c; constexpr V() {} };
constexpr V v;
int main() { return v.i == 9 ? 0 : 1; }
