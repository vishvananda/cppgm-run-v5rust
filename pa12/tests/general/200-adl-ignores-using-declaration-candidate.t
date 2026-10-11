// Expected: ADL does not add functions made visible only by a using-declaration
// when ordinary lookup already finds the nested namespace function.

struct S {};

namespace A
{
struct X
{
  int value;
};

X f(const S&);

namespace B
{
struct Y
{
  int value;
};

Y f(const S&)
{
  Y y;
  y.value = 0;
  return y;
}

Y g(const S& s)
{
  return f(s);
}
}
}

using A::f;

int main()
{
  S s;
  A::B::Y y = A::B::g(s);
  return y.value;
}
