struct Base
{
  int f(int) { return 1; }
};

struct Derived : Base
{
  using Base::f;

  int f(double) { return 2; }
};

int main()
{
  Derived d;
  return d.f(1) == 1 && d.f(1.5) == 2 ? 0 : 1;
}
