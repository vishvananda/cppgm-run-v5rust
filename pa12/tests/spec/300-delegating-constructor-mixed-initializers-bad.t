// A delegating mem-initializer must be the only mem-initializer.

struct A
{
  int value;

  A() : A(1), value(2) {}
  A(int input) : value(input) {}
};

int main()
{
  A a;
  return 0;
}
