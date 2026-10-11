int g(int x) { return x + 1; }

int call(int (*fp)(int)) { return fp(4); }

int invoke(int (&rf)(int), int x) { return rf(x); }

int main() {
  return call(g) + invoke(g, 1);
}
