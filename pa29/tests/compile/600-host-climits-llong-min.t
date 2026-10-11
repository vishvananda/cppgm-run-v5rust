// Header-free (intrinsics PA): validate the predefined integer-limit macros that
// <climits>' LLONG_MIN / LLONG_MAX / CHAR_BIT are built on, without pulling a header.
static_assert(__LONG_LONG_MAX__ == 0x7fffffffffffffffLL,
              "__LONG_LONG_MAX__ must be the 64-bit signed max");
static_assert((-__LONG_LONG_MAX__ - 1) < 0,
              "long long min (-MAX - 1) must be negative");
static_assert(sizeof(long long) * __CHAR_BIT__ >= 64,
              "long long must be at least 64 bits");
static_assert(__CHAR_BIT__ == 8, "a byte must be 8 bits");
