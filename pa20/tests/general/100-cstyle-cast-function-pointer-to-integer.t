typedef unsigned long uintptr_t;
typedef int (*callback_t)(int, const void *, int);

uintptr_t cast_fn(callback_t fp) {
  return (uintptr_t)fp;
}

int main() {
  return 0;
}
