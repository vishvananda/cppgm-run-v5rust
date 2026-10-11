template<class T>
struct atomic_box {
  _Atomic(T) value;
};

int load_box(const atomic_box<int>* p) {
  return __c11_atomic_load(&p->value, __ATOMIC_ACQUIRE);
}

void init_box(atomic_box<int>* p, int value) {
  __c11_atomic_init(&p->value, value);
}

void store_box(atomic_box<int>* p, int value) {
  __c11_atomic_store(&p->value, value, __ATOMIC_RELEASE);
}

int exchange_box(atomic_box<int>* p, int value) {
  return __c11_atomic_exchange(&p->value, value, __ATOMIC_ACQ_REL);
}

bool cas_strong_box(atomic_box<int>* p, int* expected, int desired) {
  return __c11_atomic_compare_exchange_strong(
      &p->value, expected, desired, __ATOMIC_SEQ_CST, __ATOMIC_ACQUIRE);
}

bool cas_weak_box(atomic_box<int>* p, int* expected, int desired) {
  return __c11_atomic_compare_exchange_weak(
      &p->value, expected, desired, __ATOMIC_SEQ_CST, __ATOMIC_RELAXED);
}

int fetch_math(atomic_box<int>* p) {
  int a = __c11_atomic_fetch_add(&p->value, 5, __ATOMIC_RELAXED);
  int b = __c11_atomic_fetch_sub(&p->value, 3, __ATOMIC_ACQ_REL);
  return a + b;
}

unsigned fetch_bits(atomic_box<unsigned>* p) {
  unsigned a = __c11_atomic_fetch_and(&p->value, 3u, __ATOMIC_RELAXED);
  unsigned b = __c11_atomic_fetch_or(&p->value, 4u, __ATOMIC_RELEASE);
  unsigned c = __c11_atomic_fetch_xor(&p->value, 1u, __ATOMIC_ACQ_REL);
  return a ^ b ^ c;
}

static_assert(__c11_atomic_is_lock_free(sizeof(int)), "");

int main() {
  atomic_box<int> ints;
  atomic_box<unsigned> bits;
  init_box(&ints, 1);
  store_box(&ints, 2);
  __c11_atomic_init(&bits.value, 7u);
  int expected = 2;
  cas_strong_box(&ints, &expected, 3);
  expected = 3;
  cas_weak_box(&ints, &expected, 4);
  return load_box(&ints) + exchange_box(&ints, 5) + fetch_math(&ints) + fetch_bits(&bits);
}
