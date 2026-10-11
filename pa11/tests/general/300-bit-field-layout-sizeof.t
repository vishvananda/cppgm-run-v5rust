struct Bits {
  unsigned a : 1;
  unsigned b : 2;
};

int main() {
  return sizeof(Bits);
}
