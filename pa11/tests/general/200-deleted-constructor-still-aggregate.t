struct X {
  X() = delete;
  int m;
};

int main() {
  X x = {1};
  return x.m != 1;
}
