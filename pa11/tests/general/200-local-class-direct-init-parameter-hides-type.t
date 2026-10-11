struct M {
  int v;
  M() : v(9) {}
};

struct Lock {
  int v;
  Lock(M& m) : v(m.v) {}
};

int run(M& M) {
  Lock lk(M);
  return lk.v - 9;
}

int main() {
  M m;
  return run(m);
}
