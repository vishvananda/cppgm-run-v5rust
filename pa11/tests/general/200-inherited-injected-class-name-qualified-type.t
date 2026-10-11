// VALIDATION: compile-pass
// N3485 focus: 9 [class] injected-class-name lookup through a base scope

struct Base
{
  int value;
};

struct Derived : Base
{
};

Derived::Base *as_base(Derived *d)
{
  return d;
}

int main()
{
  Derived d;
  d.value = 7;
  Derived::Base *p = as_base(&d);
  return p->value == 7 ? 0 : 1;
}
