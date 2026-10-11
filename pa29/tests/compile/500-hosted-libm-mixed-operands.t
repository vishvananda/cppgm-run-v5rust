// The libm entries whose operands are not all the floating type: an exponent
// read back through a pointer, an exponent passed in, an integral result, and
// a second operand that is always long double.
double exponent_out(double x, int* e) { return __builtin_frexp(x, e); }
float exponent_out_float(float x, int* e) { return __builtin_frexpf(x, e); }
long double exponent_out_long(long double x, int* e) { return __builtin_frexpl(x, e); }

double exponent_in(double x, int e) { return __builtin_ldexp(x, e); }
double scaled(double x, int e) { return __builtin_scalbn(x, e); }
double scaled_long(double x, long e) { return __builtin_scalbln(x, e); }

double whole_part(double x, double* part) { return __builtin_modf(x, part); }

double quotient(double x, double y, int* q) { return __builtin_remquo(x, y, q); }

double toward(double x, long double y) { return __builtin_nexttoward(x, y); }

int exponent_of(double x) { return __builtin_ilogb(x); }
long nearest_long(double x) { return __builtin_lrint(x) + __builtin_lround(x); }
long long nearest_long_long(double x) {
  return __builtin_llrint(x) + __builtin_llround(x);
}

int main() { return 0; }
