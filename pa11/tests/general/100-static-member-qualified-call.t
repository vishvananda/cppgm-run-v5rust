// HHC-106
struct X {
  static int f(int x) {
    return x;
  }
};

int g() {
  return X::f(1);
}

int main() {
  return 0;
}
