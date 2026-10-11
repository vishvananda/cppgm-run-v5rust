inline unsigned long long lazy_header_u128_mul(
    unsigned long long x,
    unsigned long long y,
    unsigned long long & hi)
{
  __uint128_t r = (__uint128_t)x * y;
  hi = (unsigned long long)(r >> 64);
  return (unsigned long long)r;
}
