struct S {
  mutable int x;
  int *ptr() const { return &x; }
};

int main() {
  const S s = {};
  *s.ptr() = 7;
  return *s.ptr() == 7 ? 0 : 1;
}
