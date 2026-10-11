struct Bits {
  unsigned a : 1;
  unsigned : 0;
  unsigned b : 1;
};

int main() {
  return sizeof(Bits);
}
