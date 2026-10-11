namespace A {
  const int i = 1;

  namespace B {
    namespace C {
      const int i = 2;
    }

    using namespace C;
  }

  namespace D {
    using namespace B;
    using namespace C;

    int f() {
      return i;
    }
  }
}

int main() {
  return A::D::f();
}
