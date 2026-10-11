struct Bits {
  unsigned a : 1;
};

const unsigned long V = __builtin_offsetof(Bits, a);

int main() {
  return 0;
}
