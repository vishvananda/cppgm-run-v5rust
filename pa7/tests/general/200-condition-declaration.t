int g(int x) { return x; }
int f() {
  if (int x = g(1))
    return x;
  return 0;
}
