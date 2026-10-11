struct Guard {
  Guard() {}
  ~Guard() {}
};

int f(bool b) {
  Guard guard;
  if (b) {
    Guard guard;
    return 1;
  }
  return 0;
}
