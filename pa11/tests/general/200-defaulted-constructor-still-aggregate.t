struct X {
  X() = default;
  int m;
};

int main() {
  X x = {1};
  return x.m != 1;
}
