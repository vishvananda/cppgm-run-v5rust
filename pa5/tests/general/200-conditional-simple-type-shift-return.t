unsigned f(unsigned w) {
  return w < 64 ? unsigned(~0) >> 1 : unsigned(0);
}
