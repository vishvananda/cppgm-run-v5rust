struct Iter {
  int n;
  Iter& operator--() { --n; return *this; }
  int operator*() const { return n; }
};

struct Wrap {
  Iter it;
};

int main() {
  Wrap w;
  w.it.n = 4;
  return *--w.it - 3;
}
