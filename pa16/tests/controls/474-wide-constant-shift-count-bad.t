// N3485 5.19; supported __int128 extension.
// expect reject
constexpr __uint128_t value = __uint128_t(1) << (__uint128_t(1) << 64);
