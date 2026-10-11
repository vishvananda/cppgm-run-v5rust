void copy4(char *dst, const char *src) {
  __builtin_memcpy(dst, src, 4);
}

void move4(char *dst, const char *src) {
  __builtin_memmove(dst, src, 4);
}
