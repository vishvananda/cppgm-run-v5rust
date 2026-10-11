struct X {
  auto f() { return 1; }
};

int main() {
  X x;
  return x.f() - 1;
}
