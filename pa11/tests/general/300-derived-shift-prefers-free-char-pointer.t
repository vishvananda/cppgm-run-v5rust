struct Base {
  int tag;

  Base() : tag(0) {}

  Base& operator<<(bool) {
    tag = 1;
    return *this;
  }
};

Base& operator<<(Base& b, const char*) {
  b.tag = 2;
  return b;
}

struct Derived : Base {};

int main() {
  Derived d;
  d << "x";
  return d.tag == 2 ? 0 : 1;
}
