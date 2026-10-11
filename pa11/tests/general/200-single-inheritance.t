struct YA {
  int a;
  YA() : a(1) {}
};

struct YB : public YA {
  int b;
  YB() : b(2) {}
  int sum() { return a + b; }
};

int main() {
  YB x;
  return x.sum();
}
