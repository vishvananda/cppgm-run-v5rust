// VALIDATION: compile-pass
// GNU/Clang builtin compatibility: __builtin_vsnprintf with __builtin_va_list.

typedef __builtin_va_list va_list;

int format(const char *fmt, ...)
{
  char buf[32];
  va_list ap;
  __builtin_va_start(ap, fmt);
  int n = __builtin_vsnprintf(buf, sizeof(buf), fmt, ap);
  __builtin_va_end(ap);
  return n;
}

int main()
{
  return format("%d", 7) < 0;
}
