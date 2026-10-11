struct S {
  typedef int *P;
  typedef const int *CP;

  int value;
  P p;

  S() : value(7), p(&value) {}

  int f(P)
  {
    return 1;
  }

  int f(CP) const
  {
    return *p;
  }

  int g() const
  {
    return f(p);
  }
};

int main()
{
  S s;
  return s.g() - 7;
}
