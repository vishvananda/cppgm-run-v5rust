int *f(int& a, int& b, bool c) {
  return &(c ? a : b);
}

int main() {
  int a = 1;
  int b = 2;
  return *f(a, b, true) - 1;
}
