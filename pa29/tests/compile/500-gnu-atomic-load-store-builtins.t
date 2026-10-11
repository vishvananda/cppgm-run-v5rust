template<class T>
T load_n(const T* p) {
  return __atomic_load_n(p, __ATOMIC_RELAXED);
}

void store_ptr(int* p, int value) {
  int tmp = value;
  __atomic_store(p, &tmp, __ATOMIC_RELEASE);
}

int load_ptr(const int* p) {
  int out = *p;
  __atomic_load(p, &out, __ATOMIC_ACQUIRE);
  return out;
}

int main() {
  int x = 7;
  store_ptr(&x, 11);
  return load_n(&x) == 11 && load_ptr(&x) == 11 ? 0 : 1;
}
