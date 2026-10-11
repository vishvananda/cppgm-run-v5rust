struct Base {
  const Base& operator=(const Base&) { return *this; }
};

struct Derived : Base {
  Derived& operator=(const Derived& rhs) {
    Base::operator=(rhs);
    return *this;
  }
};

int main() {
  Derived a;
  Derived b;
  a = b;
  return 0;
}
