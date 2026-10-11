struct Left {
  virtual int f() { return 1; }
};

struct Right {
  virtual int g() const { return 2; }
};

struct Derived : Left, Right {
  int f() override { return 41; }
  int g() const override { return 17; }
};

int main() {
  Derived d;
  return 0;
}
