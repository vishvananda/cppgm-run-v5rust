// expect reject
// The selected cooked literal operator demands a representable long double value.
int operator""_value(long double x) { return int(x); }
int main() { return 0x1p+99999_value; }
