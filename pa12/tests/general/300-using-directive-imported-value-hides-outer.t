namespace A {
  const int i = 1;

  namespace B {
    namespace C {
      const int i = 2;
    }

    using namespace C;

    int f() {
      return i;
    }
  }
}

int main() {
  return A::B::f() - 2;
}
