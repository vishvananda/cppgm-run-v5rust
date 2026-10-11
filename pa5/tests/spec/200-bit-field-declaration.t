// N3485 focus: 9.6 [class.bit] bit-field declarations
struct Bits {
  unsigned a : 1;
  unsigned b : 2;
  char c : 0 ? 1 : 2;
};
