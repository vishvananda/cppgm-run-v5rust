typedef unsigned __int128 wide;

static const wide pow10_table[] = {
  1,
  10000000000000000000ull,
  static_cast<wide>(10000000000000000000ull) * 10,
  static_cast<wide>(10000000000000000000ull) * 100,
  static_cast<wide>(10000000000000000000ull) * 10000000000000000000ull,
  ~static_cast<wide>(0),
};

static const __int128 signed_table[] = {
  -1,
  -static_cast<__int128>(10000000000000000000ull) * 10,
  static_cast<__int128>(10000000000000000000ull) * 10,
};

wide pow10_at(unsigned index) { return pow10_table[index]; }
__int128 signed_at(unsigned index) { return signed_table[index]; }
wide add_big(wide value) { return value + (static_cast<wide>(10000000000000000000ull) * 10); }
