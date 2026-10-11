struct X {
  int value;

  X() : value(0) {}

  int add(int y)
  {
    return value + y;
  }
};

typedef int (X::*pmf)(int);

int main()
{
  X x;
  x.value = 3;
  pmf p = &X::add;
  return (x.*p)(4) - 7;
}
