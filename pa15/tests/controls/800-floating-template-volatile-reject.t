// expect reject
// N3485 5.19: removing glvalue cv does not make a volatile object readable.
constexpr volatile double value = 2.5;
template<int N> struct Value {};
Value<int(const_cast<const double&>(value))> object;
int main() { return 0; }
