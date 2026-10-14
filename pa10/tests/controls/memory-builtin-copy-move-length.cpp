// expect exit 0
inline void unused(char *dst, const char *src) {
  __builtin_memcpy(dst, src, 4);
}
void *copy(void *dst, const void *src, unsigned long n) {
  return ::__builtin_memcpy(dst, src, n);
}
int main() {
  char src[4] = {1, 2, 3, 0};
  char dst[4] = {0, 0, 0, 0};
  if (copy(dst, src, 4) != dst) return 1;
  if (__builtin_strlen(dst) != 3) return 2;
  if (__builtin_memmove(dst + 1, dst, 3) != dst + 1) return 3;
  if (dst[0] != 1 || dst[1] != 1 || dst[2] != 2 || dst[3] != 3) return 4;
  void *(*p)(void *, const void *, unsigned long) = __builtin_memcpy;
  if (p(dst, src, 4) != dst) return 5;
  return dst[2] == 3 && dst[3] == 0 ? 0 : 6;
}
