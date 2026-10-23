// Own-width generic operations, suffix conversions and constexpr demand.
constexpr int population(unsigned long x) { return __builtin_popcountl(x); }
constexpr int trailing(unsigned x, int zero) { return __builtin_ctzg(x, zero); }
static_assert(population(0xfful) == 8, "");
static_assert(trailing(0u, -91) == -91, "");
static_assert(__builtin_clzg((unsigned char)1) == 7, "");
static_assert(__builtin_ctzg((unsigned short)0, (short)-1) == -1, "");
static_assert(__builtin_popcountg((unsigned __int128)-1) == 128, "");
static_assert(__builtin_clzg((unsigned __int128)1) == 127, "");
static_assert(__builtin_ctzg((unsigned __int128)1 << 95) == 95, "");
static_assert(__builtin_clzg((unsigned __int128)0, 128) == 128, "");
static_assert(__builtin_bswap16(0x123456u) == 0x5634u, "");
static_assert(__builtin_popcount((signed char)-1) == 32, "");
static_assert(noexcept(::__builtin_popcount(1u)), "");
template<class T> auto count(T x) -> decltype(__builtin_popcountg(x)) { return 1; }
int count(...) { return 2; }
int instantiated() { return count(-1) + count(1u); }
