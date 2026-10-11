struct Bits {
  unsigned a : 1;
  unsigned b : 2;

  Bits() : a(1), b(2) {}
};

int main() {
  Bits x;
  Bits y = x;
  y = x;
  return 0;
}
