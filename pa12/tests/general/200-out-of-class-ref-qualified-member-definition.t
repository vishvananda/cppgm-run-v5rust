struct X {
  int f() &;
  int f() &&;
};

int X::f() & { return 1; }
int X::f() && { return 2; }

int main() {
  X x;
  return x.f() + X().f() - 3;
}
