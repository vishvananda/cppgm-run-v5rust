namespace n {
struct Q {
  int value;

  Q(int x) : value(x) {}
};

int f(int x, Q & q)
{
  return 100 + x + q.value;
}
}

struct A {
  int base;

  A() : base(3) {}

  int f(int x, n::Q & q) const
  {
    return base + x + q.value;
  }

  int g() const
  {
    n::Q q(4);
    return f(5, q);
  }
};

int main()
{
  A a;
  return a.g() - 12;
}
