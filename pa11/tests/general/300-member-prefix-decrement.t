struct X {
  int n;
  X& operator--() { --n; return *this; }
};

int main() {
  X x;
  x.n = 4;
  --x;
  return x.n - 3;
}
