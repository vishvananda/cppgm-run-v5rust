#include <cstdio>
#include <cstdint>
template <class T> struct typename_u; template <> struct typename_u<uint8_t> { typedef uint8_t type; }; template <> struct typename_u<int8_t> { typedef uint8_t type; }; template <> struct typename_u<uint16_t> { typedef uint16_t type; }; template <> struct typename_u<int16_t> { typedef uint16_t type; }; template <> struct typename_u<uint32_t> { typedef uint32_t type; }; template <> struct typename_u<int32_t> { typedef uint32_t type; };
#define DECL(T, name) long name##_sw(T); long name##_sp(T); long name##_br(T, long); long name##_both(T, long*);
DECL(uint8_t, u8) DECL(int8_t, i8) DECL(uint16_t, u16) DECL(int16_t, i16) DECL(uint32_t, u32) DECL(int32_t, i32)
long b_br(bool, long);
typedef long (*F1)(long); typedef long (*F2)(long, long); typedef long (*F3)(long, long*);
int main() {
  long sum = 0; long cell = 0;
  const unsigned long junk[] = { 0, 0xffffffffffffff00ul, 0x1234567812345600ul, 0xdeadbeef00000000ul, 0xffff0000ul, 0x100ul };
  const long vals[] = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 40, 100, 127, 128, 255, 256, 65535, 65536, -1, -2, -128, -129, 0x7fffffff, 0x80000000L };
#define RUN(T, name) \
  for (unsigned j = 0; j < 6; ++j) for (unsigned v = 0; v < sizeof(vals) / sizeof(vals[0]); ++v) { \
    T t = (T)vals[v]; unsigned long bits = (unsigned long)(uint64_t)(typename_u<T>::type)t; \
    unsigned long width = sizeof(T) * 8; unsigned long arg = width == 64 ? bits : ((junk[j] >> width << width) | bits); \
    sum = sum * 31 + ((F1)(void*)name##_sw)((long)arg); sum = sum * 31 + ((F1)(void*)name##_sp)((long)arg); \
    sum = sum * 31 + ((F2)(void*)name##_br)((long)arg, 5); sum = sum * 31 + ((F3)(void*)name##_both)((long)arg, &cell) + cell; }
  RUN(uint8_t, u8) RUN(int8_t, i8) RUN(uint16_t, u16) RUN(int16_t, i16) RUN(uint32_t, u32) RUN(int32_t, i32)
  for (unsigned j = 0; j < 6; ++j) for (int b = 0; b < 2; ++b) sum = sum * 31 + ((F2)(void*)b_br)((long)((junk[j] & ~0xfful) | (unsigned long)b), 5);
  printf("%ld\n", sum);
}
