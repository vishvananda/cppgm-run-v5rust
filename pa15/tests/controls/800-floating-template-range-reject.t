// expect reject
// N3485 4.9: a truncated value outside the destination range is not constant.
template<int N> struct Value {};
Value<static_cast<int>(2147483648.0)> value;
int main() { return 0; }
