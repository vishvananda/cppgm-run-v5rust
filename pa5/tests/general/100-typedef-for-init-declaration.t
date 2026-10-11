// HHC-081
typedef unsigned long size_t;

void f(size_t n) {
  size_t count = n;
  for (size_t i = 0; i != count; ++i) {
  }
}
