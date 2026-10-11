typedef unsigned long dev_t;

dev_t f(int x) {
  return (dev_t)((x) << 24);
}
