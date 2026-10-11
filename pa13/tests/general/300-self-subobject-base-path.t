// HHC-118
struct B {
  virtual int f() const { return 1; }
};

int main() {
  B b;
  return b.f() - 1;
}
