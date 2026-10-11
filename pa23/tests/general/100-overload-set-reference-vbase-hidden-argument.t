struct V
{
  int marker;
  V() : marker(0) {}
};

struct A : virtual V
{
  A() {}
};

struct D : A
{
  D() {}
};

int read(A & a)
{
  return a.marker;
}

int read(int)
{
  return 0;
}

void poison(int, V * p)
{
  p->marker = 13;
}

int main()
{
  D d;
  V other;
  d.marker = 7;
  poison(0, &other);
  return read(d) == 7 ? 0 : 1;
}
