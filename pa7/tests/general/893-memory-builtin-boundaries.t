// Reserved memory functions retain ordinary typed lookup and conversion facts.
void *copy(void *dst, const void *src, unsigned long n) {
  return ::__builtin_memcpy(dst, src, n);
}
void *move(void *dst, const void *src, unsigned long n) {
  return __builtin_memmove(dst, src, n);
}
void *fill(void *dst, int value, unsigned long n) {
  return __builtin_memset(dst, value, n);
}
int compare(const void *a, const void *b, unsigned long n) {
  return __builtin_memcmp(a, b, n);
}
unsigned long length(const char *text) { return __builtin_strlen(text); }
void *(*copy_pointer)(void *, const void *, unsigned long) = __builtin_memcpy;
void *again(void *dst, const void *src) { return copy_pointer(dst, src, 4); }
int query(void *dst, const void *src) {
  return noexcept(__builtin_memcpy(dst, src, 4));
}
