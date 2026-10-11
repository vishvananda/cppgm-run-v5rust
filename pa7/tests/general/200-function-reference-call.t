int g(int x) { return x; }
int invoke(int (&rf)(int), int x) {
  return rf(x);
}
