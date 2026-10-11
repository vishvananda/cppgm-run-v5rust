struct S {
  S();
  S(const S&) = delete;
  S(S&&);
};

S::S() {}

S make() { return S(); }

int main() {
  S s = make();
  return 0;
}
