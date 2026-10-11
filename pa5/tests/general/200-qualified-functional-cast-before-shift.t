namespace numbers {
using word = unsigned long;
}

unsigned long mask(unsigned int bits) {
  return numbers::word(1) << bits;
}
