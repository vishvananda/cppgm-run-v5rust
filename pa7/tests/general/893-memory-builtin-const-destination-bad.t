// A builtin uses ordinary pointer conversion rules; const storage is not writable.
void copy(const char *dst, const char *src) {
  __builtin_memcpy(dst, src, 4);
}
