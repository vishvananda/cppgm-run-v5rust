template<class T>
T exchange_n(T* p, T value) {
  return __atomic_exchange_n(p, value, __ATOMIC_ACQ_REL);
}

template<class T>
bool compare_exchange_n(T* p, T* expected, T desired) {
  return __atomic_compare_exchange_n(
      p, expected, desired, false, __ATOMIC_SEQ_CST, __ATOMIC_ACQUIRE);
}

int main() {
  long value = 1;
  __atomic_store_n(&value, 2L, __ATOMIC_RELEASE);
  long loaded = __atomic_load_n(&value, __ATOMIC_ACQUIRE);
  long old = exchange_n(&value, 3L);

  long expected = 3;
  bool exchanged = compare_exchange_n(&value, &expected, 4L);
  long wrong_expected = 99;
  bool missed = compare_exchange_n(&value, &wrong_expected, 5L);

  unsigned bits = 7;
  unsigned a = __atomic_fetch_and(&bits, 3u, __ATOMIC_RELAXED);
  unsigned b = __atomic_fetch_or(&bits, 8u, __ATOMIC_RELEASE);
  unsigned c = __atomic_fetch_xor(&bits, 1u, __ATOMIC_ACQ_REL);

  unsigned char lock = 0;
  bool was_set = __atomic_test_and_set(&lock, __ATOMIC_ACQUIRE);
  bool became_set = lock != 0;
  __atomic_clear(&lock, __ATOMIC_RELEASE);

  return loaded == 2 &&
         old == 2 &&
         exchanged &&
         expected == 3 &&
         !missed &&
         wrong_expected == 4 &&
         value == 4 &&
         a == 7 &&
         b == 3 &&
         c == 11 &&
         bits == 10 &&
         !was_set &&
         became_set &&
         lock == 0 ? 0 : 1;
}
