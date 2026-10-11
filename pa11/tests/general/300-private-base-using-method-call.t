struct Base {
  int f() const {
    return 7;
  }
};

class Derived : private Base {
public:
  using Base::f;
};

int main() {
  Derived d;
  return d.f() - 7;
}
