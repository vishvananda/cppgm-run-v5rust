struct X {
  int n;
  auto&& base() & { return *this; }
};

int main() {
  X x;
  x.base().n = 3;
  return x.n - 3;
}
