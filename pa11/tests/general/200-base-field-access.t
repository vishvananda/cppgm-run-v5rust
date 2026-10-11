struct YA {
  int a;
  YA() : a(1) {}
};

struct YB : public YA {
  int b;
  YB() : b(2) {}
};

int main() {
  YB x;
  return x.a + x.b;
}
