struct X {
  int n;
  X& operator--() { --n; return *this; }
  int operator*() const { return n; }
};

int main() {
  X x;
  x.n = 4;
  return *--x - 3;
}
