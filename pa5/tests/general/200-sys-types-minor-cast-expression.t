typedef unsigned int __uint32_t;
typedef int __int32_t;

__int32_t minor(__uint32_t _x) {
  return (__int32_t)((_x) & 0xffffff);
}
