enum class F {
  a = 1,
  b = 2,
  both = 3
};

F operator&(F lhs, F rhs) {
  (void)lhs;
  (void)rhs;
  return static_cast<F>(3);
}

int main() {
  F value = static_cast<F>(1);
  value = value & static_cast<F>(2);
  return value == static_cast<F>(3) ? 0 : 1;
}
