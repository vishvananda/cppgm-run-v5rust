struct X {
  int f() & { return 1; }
  int f() && { return 2; }
};

int main() {
  X x;
  return x.f() - 1;
}
