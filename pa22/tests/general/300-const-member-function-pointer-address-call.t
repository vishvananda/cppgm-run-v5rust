struct S
{
  int value;

  int get() const
  {
    return value;
  }

  int add(int x) const
  {
    return value + x;
  }
};

int call0(int (S::*p)() const, const S & s)
{
  return (s.*p)();
}

int call1(int (S::*p)(int) const, const S & s)
{
  return (s.*p)(5);
}

int main()
{
  S s;
  s.value = 7;

  int (S::*p0)() const = &S::get;
  int (S::*p1)(int) const = &S::add;

  return call0(&S::get, s) + call1(&S::add, s) + (s.*p0)() + (s.*p1)(2) - 35;
}
