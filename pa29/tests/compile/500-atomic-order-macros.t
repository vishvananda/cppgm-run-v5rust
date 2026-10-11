static_assert(__ATOMIC_RELAXED == 0, "");
static_assert(__ATOMIC_CONSUME == 1, "");
static_assert(__ATOMIC_ACQUIRE == 2, "");
static_assert(__ATOMIC_RELEASE == 3, "");
static_assert(__ATOMIC_ACQ_REL == 4, "");
static_assert(__ATOMIC_SEQ_CST == 5, "");

#if !defined(__CLANG_ATOMIC_INT_LOCK_FREE) && !defined(__GCC_ATOMIC_INT_LOCK_FREE)
#error missing host atomic lock-free macro family
#endif

int main() {
  return 0;
}
