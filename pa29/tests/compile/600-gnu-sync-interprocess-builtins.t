#if !__has_builtin(__sync_synchronize)
#error expected __sync_synchronize
#endif

#if !__has_builtin(__sync_fetch_and_add)
#error expected __sync_fetch_and_add
#endif

#if !__has_builtin(__sync_val_compare_and_swap)
#error expected __sync_val_compare_and_swap
#endif

typedef unsigned int uint32_t;

uint32_t atomic_read32(volatile uint32_t *mem)
{
  uint32_t old_val = *mem;
  __sync_synchronize();
  return old_val;
}

uint32_t atomic_add32(volatile uint32_t *mem, uint32_t val)
{
  return __sync_fetch_and_add(const_cast<uint32_t *>(mem), val);
}

uint32_t atomic_cas32(volatile uint32_t *mem, uint32_t with, uint32_t cmp)
{
  return __sync_val_compare_and_swap(const_cast<uint32_t *>(mem), cmp, with);
}

void atomic_write32(volatile uint32_t *mem, uint32_t val)
{
  __sync_synchronize();
  *mem = val;
}

int main()
{
  volatile uint32_t value = 7u;
  uint32_t read = atomic_read32(&value);
  uint32_t old_add = atomic_add32(&value, 5u);
  uint32_t old_cas_fail = atomic_cas32(&value, 20u, 99u);
  uint32_t old_cas_ok = atomic_cas32(&value, 20u, 12u);
  atomic_write32(&value, 3u);
  return read == 7u &&
         old_add == 7u &&
         old_cas_fail == 12u &&
         old_cas_ok == 12u &&
         value == 3u ? 0 : 1;
}
