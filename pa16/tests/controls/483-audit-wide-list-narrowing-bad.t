// N3485 8.5.4p7: a constant beyond the integral projection cannot exempt narrowing.
// expect reject
using U = __uint128_t;
int value{U(1) << 100};
