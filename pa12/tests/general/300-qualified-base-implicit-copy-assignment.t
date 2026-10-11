struct Base { int value; };

struct Derived : Base {
  Derived & operator=(const Derived & other) {
    Base::operator=(other);
    return *this;
  }
};

int main() {
  Derived a;
  a.value = 1;
  Derived b;
  b.value = 2;
  a = b;
  return a.value == 2 ? 0 : 1;
}
