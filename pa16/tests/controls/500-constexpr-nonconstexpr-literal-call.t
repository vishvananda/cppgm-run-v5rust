// N3485 5.19p2: a non-constexpr literal operator cannot initialize a constexpr variable.
// expect reject
int operator""_value(unsigned long long n) { return n; }
constexpr int value = 3_value;
