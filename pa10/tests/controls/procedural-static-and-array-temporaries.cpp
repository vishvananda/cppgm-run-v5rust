// expect exit 0
int inits;
int next() { return ++inits; }
int guarded() { static int a[3] = {next(), next()}; return a[0] + a[1] + a[2]; }
int thread_value() { thread_local int n = next(); return n; }
int read(const int (&a)[2]) { return a[0] + a[1]; }
int main() {
  const int (&a)[2][2]{{4, 5}, {6}};
  if (guarded() != 3 || guarded() != 3 || inits != 2) return 1;
  if (thread_value() != 3 || thread_value() != 3 || inits != 3) return 2;
  return a[1][1] == 0 && read({7, 8}) == 15 ? 0 : 3;
}
