// expect exit 0
// Frozen source-compiler dialect: pp-number exponents preserve raw pack spelling.
#define JOIN(a, b) a##b
template<char... C> int operator""_digits() { return sizeof...(C); }
int operator""_raw(const char* p) { return p[0] == '0' && p[3] == '.' && p[5] == 'p' && p[6] == '+' && p[7] == '2' ? 8 : 0; }
int operator""_cooked(long double value) { return int(value); }
int main() {
  return 0x1.fp+2_digits - 8 + 0x1.fp+2_raw - 8
    + 0x1.fp+2_cooked - 7 + JOIN(0x1.fp+, 2_digits) - 8;
}
