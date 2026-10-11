// VALIDATION: emit-lowir
// N3485 focus: 10.4 [class.abstract]

struct X {
  virtual void f() = 0;
};

int main() {
  return 0;
}
