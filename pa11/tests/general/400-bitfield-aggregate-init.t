struct X { int m : 3; };

int observe(const int& value, X& x) {
  x.m = -1;
  return value;
}

int main() {
  X x = {3};
  const int& snapshot = x.m;
  x.m = -1;
  if (snapshot != 3 || x.m != -1) return 1;
  x.m = 3;
  return observe(x.m, x) != 3 || x.m != -1;
}
