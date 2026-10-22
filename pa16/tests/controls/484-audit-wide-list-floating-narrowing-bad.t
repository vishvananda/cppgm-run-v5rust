// N3485 8.5.4p7: the constant exception uses the frozen integral projection.
// expect reject
using U = __uint128_t;
double value{U(1) << 100};
