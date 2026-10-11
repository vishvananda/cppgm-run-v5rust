// A hosted type specifier spells itself as an identifier rather than a
// keyword, so it never reaches the fundamental-spelling table that turns
// `int(x)` into a conversion.  It introduces a functional cast the same way:
// libc++'s charconv tables build their 128-bit constants as
// `__uint128_t(UINT64_C(...)) * UINT64_C(10)`.

int main()
{
  __uint128_t wide = __uint128_t(10000000000000000000ULL) * 10ULL;
  __int128_t negative = __int128_t(-5);
  __float128 quad = __float128(10);

  // The conversion has to produce the specifier's own type, not merely parse:
  // the multiply overflows 64 bits and only survives in 128.
  const bool wide_ok = wide / 10ULL == __uint128_t(10000000000000000000ULL);
  const bool negative_ok = negative == -5;
  const bool quad_ok = quad == 10;

  return (wide_ok && negative_ok && quad_ok) ? 0 : 1;
}
