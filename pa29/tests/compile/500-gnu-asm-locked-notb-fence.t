inline void seq_cst_fence_from_locked_not()
{
  unsigned char dummy = 0u;
  __asm__ __volatile__("lock; notb %0" : "+m"(dummy) : : "memory");
}

inline void compiler_fence_from_compact_colons()
{
  __asm__ __volatile__("" ::: "memory");
}

int main()
{
  seq_cst_fence_from_locked_not();
  compiler_fence_from_compact_colons();
  return 0;
}
