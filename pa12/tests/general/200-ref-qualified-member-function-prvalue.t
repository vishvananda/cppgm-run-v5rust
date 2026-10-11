struct X {
  int f() & { return 1; }
  int f() && { return 2; }
};

int main() {
  return X().f() - 2;
}
