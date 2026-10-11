#if !__has_builtin(__builtin_memset)
#error expected __builtin_memset
#endif

void *fill(char *buffer, unsigned long size)
{
  return __builtin_memset(buffer, 0x5a, size);
}

int main()
{
  char buffer[8];
  return fill(buffer, sizeof(buffer)) == buffer ? 0 : 1;
}
