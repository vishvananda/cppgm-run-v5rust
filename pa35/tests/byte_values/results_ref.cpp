#include <cstdint>
__attribute__((noinline)) bool ext_b(long x) { return x > 3; }
__attribute__((noinline)) unsigned char ext_u(long x) { return (unsigned char)(x * 7); }
__attribute__((noinline)) signed char ext_s(long x) { return (signed char)(x * 13); }
