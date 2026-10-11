// VALIDATION: compile-pass

struct Base
{
  typedef int value_type;
};

struct Derived : Base
{
  int run();
};

int Derived::run()
{
  value_type *p = 0;
  return p == 0 ? 0 : 1;
}

int main()
{
  Derived d;
  return d.run();
}
