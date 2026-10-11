// HHC-175
unsigned long&& g(unsigned long&);

void f(unsigned long& x) {
  unsigned long y = 0;
  y = g(x);
}
