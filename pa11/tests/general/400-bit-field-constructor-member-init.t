struct Bits {
  unsigned a : 1;
  unsigned b : 2;

  Bits() : a(1), b(2) {}
};

int main() {
  Bits bits;
  return 0;
}
