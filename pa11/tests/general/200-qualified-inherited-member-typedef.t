// HHC-100
namespace N {
struct Base {
  typedef const char* T;
};

struct Derived : Base {};
typedef Derived Impl;
}

class X {
  typedef N::Impl I;
  I::T value;
};

int main() {
  return 0;
}
