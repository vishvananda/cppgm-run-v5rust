int g(int x) { return x; }
long g(long x) { return x; }
int f() {
  int (*fp)(int) = g;
  return 0;
}
