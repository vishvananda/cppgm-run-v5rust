// Index tags and comparison operand types retain the frozen alias facts.
unsigned objects[10];
unsigned short shorts[10];
unsigned char bytes[10];
__attribute__((noinline)) unsigned address32(bool c, unsigned n) {
  return c ? 0u : (unsigned)(unsigned long)&objects[n];
}
__attribute__((noinline)) unsigned address16(bool c, unsigned n) {
  return c ? 0u : (unsigned)(unsigned long)&shorts[n];
}
__attribute__((noinline)) unsigned address8(bool c, unsigned n) {
  return c ? 0u : (unsigned)(unsigned long)&bytes[n];
}
__attribute__((noinline)) unsigned reversed(bool c, unsigned n) {
  return c ? (unsigned)(unsigned long)&objects[n] : 0u;
}
__attribute__((noinline)) unsigned compare(double a, double b, unsigned x) {
  unsigned c = a < b;
  return (c + x) ^ c;
}
int main() {
  volatile unsigned indices[] = {0, 1, 4, 9};
  volatile bool choices[] = {false, true};
  for (int i = 0; i < 4; ++i) {
    unsigned n = indices[i];
    for (int j = 0; j < 2; ++j) {
      bool c = choices[j];
      if (address32(c, n) != (c ? 0u : (unsigned)(unsigned long)&objects[n])) return 1;
      if (address16(c, n) != (c ? 0u : (unsigned)(unsigned long)&shorts[n])) return 2;
      if (address8(c, n) != (c ? 0u : (unsigned)(unsigned long)&bytes[n])) return 3;
      if (reversed(c, n) != (c ? (unsigned)(unsigned long)&objects[n] : 0u)) return 4;
    }
    if (compare(1.0, 2.0, n) != ((1u + n) ^ 1u)) return 5;
    if (compare(2.0, 1.0, n) != n) return 6;
  }
  return 0;
}
// Floating element tags reserve the frozen backend scratch area even though
// taking an element's address performs no floating arithmetic.
float floats[10];
double doubles[10];
long double extended[10];
__attribute__((noinline)) float* float_address(unsigned n) { return &floats[n]; }
__attribute__((noinline)) double* double_address(unsigned n) { return &doubles[n]; }
__attribute__((noinline)) long double* extended_address(unsigned n) { return &extended[n]; }
