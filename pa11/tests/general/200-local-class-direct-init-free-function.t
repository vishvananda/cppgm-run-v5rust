struct M {
  int v;
  M() : v(7) {}
};

struct Lock {
  int v;
  Lock(M& m) : v(m.v) {}
};

int main() {
  M m;
  Lock lk(m);
  return lk.v - 7;
}
