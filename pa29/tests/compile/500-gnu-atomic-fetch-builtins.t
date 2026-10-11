template<class T>
T refcount_inc(T& value) {
  return __atomic_add_fetch(&value, 1, __ATOMIC_RELAXED);
}

template<class T>
T refcount_dec(T& value) {
  return __atomic_add_fetch(&value, -1, __ATOMIC_ACQ_REL);
}

static_assert(__atomic_always_lock_free(4, 0), "");
static_assert(__atomic_is_lock_free(8, 0), "");

int main() {
  long value = 10;
  long a = refcount_inc(value);
  long b = __atomic_fetch_add(&value, 5, __ATOMIC_RELAXED);
  long c = __atomic_fetch_sub(&value, 3, __ATOMIC_ACQ_REL);
  long d = refcount_dec(value);
  return a == 11 && b == 11 && c == 16 && d == 12 && value == 12 ? 0 : 1;
}
