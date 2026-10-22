// N3485 5.19; supported __int128 extension.
// expect reject
constexpr __int128_t value = (__int128_t(1) << 126) * 2;
