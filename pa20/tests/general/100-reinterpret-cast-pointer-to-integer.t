// HHC-102
typedef unsigned long size_t;

size_t f(const char* p) {
  return reinterpret_cast<size_t>(p);
}

int main() {
  return 0;
}
