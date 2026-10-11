struct X {
  bool test() { return bool(true); }
};

int main() {
  X x;
  return x.test() ? 0 : 1;
}
