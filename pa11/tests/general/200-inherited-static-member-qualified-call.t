// HHC-107
struct B {
  static int f(int x) {
    return x;
  }
};

struct D : B {};

int g() {
  return D::f(1);
}

int main() {
  return 0;
}
