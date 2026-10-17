// expect reject
// N3485 5.19 and 5.6: division by zero cannot feed a constant template argument.
template<int N> struct Value {};
Value<int(1.0 / 0.0)> value;
int main() { return 0; }
