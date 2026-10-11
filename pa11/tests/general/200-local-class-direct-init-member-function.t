struct M {
  int v;
  M() : v(5) {}
};

struct Lock {
  int v;
  Lock(M& m) : v(m.v) {}
};

struct AssocState {
  M m;

  int attach() {
    Lock lk(m);
    return lk.v - 5;
  }
};

int main() {
  AssocState s;
  return s.attach();
}
