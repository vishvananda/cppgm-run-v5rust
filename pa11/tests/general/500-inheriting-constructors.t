struct Base
{
  int value;

  Base(int v) : value(v) {}
};

struct Derived : Base
{
  using Base::Base;
};

int main()
{
  Derived d(4);
  return d.value == 4 ? 0 : 1;
}
