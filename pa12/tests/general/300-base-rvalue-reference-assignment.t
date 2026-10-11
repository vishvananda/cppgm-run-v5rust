struct Base {
  Base& operator=(Base&&) { return *this; }
};

struct Derived : Base {
  Derived& operator=(Derived&& rhs) {
    Base::operator=(static_cast<Derived&&>(rhs));
    return *this;
  }
};

int main() {
  Derived a;
  Derived b;
  a = static_cast<Derived&&>(b);
  return 0;
}
