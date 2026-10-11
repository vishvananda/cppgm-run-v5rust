// VALIDATION: compile-fail

struct A {};

struct B {
  B(A);
  int f();
};

int main()
{
  B b(A());
  return b.f();
}
