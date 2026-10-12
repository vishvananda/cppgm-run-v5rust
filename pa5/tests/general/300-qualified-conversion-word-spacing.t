struct X { using T=int; };
struct A {
  operator __attribute__((vector_size(16))) int();
  operator typename ::X::T();
};
A::operator __attribute__((vector_size(16))) int() {}
A::operator typename ::X::T() {}
