typedef unsigned long dev_t;

dev_t f(int x, int y) {
  return (dev_t)(((x) << 24) | (y));
}
