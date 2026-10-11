long &ref(long &x) {
  return x;
}

int f() {
  long x = 1;
  int y = 2;
  ref(x) += y;
  return (int)x;
}
