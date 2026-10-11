int called;

struct virtual_base
{
  virtual void function() { called = 1; }
};

struct empty_base {};

struct derived : empty_base, virtual virtual_base {};

int main()
{
  derived value;
  value.function();
  return called == 1 ? 0 : 1;
}
