struct Base {
  virtual int f(int) = 0;
  virtual int f(char) = 0;
};

struct Derived : Base {
  virtual int f(int) { return 1; }
  virtual int f(char) { return 2; }
};

int dispatch(Base& b) {
  return b.f('x');
}

int main() {
  Derived d;
  Base& b = d;
  return dispatch(b) - 2;
}
