int g(int x) { return x; }
int f() {
  int (*fp)(int) = g;
  return (*fp)(0);
}
