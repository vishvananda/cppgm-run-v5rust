// HHC-036
typedef unsigned short __uint16_t;

__uint16_t f(__uint16_t x) {
  return (__uint16_t)(x >> 8);
}
