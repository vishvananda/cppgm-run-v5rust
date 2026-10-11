struct S {
  S() noexcept;
};

int main() {
  S* p = new S();
  return p == 0;
}
