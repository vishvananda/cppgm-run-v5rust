struct Pad {
  int pad;
};

struct Base {
  explicit operator bool() const { return true; }
};

struct Derived : Pad, Base {
};

int f(Derived d) {
  while (d) {
    return 1;
  }
  return 0;
}
