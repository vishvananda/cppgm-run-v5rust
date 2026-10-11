#include <cstdint>
#define SW(T, name) \
long name##_sw(T x) { switch (x) { case 0: return 10; case 1: return 11; case 2: return 12; case 3: return 13; case 4: return 14; case 5: return 15; case 7: return 17; default: return -1; } } \
long name##_sp(T x) { switch (x) { case 1: return 1; case 40: return 2; case 100: return 3; default: return 4; } } \
long name##_br(T x, long a) { if (x) return a + 1; return a - 1; } \
long name##_both(T x, long* p) { p[0] = (long)(T)x; if (x) { switch (x) { case 1: case 2: case 3: case 5: case 8: case 9: return 5; case 6: return 6; default: return 7; } } return 0; }
SW(uint8_t, u8) SW(int8_t, i8) SW(uint16_t, u16) SW(int16_t, i16) SW(uint32_t, u32) SW(int32_t, i32)
long b_br(bool x, long a) { if (x) return a + 1; return a - 1; }
