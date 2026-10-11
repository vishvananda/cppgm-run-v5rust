#if !__has_builtin(__sync_lock_test_and_set)
#error expected __sync_lock_test_and_set
#endif

#if !__has_builtin(__sync_lock_release)
#error expected __sync_lock_release
#endif

int main()
{
  int lock = 0;
  int old = __sync_lock_test_and_set(&lock, 1);
  __sync_lock_release(&lock);
  return old == 0 && lock == 0 ? 0 : 1;
}
