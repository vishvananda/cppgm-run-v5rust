struct S {
  int x;
  S() : x{} {}
};

int main() {
  int x{};
  S s;
  return x + s.x;
}
