// The libm intrinsic family a hosted <cmath> forwards to.  Each takes its own
// result type in every operand and lowers to the C function of the same name.
float unary_float(float x) {
  return __builtin_fabsf(x) + __builtin_sqrtf(x) + __builtin_floorf(x) +
         __builtin_ceilf(x) + __builtin_truncf(x) + __builtin_roundf(x) +
         __builtin_sinf(x) + __builtin_cosf(x) + __builtin_tanf(x) +
         __builtin_expf(x) + __builtin_logf(x) + __builtin_cbrtf(x);
}

double unary_double(double x) {
  return __builtin_fabs(x) + __builtin_sqrt(x) + __builtin_log2(x) +
         __builtin_log10(x) + __builtin_asin(x) + __builtin_acos(x) +
         __builtin_atan(x) + __builtin_sinh(x) + __builtin_cosh(x) +
         __builtin_tanh(x) + __builtin_lgamma(x) + __builtin_tgamma(x) +
         __builtin_rint(x) + __builtin_nearbyint(x) + __builtin_logb(x) +
         __builtin_erf(x) + __builtin_erfc(x) + __builtin_expm1(x) +
         __builtin_log1p(x) + __builtin_exp2(x);
}

long double unary_long_double(long double x) {
  return __builtin_fabsl(x) + __builtin_sqrtl(x) + __builtin_atanhl(x) +
         __builtin_asinhl(x) + __builtin_acoshl(x);
}

double binary(double x, double y) {
  return __builtin_pow(x, y) + __builtin_fmod(x, y) + __builtin_atan2(x, y) +
         __builtin_hypot(x, y) + __builtin_copysign(x, y) +
         __builtin_fmax(x, y) + __builtin_fmin(x, y) + __builtin_fdim(x, y) +
         __builtin_nextafter(x, y) + __builtin_remainder(x, y);
}

float binary_float(float x, float y) { return __builtin_powf(x, y); }

double ternary(double x, double y, double z) { return __builtin_fma(x, y, z); }

int main() { return 0; }
