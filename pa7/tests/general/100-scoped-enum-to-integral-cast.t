// HHC-181
enum class E : unsigned long {
  A = 4
};

unsigned long f(E e) {
  return static_cast<unsigned long>(e);
}
